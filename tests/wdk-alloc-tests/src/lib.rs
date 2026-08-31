// Copyright (c) Microsoft Corporation
// License: MIT OR Apache-2.0

//! Integration tests for [`wdk_alloc`] that require a WDK configuration in the
//! Cargo build graph.

#[cfg(test)]
mod tests {
    use core::{
        alloc::{GlobalAlloc, Layout},
        ffi::c_void,
        ptr,
        sync::atomic::{AtomicBool, AtomicUsize, Ordering},
    };
    use std::sync::Mutex;

    use wdk_alloc::WdkAllocator;
    use wdk_sys::{PAGE_SIZE, POOL_FLAG_NON_PAGED, POOL_FLAGS, PVOID, SIZE_T, ULONG};

    const MOCK_POOL_SIZE: usize = 16 * 1024;
    #[cfg(target_pointer_width = "32")]
    const MIN_POOL_ALIGNMENT: usize = 8;
    #[cfg(target_pointer_width = "64")]
    const MIN_POOL_ALIGNMENT: usize = 16;
    const RUST_TAG: ULONG = u32::from_ne_bytes(*b"rust");

    #[repr(C, align(4096))]
    struct MockPool([u8; MOCK_POOL_SIZE]);

    static mut MOCK_POOL: MockPool = MockPool([0; MOCK_POOL_SIZE]);
    static TEST_LOCK: Mutex<()> = Mutex::new(());
    static FAIL_NEXT_ALLOCATION: AtomicBool = AtomicBool::new(false);
    static LAST_ALLOCATION_SIZE: AtomicUsize = AtomicUsize::new(0);
    static LAST_POOL_POINTER: AtomicUsize = AtomicUsize::new(0);
    static LAST_FREED_POINTER: AtomicUsize = AtomicUsize::new(0);
    static LAST_POOL_FLAGS: AtomicUsize = AtomicUsize::new(0);
    static LAST_POOL_TAG: AtomicUsize = AtomicUsize::new(0);

    // SAFETY: This test binary provides the only definition of the WDK
    // allocator symbol, and the returned storage is a page-aligned static
    // buffer that lives for the duration of every allocation exercised
    // below.
    #[unsafe(no_mangle)]
    #[allow(non_snake_case, reason = "the symbol name is fixed by the WDK ABI")]
    unsafe extern "C" fn ExAllocatePool2(
        pool_flags: POOL_FLAGS,
        number_of_bytes: SIZE_T,
        tag: ULONG,
    ) -> PVOID {
        let number_of_bytes = usize::try_from(number_of_bytes).unwrap_or(usize::MAX);
        LAST_ALLOCATION_SIZE.store(number_of_bytes, Ordering::SeqCst);
        LAST_POOL_FLAGS.store(
            usize::try_from(pool_flags).unwrap_or(usize::MAX),
            Ordering::SeqCst,
        );
        LAST_POOL_TAG.store(usize::try_from(tag).unwrap_or(usize::MAX), Ordering::SeqCst);

        if FAIL_NEXT_ALLOCATION.swap(false, Ordering::SeqCst) {
            return ptr::null_mut();
        }

        let offset = if number_of_bytes >= PAGE_SIZE as usize {
            0
        } else {
            MIN_POOL_ALIGNMENT
        };
        if number_of_bytes > MOCK_POOL_SIZE - offset {
            return ptr::null_mut();
        }

        let pool_base = ptr::addr_of_mut!(MOCK_POOL).cast::<u8>();
        // SAFETY: The bounds check above proves `offset` is inside `MOCK_POOL`.
        let pool_pointer = unsafe { pool_base.add(offset) };
        LAST_POOL_POINTER.store(pool_pointer.addr(), Ordering::SeqCst);
        pool_pointer.cast::<c_void>()
    }

    // SAFETY: This test binary provides the only definition of the WDK free
    // symbol. The mock owns static storage, so freeing records the pointer
    // without ending the storage lifetime.
    #[unsafe(no_mangle)]
    #[allow(non_snake_case, reason = "the symbol name is fixed by the WDK ABI")]
    unsafe extern "C" fn ExFreePool(pool_pointer: PVOID) {
        LAST_FREED_POINTER.store(pool_pointer.addr(), Ordering::SeqCst);
    }

    fn run_serially(test: impl FnOnce()) {
        let _guard = TEST_LOCK.lock().unwrap_or_else(|error| error.into_inner());
        FAIL_NEXT_ALLOCATION.store(false, Ordering::SeqCst);
        LAST_ALLOCATION_SIZE.store(0, Ordering::SeqCst);
        LAST_POOL_POINTER.store(0, Ordering::SeqCst);
        LAST_FREED_POINTER.store(0, Ordering::SeqCst);
        LAST_POOL_FLAGS.store(0, Ordering::SeqCst);
        LAST_POOL_TAG.store(0, Ordering::SeqCst);
        test();
    }

    fn allocate(layout: Layout) -> *mut u8 {
        // SAFETY: Every caller supplies a non-zero layout and either checks a
        // null result or pairs the allocation with `deallocate` below.
        unsafe { WdkAllocator.alloc(layout) }
    }

    fn deallocate(pointer: *mut u8, layout: Layout) {
        // SAFETY: Every caller passes a live pointer returned by `allocate`
        // with the same layout, and calls this helper at most once for
        // that pointer.
        unsafe {
            WdkAllocator.dealloc(pointer, layout);
        }
    }

    fn assert_allocation_covers(pointer: *mut u8, layout: Layout) {
        let pool_pointer = LAST_POOL_POINTER.load(Ordering::SeqCst);
        let allocation_end = pool_pointer
            .checked_add(LAST_ALLOCATION_SIZE.load(Ordering::SeqCst))
            .unwrap();
        let layout_end = pointer.addr().checked_add(layout.size()).unwrap();
        assert!(layout_end <= allocation_end);
    }

    #[test]
    fn uses_pool_alignment_directly() {
        run_serially(|| {
            let layout = Layout::from_size_align(64, MIN_POOL_ALIGNMENT).unwrap();
            let pointer = allocate(layout);

            assert!(!pointer.is_null());
            assert_eq!(pointer.addr(), LAST_POOL_POINTER.load(Ordering::SeqCst));
            assert_allocation_covers(pointer, layout);
            assert_eq!(
                LAST_POOL_FLAGS.load(Ordering::SeqCst),
                usize::try_from(POOL_FLAG_NON_PAGED).unwrap()
            );
            assert_eq!(
                LAST_POOL_TAG.load(Ordering::SeqCst),
                usize::try_from(RUST_TAG).unwrap()
            );

            deallocate(pointer, layout);
            assert_eq!(LAST_FREED_POINTER.load(Ordering::SeqCst), pointer.addr());
        });
    }

    #[test]
    fn realigns_sub_page_allocations_and_frees_the_pool_pointer() {
        run_serially(|| {
            let layout = Layout::from_size_align(37, 256).unwrap();
            let pointer = allocate(layout);
            let pool_pointer = LAST_POOL_POINTER.load(Ordering::SeqCst);

            assert!(!pointer.is_null());
            assert_ne!(pointer.addr(), pool_pointer);
            assert_eq!(pointer.addr() % layout.align(), 0);
            assert_allocation_covers(pointer, layout);

            // SAFETY: `pointer` addresses the first byte of the live
            // allocation.
            unsafe {
                pointer.write(0xA5);
            }
            // SAFETY: The byte initialized immediately above remains allocated.
            assert_eq!(unsafe { pointer.read() }, 0xA5);

            deallocate(pointer, layout);
            assert_eq!(LAST_FREED_POINTER.load(Ordering::SeqCst), pool_pointer);
        });
    }

    #[test]
    fn uses_page_alignment_directly() {
        run_serially(|| {
            let layout = Layout::from_size_align(PAGE_SIZE as usize, PAGE_SIZE as usize).unwrap();
            let pointer = allocate(layout);

            assert!(!pointer.is_null());
            assert_eq!(pointer.addr(), LAST_POOL_POINTER.load(Ordering::SeqCst));
            assert_eq!(pointer.addr() % layout.align(), 0);
            assert_allocation_covers(pointer, layout);

            deallocate(pointer, layout);
            assert_eq!(LAST_FREED_POINTER.load(Ordering::SeqCst), pointer.addr());
        });
    }

    #[test]
    fn propagates_pool_allocation_failure() {
        run_serially(|| {
            let layout = Layout::from_size_align(37, 256).unwrap();
            FAIL_NEXT_ALLOCATION.store(true, Ordering::SeqCst);

            assert!(allocate(layout).is_null());
            assert_eq!(LAST_FREED_POINTER.load(Ordering::SeqCst), 0);
        });
    }
}
