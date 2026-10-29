// Copyright (c) Microsoft Corporation
// License: MIT OR Apache-2.0

use core::{fmt, marker::PhantomData, mem::ManuallyDrop, ptr::NonNull};

use wdk_sys::{ERESOURCE, PERESOURCE, file_system};

use super::{InvalidPointer, checked_pointer};

/// Borrowed access to initialized, stationary driver-owned resource storage.
/// This view does not initialize or delete storage and is thread-affine.
pub struct ResourceRef<'a> {
    resource: NonNull<ERESOURCE>,
    lifetime: PhantomData<&'a ERESOURCE>,
}

/// A resource acquisition could not grant access.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceAcquireError;
impl fmt::Display for ResourceAcquireError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("executive resource is contended")
    }
}
impl core::error::Error for ResourceAcquireError {}

/// Same-thread ownership of one resource acquisition and normal-kernel-APC
/// scope. Drop releases the resource before restoring APC delivery. The guard
/// exposes no Rust reference to kernel-mutated resource fields.
#[must_use = "retain the acquisition until its protected operation has ended"]
pub struct ResourceGuard<'a> {
    resource: NonNull<ERESOURCE>,
    lifetime: PhantomData<&'a ERESOURCE>,
}
impl Drop for ResourceGuard<'_> {
    fn drop(&mut self) {
        // SAFETY: Construction acquired this live resource on the current
        // thread; the non-Send/non-Sync guard retains the paired release
        // responsibility.
        unsafe { file_system::release_resource(self.resource.as_ptr()) };
    }
}
impl ResourceGuard<'_> {
    /// Transfers the resource/APC scope to a native acquire/release protocol.
    /// The native owner must retain storage and release once on this thread
    /// using `wdk_sys::file_system::release_resource`. Forgetting the scope
    /// retains the acquisition; this does not transfer storage deletion rights.
    #[must_use = "the native owner must retain and release this acquisition"]
    pub fn into_raw(self) -> PERESOURCE {
        ManuallyDrop::new(self).resource.as_ptr()
    }
}
impl<'a> ResourceRef<'a> {
    /// Borrows an initialized resource without creating a Rust reference to it.
    ///
    /// # Errors
    /// Rejects null or misaligned addresses.
    ///
    /// # Safety
    /// Storage must be initialized, stationary, nonpaged and live throughout
    /// 'a, including acquisitions transferred to native callbacks. No path
    /// may delete or reinitialize it while in use. Concurrent access must obey
    /// the WDK resource protocol without aliasing Rust references to mutations.
    pub unsafe fn from_raw(resource: PERESOURCE) -> Result<Self, InvalidPointer> {
        Ok(Self {
            resource: checked_pointer(resource)?,
            lifetime: PhantomData,
        })
    }

    /// Acquires shared access, waiting until it is available.
    ///
    /// # Errors
    /// Returns an error if the native acquisition does not grant access.
    ///
    /// # Safety
    /// Run at `APC_LEVEL` or below. Release on this thread at that IRQL without
    /// enabling normal kernel APCs. Follow resource ordering; do not defer work
    /// depending on this acquisition beyond its retaining owner.
    pub unsafe fn acquire_shared(&self) -> Result<ResourceGuard<'a>, ResourceAcquireError> {
        // SAFETY: The caller establishes the acquisition execution contract.
        unsafe { self.acquire(false, true) }
    }

    /// Acquires exclusive access, waiting until it is available.
    ///
    /// # Errors
    /// Returns an error if native acquisition does not grant access.
    ///
    /// # Safety
    /// The execution requirements of `Self::acquire_shared` apply. Do not
    /// upgrade a held shared acquisition to exclusive access.
    pub unsafe fn acquire_exclusive(&self) -> Result<ResourceGuard<'a>, ResourceAcquireError> {
        // SAFETY: The caller establishes the exclusive acquisition contract.
        unsafe { self.acquire(true, true) }
    }

    /// Attempts shared access without waiting. Failure retains no APC scope.
    ///
    /// # Errors
    /// Returns an error on contention.
    ///
    /// # Safety
    /// The execution requirements of `Self::acquire_shared` apply.
    pub unsafe fn try_acquire_shared(&self) -> Result<ResourceGuard<'a>, ResourceAcquireError> {
        // SAFETY: The caller establishes the acquisition execution contract.
        unsafe { self.acquire(false, false) }
    }

    /// Attempts exclusive access without waiting. Failure retains no APC scope.
    ///
    /// # Errors
    /// Returns an error on contention.
    ///
    /// # Safety
    /// The execution requirements of `Self::acquire_exclusive` apply.
    pub unsafe fn try_acquire_exclusive(&self) -> Result<ResourceGuard<'a>, ResourceAcquireError> {
        // SAFETY: The caller establishes the exclusive acquisition contract.
        unsafe { self.acquire(true, false) }
    }

    unsafe fn acquire(
        &self,
        exclusive: bool,
        wait: bool,
    ) -> Result<ResourceGuard<'a>, ResourceAcquireError> {
        let acquired = if exclusive {
            // SAFETY: This view retains storage; the caller establishes
            // thread/IRQL.
            unsafe {
                file_system::acquire_resource_exclusive(self.resource.as_ptr(), u8::from(wait))
            }
        } else {
            // SAFETY: This view retains storage; the caller establishes
            // thread/IRQL.
            unsafe { file_system::acquire_resource_shared(self.resource.as_ptr(), u8::from(wait)) }
        };
        if acquired == 0 {
            return Err(ResourceAcquireError);
        }
        Ok(ResourceGuard {
            resource: self.resource,
            lifetime: PhantomData,
        })
    }
}
