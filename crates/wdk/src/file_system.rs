// Copyright (c) Microsoft Corporation
// License: MIT OR Apache-2.0

//! Native file-system driver execution, IRP completion, and borrowed FCB APIs.
//!
//! The driver owns kernel storage, publication, cancellation, and shutdown.
//! These APIs do not allocate, queue IRPs, or own cache maps or oplocks.

use core::{fmt, marker::PhantomData, ptr::NonNull};

pub use self::{
    fcb::FcbHeader,
    irp::{CompletionStatus, DispatchIrp, InvalidCompletionStatus, InvalidDispatchIrp},
    resource::{ResourceAcquireError, ResourceRef},
};

mod fcb;
mod irp;
mod resource;

/// A pointer cannot be admitted as an aligned, non-null kernel object.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InvalidPointer {
    /// No object was supplied.
    Null,
    /// The address does not satisfy the object's alignment.
    Misaligned,
}

impl fmt::Display for InvalidPointer {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Null => "null kernel-object pointer",
            Self::Misaligned => "misaligned kernel-object pointer",
        })
    }
}

impl core::error::Error for InvalidPointer {}

fn checked_pointer<T>(pointer: *mut T) -> Result<NonNull<T>, InvalidPointer> {
    let pointer = NonNull::new(pointer).ok_or(InvalidPointer::Null)?;
    if !pointer.as_ptr().is_aligned() {
        return Err(InvalidPointer::Misaligned);
    }
    Ok(pointer)
}

/// A same-thread guard for a file-system critical region.
///
/// Dropping the guard calls `FsRtlExitFileSystem`. The guard is neither `Send`
/// nor `Sync`, so safe Rust cannot move it to, or share it with, another
/// execution thread.
///
/// Do not hold this guard across deferred work, a thread boundary, or a call to
/// `IoCallDriver`.
#[must_use = "dropping the guard is required to exit the file-system critical region"]
pub struct CriticalRegionGuard {
    thread_affinity: PhantomData<*mut ()>,
}

impl CriticalRegionGuard {
    /// Enters a file-system critical region on the current execution thread.
    ///
    /// # Safety
    ///
    /// The current IRQL must be at most `APC_LEVEL`. The returned guard must
    /// not be leaked and must be dropped on the same execution thread
    /// before returning control across deferred work, a thread boundary, or
    /// `IoCallDriver`.
    pub unsafe fn enter() -> Self {
        // SAFETY: The caller establishes the WDK IRQL and same-thread pairing
        // contract. The non-Send/non-Sync guard owns the matching exit
        // operation.
        unsafe { wdk_sys::ntddk::FsRtlEnterFileSystem() };

        Self {
            thread_affinity: PhantomData,
        }
    }
}

impl Drop for CriticalRegionGuard {
    fn drop(&mut self) {
        // SAFETY: Construction entered the critical region, and the guard's
        // thread-affine type plus the unsafe constructor contract
        // require this drop to occur on that same execution thread.
        unsafe { wdk_sys::ntddk::FsRtlExitFileSystem() };
    }
}
