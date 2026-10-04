// Copyright (c) Microsoft Corporation
// License: MIT OR Apache-2.0

use core::{fmt, marker::PhantomData, ptr::NonNull};

use wdk_sys::{ERESOURCE, PERESOURCE, ntddk};

use super::{CriticalRegionGuard, InvalidPointer, checked_pointer};

/// Borrowed access to an initialized driver-owned executive resource.
///
/// The driver initializes, keeps stationary, and deletes the resource after
/// all users stop. This view owns neither its storage nor its deletion. It is
/// thread-affine; another dispatch thread may acquire its own borrowed view
/// through that thread's unsafe driver boundary.
pub struct ResourceRef<'a> {
    resource: NonNull<ERESOURCE>,
    lifetime: PhantomData<&'a ERESOURCE>,
}

/// The executive resource did not grant access.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceAcquireError;

impl fmt::Display for ResourceAcquireError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("executive resource is contended")
    }
}

impl core::error::Error for ResourceAcquireError {}

#[derive(Clone, Copy)]
enum Access {
    Shared,
    Exclusive,
}

struct AcquiredResource {
    resource: NonNull<ERESOURCE>,
    _critical_region: CriticalRegionGuard,
}

impl Drop for AcquiredResource {
    fn drop(&mut self) {
        // SAFETY: This private guard exists only after successful acquisition.
        // The scoped callback contract preserves thread and IRQL; its region
        // remains active until after this release.
        unsafe { ntddk::ExReleaseResourceLite(self.resource.as_ptr()) };
    }
}

impl ResourceRef<'_> {
    /// Borrows driver-owned executive resource storage.
    ///
    /// # Errors
    ///
    /// Rejects null or misaligned resource addresses.
    ///
    /// # Safety
    ///
    /// An aligned non-null pointer must refer to a stationary, initialized
    /// ERESOURCE in nonpaged pool, valid throughout `'a`. No path may delete or
    /// reinitialize it while this view or any acquisition is in use. Concurrent
    /// users must follow the ERESOURCE protocol without creating Rust
    /// references that alias the kernel's mutations.
    pub unsafe fn from_raw(resource: PERESOURCE) -> Result<Self, InvalidPointer> {
        Ok(Self {
            resource: checked_pointer(resource)?,
            lifetime: PhantomData,
        })
    }

    /// Runs a synchronous callback with shared access, waiting for acquisition.
    ///
    /// # Errors
    ///
    /// Returns `ResourceAcquireError` if the resource does not grant access.
    /// The callback is then not invoked and APC delivery is restored.
    ///
    /// # Safety
    ///
    /// IRQL must be at most `APC_LEVEL`. The callback must return on the same
    /// thread at that IRQL, must not release this acquisition or enable normal
    /// kernel APCs, and must not transfer work that depends on this acquisition
    /// beyond the callback. Follow the driver's resource acquisition order.
    pub unsafe fn with_shared<R>(
        &self,
        body: impl FnOnce() -> R,
    ) -> Result<R, ResourceAcquireError> {
        // SAFETY: The caller establishes the scoped acquisition contract.
        unsafe { self.with_access(Access::Shared, true, body) }
    }

    /// Runs a synchronous callback with exclusive access, waiting for
    /// acquisition.
    ///
    /// # Errors
    ///
    /// Returns `ResourceAcquireError` without invoking the callback if access
    /// is not granted; APC delivery is restored.
    ///
    /// # Safety
    ///
    /// The execution and callback requirements of [`Self::with_shared`] apply.
    /// Do not acquire exclusively while holding this resource only in shared
    /// mode; release shared access first to avoid an upgrade deadlock.
    pub unsafe fn with_exclusive<R>(
        &self,
        body: impl FnOnce() -> R,
    ) -> Result<R, ResourceAcquireError> {
        // SAFETY: The caller establishes the scoped exclusive contract.
        unsafe { self.with_access(Access::Exclusive, true, body) }
    }

    /// Runs a callback with shared access only when immediately available.
    ///
    /// # Errors
    ///
    /// Returns `ResourceAcquireError` on contention, without invoking the
    /// callback or retaining an acquisition or critical region.
    ///
    /// # Safety
    ///
    /// The execution and callback requirements of [`Self::with_shared`] apply.
    pub unsafe fn try_with_shared<R>(
        &self,
        body: impl FnOnce() -> R,
    ) -> Result<R, ResourceAcquireError> {
        // SAFETY: The caller establishes the scoped shared contract.
        unsafe { self.with_access(Access::Shared, false, body) }
    }

    /// Runs a callback with exclusive access only when immediately available.
    ///
    /// # Errors
    ///
    /// Returns `ResourceAcquireError` on contention, without invoking the
    /// callback or retaining an acquisition or critical region.
    ///
    /// # Safety
    ///
    /// The execution and callback requirements of [`Self::with_exclusive`]
    /// apply.
    pub unsafe fn try_with_exclusive<R>(
        &self,
        body: impl FnOnce() -> R,
    ) -> Result<R, ResourceAcquireError> {
        // SAFETY: The caller establishes the scoped exclusive contract.
        unsafe { self.with_access(Access::Exclusive, false, body) }
    }

    unsafe fn with_access<R>(
        &self,
        access: Access,
        wait: bool,
        body: impl FnOnce() -> R,
    ) -> Result<R, ResourceAcquireError> {
        // SAFETY: The public operation requires APC_LEVEL or below and keeps
        // region ownership private to this synchronous scope.
        let critical_region = unsafe { CriticalRegionGuard::enter() };
        let acquired = match access {
            // SAFETY: The resource is live, and normal APCs are disabled.
            Access::Shared => unsafe {
                ntddk::ExAcquireResourceSharedLite(self.resource.as_ptr(), u8::from(wait))
            },
            // SAFETY: The resource is live, normal APCs are disabled, and the
            // public operation's contract excludes a shared-to-exclusive upgrade.
            Access::Exclusive => unsafe {
                ntddk::ExAcquireResourceExclusiveLite(self.resource.as_ptr(), u8::from(wait))
            },
        };
        if acquired == 0 {
            return Err(ResourceAcquireError);
        }
        let _acquisition = AcquiredResource {
            resource: self.resource,
            _critical_region: critical_region,
        };
        Ok(body())
    }
}
