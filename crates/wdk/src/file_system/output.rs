// License: MIT OR Apache-2.0

//! Locked output pages captured before leaving the requestor process.

use core::ptr::NonNull;

use wdk_sys::{KPROCESSOR_MODE, MDL, NTSTATUS, PVOID, STATUS_INVALID_PARAMETER, file_system};

/// A nonempty writable requestor range with resident pages and an NX system
/// mapping. No Rust reference to requestor memory is formed. Drop unlocks and
/// frees its MDL at DISPATCH_LEVEL or below. This owner may move between
/// threads after capture; shared access is not provided.
#[derive(Debug)]
pub struct LockedRequestorOutput {
    mdl: NonNull<MDL>,
    mapping: NonNull<core::ffi::c_void>,
    capacity: u32,
}

// SAFETY: Locked pages and their system mapping survive the requestor context.
// Sole ownership serializes writes and teardown; no thread-affine operation is
// retained.
unsafe impl Send for LockedRequestorOutput {}

impl LockedRequestorOutput {
    /// Captures exactly the supplied range. No owner is returned on probe, MDL
    /// allocation or system-mapping failure.
    ///
    /// # Safety
    /// Run in the original requestor process at APC_LEVEL or below. The range
    /// must not alias live Rust references or concurrently accessed driver
    /// data. Grant exclusive kernel write access until this owner is consumed
    /// or dropped. All subsequent uses and destruction must run at their
    /// documented IRQL.
    ///
    /// # Errors
    /// Returns the native buffer/probe or insufficient-resources status.
    pub unsafe fn capture(
        buffer: PVOID,
        capacity: u32,
        mode: KPROCESSOR_MODE,
    ) -> Result<Self, NTSTATUS> {
        let mut mdl = core::ptr::null_mut();
        let mut mapping = core::ptr::null_mut();
        // SAFETY: The caller supplies original-process capture authority and
        // exclusive access; native code validates mode/range and catches probe
        // faults.
        let status = unsafe {
            file_system::capture_requestor_output(buffer, capacity, mode, &mut mdl, &mut mapping)
        };
        if status < 0 {
            return Err(status);
        }
        Ok(Self {
            // SAFETY: Native success publishes one initialized non-null MDL.
            mdl: unsafe { NonNull::new_unchecked(mdl) },
            // SAFETY: Native success establishes a non-null NX system mapping.
            mapping: unsafe { NonNull::new_unchecked(mapping) },
            capacity,
        })
    }

    /// Returns the exact captured byte capacity.
    #[must_use]
    pub const fn capacity(&self) -> u32 {
        self.capacity
    }

    /// Publishes owned initialized bytes, then consumes the mapping on every
    /// result.
    ///
    /// # Safety
    /// Run at PASSIVE_LEVEL with a source disjoint from the captured output.
    ///
    /// # Errors
    /// Returns invalid-parameter without copying if the source exceeds
    /// capacity.
    pub unsafe fn copy_from(self, source: &[u8]) -> Result<(), NTSTATUS> {
        let length = u32::try_from(source.len()).map_err(|_| STATUS_INVALID_PARAMETER)?;
        if length > self.capacity {
            return Err(STATUS_INVALID_PARAMETER);
        }
        // SAFETY: Capture owns the writable mapping, the range is bounded, and
        // the caller supplies disjoint initialized bytes at PASSIVE_LEVEL.
        unsafe {
            file_system::copy_requestor_output(
                self.mapping.as_ptr(),
                source.as_ptr().cast(),
                length,
            )
        };
        Ok(())
    }
}

impl Drop for LockedRequestorOutput {
    fn drop(&mut self) {
        // SAFETY: This unique MDL has no remaining mapping users. Capture's
        // contract requires destruction at DISPATCH_LEVEL or below.
        unsafe { file_system::release_requestor_output(self.mdl.as_ptr()) };
    }
}
