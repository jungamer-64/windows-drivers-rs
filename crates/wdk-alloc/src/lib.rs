// Copyright (c) Microsoft Corporation
// License: MIT OR Apache-2.0

//! Allocator implementation to use with `#[global_allocator]` to allow use of
//! [`core::alloc`].
//!
//! # Example
//! ```rust, no_run
//! #[cfg(all(
//!     any(driver_model__driver_type = "WDM", driver_model__driver_type = "KMDF"),
//!     not(test)
//! ))]
//! use wdk_alloc::WdkAllocator;
//!
//! #[cfg(all(
//!     any(driver_model__driver_type = "WDM", driver_model__driver_type = "KMDF"),
//!     not(test)
//! ))]
//! #[global_allocator]
//! static GLOBAL_ALLOCATOR: WdkAllocator = WdkAllocator;
//! ```

#![no_std]

#[cfg(any(driver_model__driver_type = "WDM", driver_model__driver_type = "KMDF"))]
pub use kernel_mode::*;

#[cfg(any(driver_model__driver_type = "WDM", driver_model__driver_type = "KMDF"))]
mod kernel_mode {

    use core::{
        alloc::{GlobalAlloc, Layout},
        mem::size_of,
    };

    use wdk_sys::{
        PAGE_SIZE,
        POOL_FLAG_NON_PAGED,
        PVOID,
        SIZE_T,
        ULONG,
        ntddk::{ExAllocatePool2, ExFreePool},
    };

    /// Allocator implementation to use with `#[global_allocator]` to allow use
    /// of [`core::alloc`].
    ///
    /// # Safety
    /// This allocator is only safe to use for allocations happening at `IRQL`
    /// <= `DISPATCH_LEVEL`
    pub struct WdkAllocator;

    // The value of memory tags are stored in little-endian order, so it is
    // convenient to reverse the order for readability in tooling (ie. Windbg)
    const RUST_TAG: ULONG = u32::from_ne_bytes(*b"rust");

    // `ExAllocatePool2` aligns sub-page allocations to 8 bytes on 32-bit
    // systems and 16 bytes on 64-bit systems.
    const MIN_POOL_ALIGNMENT: usize = size_of::<usize>() * 2;

    #[inline]
    fn requires_manual_alignment(layout: Layout) -> bool {
        let pool_alignment = if layout.size() >= PAGE_SIZE as usize {
            PAGE_SIZE as usize
        } else {
            MIN_POOL_ALIGNMENT
        };

        layout.align() > pool_alignment
    }

    // SAFETY: This is safe because the Wdk allocator:
    //         1. can never unwind since it can never panic
    //         2. alloc and dealloc satisfy every layout's size and alignment
    unsafe impl GlobalAlloc for WdkAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            let allocation_size = if requires_manual_alignment(layout) {
                let Some(allocation_size) = layout.size().checked_add(layout.align()) else {
                    return core::ptr::null_mut();
                };
                allocation_size
            } else {
                layout.size()
            };
            let Ok(allocation_size) = SIZE_T::try_from(allocation_size) else {
                return core::ptr::null_mut();
            };

            let ptr =
                // SAFETY: `ExAllocatePool2` is safe to call from any `IRQL` <= `DISPATCH_LEVEL` since its allocating from `POOL_FLAG_NON_PAGED`
                unsafe {
                    ExAllocatePool2(
                        POOL_FLAG_NON_PAGED,
                        allocation_size,
                        RUST_TAG,
                    )
                };
            if ptr.is_null() {
                return core::ptr::null_mut();
            }

            if !requires_manual_alignment(layout) {
                return ptr.cast();
            }

            let alignment_mask = !(layout.align() - 1);
            let Some(aligned_address) = ptr
                .addr()
                .checked_add(layout.align())
                .map(|address| address & alignment_mask)
            else {
                // SAFETY: `ptr` was returned by `ExAllocatePool2` above and has
                // not been freed yet.
                unsafe {
                    ExFreePool(ptr);
                }
                return core::ptr::null_mut();
            };
            let aligned_ptr = ptr.cast::<u8>().with_addr(aligned_address);

            // SAFETY: The pool pointer is aligned to at least
            // `MIN_POOL_ALIGNMENT`, while the requested alignment is a larger
            // power of two. Therefore the strictly next aligned address leaves
            // at least one pointer-sized slot inside the allocation.
            let original_pointer_slot = unsafe { aligned_ptr.cast::<PVOID>().sub(1) };
            // SAFETY: `original_pointer_slot` points to the pointer-sized slot
            // immediately preceding `aligned_ptr` inside this allocation.
            unsafe {
                original_pointer_slot.write(ptr);
            }

            aligned_ptr
        }

        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            let pool_ptr = if requires_manual_alignment(layout) {
                // SAFETY: `alloc` stored the original pool pointer in the
                // pointer-sized slot immediately preceding every manually
                // aligned pointer it returns.
                let original_pointer_slot = unsafe { ptr.cast::<PVOID>().sub(1) };
                // SAFETY: The caller must pass the same layout used for
                // `alloc`, so `original_pointer_slot`
                // identifies the initialized slot created by
                // that allocation.
                unsafe { original_pointer_slot.read() }
            } else {
                ptr.cast()
            };

            // SAFETY: `ExFreePool` is safe to call from any `IRQL` <=
            // `DISPATCH_LEVEL` since it is freeing the original
            // pointer allocated from `POOL_FLAG_NON_PAGED` in
            // `alloc`
            unsafe {
                ExFreePool(pool_ptr);
            }
        }
    }
}
