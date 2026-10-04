// Copyright (c) Microsoft Corporation
// License: MIT OR Apache-2.0

use core::{fmt, ptr::NonNull};

use wdk_sys::{
    CCHAR,
    IO_STACK_LOCATION,
    IRP,
    NTSTATUS,
    PIRP,
    STATUS_MORE_PROCESSING_REQUIRED,
    STATUS_PENDING,
    ULONG_PTR,
    ntddk,
};

use super::{InvalidPointer, checked_pointer};

/// A final status for synchronous dispatch completion.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CompletionStatus(NTSTATUS);

/// An IRP continuation status cannot represent synchronous completion.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InvalidCompletionStatus {
    /// The IRP is still pending.
    Pending,
    /// A completion routine retains responsibility for further processing.
    MoreProcessingRequired,
}

impl fmt::Display for InvalidCompletionStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Pending => "a pending IRP cannot be completed synchronously",
            Self::MoreProcessingRequired => "further processing is not a final dispatch status",
        })
    }
}

impl core::error::Error for InvalidCompletionStatus {}

impl TryFrom<NTSTATUS> for CompletionStatus {
    type Error = InvalidCompletionStatus;

    /// Admits a final dispatch status.
    ///
    /// # Errors
    ///
    /// Rejects `STATUS_PENDING` and `STATUS_MORE_PROCESSING_REQUIRED`.
    fn try_from(status: NTSTATUS) -> Result<Self, Self::Error> {
        match status {
            STATUS_PENDING => Err(InvalidCompletionStatus::Pending),
            STATUS_MORE_PROCESSING_REQUIRED => Err(InvalidCompletionStatus::MoreProcessingRequired),
            _ => Ok(Self(status)),
        }
    }
}

impl From<CompletionStatus> for NTSTATUS {
    fn from(status: CompletionStatus) -> Self {
        status.0
    }
}

/// An IRP cannot be admitted for synchronous processing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InvalidDispatchIrp {
    /// The IRP address is null or misaligned.
    Pointer(InvalidPointer),
    /// The current location is outside the IRP's stack.
    NoCurrentStack,
    /// The current stack address is null or misaligned.
    StackPointer(InvalidPointer),
}

impl fmt::Display for InvalidDispatchIrp {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pointer(error) => write!(formatter, "invalid IRP: {error}"),
            Self::NoCurrentStack => formatter.write_str("IRP has no current stack location"),
            Self::StackPointer(error) => write!(formatter, "invalid IRP stack: {error}"),
        }
    }
}

impl core::error::Error for InvalidDispatchIrp {}

/// Exclusive synchronous processing and completion authority for an incoming
/// IRP.
///
/// This thread-affine value is neither cloneable nor shareable. Completion
/// consumes it. Dropping it does not complete or free the IRP: the dispatch
/// owner remains responsible for an unfinished request. It cannot be used for
/// driver-allocated IRPs, queued requests, or requests already sent down-stack.
#[must_use = "an incoming IRP requires an explicit completion or an external owner"]
pub struct DispatchIrp {
    irp: NonNull<IRP>,
}

impl DispatchIrp {
    /// Takes synchronous processing authority from a driver dispatch boundary.
    ///
    /// # Errors
    ///
    /// Rejects null or misaligned IRP/stack addresses and an absent current
    /// stack location. Rejection does not consume or complete the request.
    ///
    /// # Safety
    ///
    /// A non-null, aligned pointer must designate a live, initialized incoming
    /// IRP. If its current-stack address is non-null and aligned, that address
    /// must designate live, initialized stack storage. The caller must
    /// exclusively own processing and completion, with no racing cancel
    /// routine or lower driver. IRP and stack storage must remain resident in
    /// nonpaged memory. Until completion or drop, no other path may complete,
    /// free, queue, or mutate the IRP/stack. After completion the caller
    /// must not use any original pointer or alias.
    pub unsafe fn from_raw(irp: PIRP) -> Result<Self, InvalidDispatchIrp> {
        let irp = checked_pointer(irp).map_err(InvalidDispatchIrp::Pointer)?;
        // SAFETY: The caller supplies a live, initialized, exclusively owned
        // IRP.
        let request = unsafe { irp.as_ref() };
        if request.CurrentLocation <= 0 || request.CurrentLocation > request.StackCount {
            return Err(InvalidDispatchIrp::NoCurrentStack);
        }
        // SAFETY: The initialized IRP contains the current stack pointer.
        let stack = unsafe { ntddk::IoGetCurrentIrpStackLocation(irp.as_ptr()) };
        checked_pointer(stack).map_err(InvalidDispatchIrp::StackPointer)?;
        Ok(Self { irp })
    }

    /// Borrows the current stack while this value owns synchronous processing.
    #[must_use]
    pub fn current_stack(&self) -> &IO_STACK_LOCATION {
        // SAFETY: Admission established a current location and exclusive,
        // stable IRP ownership; this API cannot change that location.
        let stack = unsafe { ntddk::IoGetCurrentIrpStackLocation(self.irp.as_ptr()) };
        // SAFETY: Admission checked alignment and non-nullness; the constructor
        // contract keeps the initialized stack alive for this borrow.
        unsafe { &*stack }
    }

    /// Publishes the final status and information, completes the IRP, and
    /// returns the saved dispatch status without accessing the completed IRP.
    /// `information` retains its request-specific WDK meaning.
    ///
    /// # Safety
    ///
    /// IRQL must be at most `DISPATCH_LEVEL`, with no spin lock held. The
    /// status and information must be appropriate for this request.
    /// `priority_boost` must be a system-defined boost; errors require
    /// `IO_NO_INCREMENT`. No resource needed by an upper completion
    /// callback may be held.
    #[must_use = "return this saved status from the dispatch routine"]
    pub unsafe fn complete(
        mut self,
        status: CompletionStatus,
        information: ULONG_PTR,
        priority_boost: CCHAR,
    ) -> NTSTATUS {
        let status = NTSTATUS::from(status);
        // SAFETY: The constructor grants exclusive access until completion.
        let request = unsafe { self.irp.as_mut() };
        request.IoStatus.__bindgen_anon_1.Status = status;
        request.IoStatus.Information = information;
        // SAFETY: The caller establishes the completion execution context.
        // Consuming self removes the only Rust processing authority.
        unsafe { ntddk::IoCompleteRequest(self.irp.as_ptr(), priority_boost) };
        status
    }
}
