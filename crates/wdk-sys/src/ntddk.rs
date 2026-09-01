// Copyright (c) Microsoft Corporation
// License: MIT OR Apache-2.0

//! Direct FFI bindings to NTDDK APIs from the Windows Driver Kit (WDK)
//!
//! This module contains all bindings to functions, constants, methods,
//! constructors and destructors in `ntddk.h`. Types are not included in this
//! module, but are available in the top-level `wdk_sys` module.

pub use bindings::*;

/// Reports whether a file-lock structure currently contains locks.
pub use crate::macros::ntifs::wdk_sys_FsRtlAreThereCurrentFileLocks as FsRtlAreThereCurrentFileLocks;
/// Disables delivery of normal kernel APCs for the current thread.
///
/// File-system entry points use this before acquiring resources protected from
/// normal kernel APC delivery. Every successful entry must be paired with
/// [`FsRtlExitFileSystem`] on the same thread. See the
/// [WDK contract](https://learn.microsoft.com/en-us/windows-hardware/drivers/ifs/fsrtlenterfilesystem).
pub use crate::macros::ntifs::wdk_sys_FsRtlEnterFileSystem as FsRtlEnterFileSystem;
/// Re-enables normal kernel APC delivery for a matching
/// [`FsRtlEnterFileSystem`] call.
pub use crate::macros::ntifs::wdk_sys_FsRtlExitFileSystem as FsRtlExitFileSystem;
/// Initializes an advanced FCB header, optional file-context storage, and
/// optional auto-expand push lock according to the `ntifs.h` macro contract.
pub use crate::macros::ntifs::wdk_sys_FsRtlSetupAdvancedHeaderEx2 as FsRtlSetupAdvancedHeaderEx2;
/// Completes an IRP and applies the specified priority boost to the initiating
/// thread.
///
/// This is the callable form of the WDK `IoCompleteRequest` macro.
pub use crate::macros::ntifs::wdk_sys_IoCompleteRequest as IoCompleteRequest;
/// Returns the current I/O stack location of an IRP.
///
/// This is the callable form of the WDK `IoGetCurrentIrpStackLocation` inline
/// function.
pub use crate::macros::ntifs::wdk_sys_IoGetCurrentIrpStackLocation as IoGetCurrentIrpStackLocation;
/// Returns the next-lower I/O stack location of an IRP.
///
/// This is the callable form of the WDK `IoGetNextIrpStackLocation` inline
/// function.
pub use crate::macros::ntifs::wdk_sys_IoGetNextIrpStackLocation as IoGetNextIrpStackLocation;
/// Marks the current IRP stack location as having returned `STATUS_PENDING`.
///
/// A driver that queues an IRP and returns `STATUS_PENDING` must mark the IRP
/// pending before placing it in the queue. After the IRP is queued, only the
/// cancel or completion path may complete it. See the
/// [WDK contract](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/nf-wdm-iomarkirppending).
pub use crate::macros::ntifs::wdk_sys_IoMarkIrpPending as IoMarkIrpPending;
/// Atomically replaces an IRP's cancel routine and returns the previous
/// routine.
///
/// The exchange does not by itself serialize cancel-routine execution. Callers
/// must follow the cancel spin-lock and cancellation-state protocol described
/// by the [WDK contract](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/wdm/nf-wdm-iosetcancelroutine).
pub use crate::macros::ntifs::wdk_sys_IoSetCancelRoutine as IoSetCancelRoutine;

#[allow(missing_docs)]
#[allow(clippy::derive_partial_eq_without_eq)]
mod bindings {
    #[allow(
        clippy::wildcard_imports,
        reason = "the underlying c code relies on all type definitions being in scope, which \
                  results in the bindgen generated code relying on the generated types being in \
                  scope as well"
    )]
    use crate::types::*;

    include!(concat!(env!("OUT_DIR"), "/ntddk.rs"));
}
