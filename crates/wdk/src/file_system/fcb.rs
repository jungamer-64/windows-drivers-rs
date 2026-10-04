// Copyright (c) Microsoft Corporation
// License: MIT OR Apache-2.0

use core::{marker::PhantomData, pin::Pin, ptr::NonNull};

use wdk_sys::{FAST_MUTEX, FSRTL_ADVANCED_FCB_HEADER, FSRTL_COMMON_FCB_HEADER, PVOID, ntddk};

/// A borrowed advanced FCB header with initialized stream-context support.
///
/// The header, fast mutex, and optional per-file context slot remain owned by
/// the driver. This value ties their Rust borrows together; it does not own
/// publication to file objects, cache maps, resources, or an auto-expand lock.
/// Dropping or forgetting the view does not tear down the header. The driver
/// must still finish external uses and tear down stream contexts before freeing
/// or moving storage. Per-file context teardown belongs to the owning file.
#[must_use = "the driver must retain header storage and explicitly tear down stream contexts"]
pub struct FcbHeader<'a> {
    header: NonNull<FSRTL_ADVANCED_FCB_HEADER>,
    lifetime: PhantomData<(
        &'a mut FSRTL_ADVANCED_FCB_HEADER,
        &'a mut FAST_MUTEX,
        &'a mut PVOID,
    )>,
}

impl<'a> FcbHeader<'a> {
    /// Initializes stream support and optional file-context support in place.
    /// The auto-expand push lock is left absent; its allocation and lifetime
    /// belong to the driver when supplied through the raw WDK boundary.
    ///
    /// # Safety
    ///
    /// IRQL must be at most `APC_LEVEL`. The header must not already have live
    /// contexts, cache-map users, or oplock state. Its base fields must contain
    /// valid driver-supplied values. The fast mutex must be initialized and in
    /// nonpaged pool. The optional slot must belong to the owning per-file
    /// structure. All supplied storage must remain at these addresses until
    /// external references end and teardown finishes, even if the Rust view is
    /// dropped or forgotten. Paging-file headers must be in nonpaged pool.
    /// Kernel and driver access must obey the WDK synchronization protocol.
    pub unsafe fn initialize(
        header: Pin<&'a mut FSRTL_ADVANCED_FCB_HEADER>,
        fast_mutex: Pin<&'a mut FAST_MUTEX>,
        file_context_slot: Option<Pin<&'a mut PVOID>>,
    ) -> Self {
        let header = NonNull::from(header.get_mut());
        let fast_mutex = core::ptr::from_mut(fast_mutex.get_mut());
        let file_context_slot = file_context_slot.map_or(core::ptr::null_mut(), |slot| {
            core::ptr::from_mut(slot.get_mut())
        });
        // SAFETY: The caller supplies stationary, initialized storage with
        // the lifetime and pool guarantees required by the WDK macro.
        unsafe {
            ntddk::FsRtlSetupAdvancedHeaderEx2(
                header.as_ptr(),
                fast_mutex,
                file_context_slot,
                core::ptr::null_mut(),
            );
        };
        Self {
            header,
            lifetime: PhantomData,
        }
    }

    /// Borrows the header's address for WDK interoperation. External users must
    /// stop before teardown; this pointer does not transfer storage ownership.
    #[must_use]
    pub const fn as_ptr(&self) -> *mut FSRTL_ADVANCED_FCB_HEADER {
        self.header.as_ptr()
    }

    /// Borrows the embedded common header for WDK interoperation.
    ///
    /// The common header is the ABI-defined prefix; the C export translation
    /// unit checks its offset and extent against the authoritative WDK header.
    /// Access to sizes and resources requires the WDK synchronization protocol.
    #[must_use]
    pub const fn as_common_ptr(&self) -> *mut FSRTL_COMMON_FCB_HEADER {
        self.header.as_ptr().cast()
    }

    /// Tears down stream contexts after external use has stopped. This consumes
    /// the view but does not free storage, delete executive resources, or free
    /// a driver-supplied auto-expand push lock.
    ///
    /// # Safety
    ///
    /// IRQL must be at most `APC_LEVEL`. All file-object, cache-map, oplock,
    /// and concurrent context users must be quiescent. Registered free
    /// callbacks must remain valid and follow WDK reentrancy and locking
    /// requirements. No outstanding Rust or foreign reference may use the
    /// header afterward.
    pub unsafe fn teardown(self) {
        // SAFETY: Construction established stream support; the caller proves
        // external quiescence and the required teardown execution context.
        unsafe { ntddk::FsRtlTeardownPerStreamContexts(self.header.as_ptr()) };
    }
}
