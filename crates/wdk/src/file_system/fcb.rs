// Copyright (c) Microsoft Corporation
// License: MIT OR Apache-2.0

use core::{marker::PhantomData, pin::Pin, ptr::NonNull};

use wdk_sys::{FAST_MUTEX, FSRTL_ADVANCED_FCB_HEADER, FSRTL_COMMON_FCB_HEADER, PVOID};

/// A borrowed advanced FCB header with initialized stream-context support.
///
/// The header, fast mutex, and optional per-file context slot remain owned by
/// the driver. This value ties their Rust borrows together; it does not own
/// publication to file objects, cache maps or resources. Initialization owns
/// the native auto-expand lock. Dropping or forgetting the view does not tear
/// down the header. The driver must still finish external uses and tear down
/// stream contexts before freeing or moving storage. Per-file context teardown
/// belongs to the owning file.
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
    /// Initializes stationary header/mutex storage and owns its `AePushLock`.
    ///
    /// # Errors
    /// Returns the native allocation status without publishing an initialized
    /// header.
    ///
    /// # Safety
    /// Run at `APC_LEVEL` or below. Header base fields must be initialized and
    /// have no live contexts, mappings or oplocks. Header, mutex and optional
    /// per-file slot must remain stationary until external users stop and
    /// teardown finishes, even if this view is dropped or forgotten. Mutex and
    /// paging-file headers must be nonpaged. Obey WDK synchronization rules.
    pub unsafe fn initialize(
        header: Pin<&'a mut FSRTL_ADVANCED_FCB_HEADER>,
        fast_mutex: Pin<&'a mut FAST_MUTEX>,
        file_context_slot: Option<Pin<&'a mut PVOID>>,
        pool_tag: wdk_sys::ULONG,
    ) -> Result<Self, wdk_sys::NTSTATUS> {
        let header = NonNull::from(header.get_mut());
        let mutex = core::ptr::from_mut(fast_mutex.get_mut());
        let slot = file_context_slot.map_or(core::ptr::null_mut(), |slot| {
            core::ptr::from_mut(slot.get_mut())
        });
        // SAFETY: The caller supplies stationary storage with the WDK lifetime,
        // initialization and pool requirements; C owns allocation rollback.
        let status = unsafe {
            wdk_sys::file_system::initialize_fcb_header(header.as_ptr(), mutex, slot, pool_tag)
        };
        if status < 0 {
            return Err(status);
        }
        Ok(Self {
            header,
            lifetime: PhantomData,
        })
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
    /// the view but frees the owned auto-expand push lock but does not free
    /// storage or delete executive resources.
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
        unsafe { wdk_sys::file_system::teardown_fcb_header(self.header.as_ptr()) };
    }
}
