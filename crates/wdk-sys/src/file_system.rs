// Copyright (c) Microsoft Corporation
// License: MIT OR Apache-2.0

//! Raw native ownership boundaries. The exported C header is supplied through
//! Cargo's `DEP_WDK_NATIVE_INCLUDE` metadata to direct consumers.

/// Acquires exclusive access with the same resource/APC ownership contract.
pub use crate::macros::ntifs::wdk_sys_acquire_resource_exclusive as acquire_resource_exclusive;
/// Acquires shared access after entering a critical region. Failure restores
/// the incoming APC state. Success requires one same-thread `release_resource`.
pub use crate::macros::ntifs::wdk_sys_acquire_resource_shared as acquire_resource_shared;
/// Locks and maps a nonempty requestor output in the original process. Failure
/// publishes no owner.
pub use crate::macros::ntifs::wdk_sys_capture_requestor_output as capture_requestor_output;
/// Consumes a write chain on every result. Failure aborts ownership without
/// promising data rollback.
pub use crate::macros::ntifs::wdk_sys_cc_complete_mdl_write as cc_complete_mdl_write;
/// Acquires a Cc write chain; expected SEH failures abort partial chains and
/// zero both outputs.
pub use crate::macros::ntifs::wdk_sys_cc_prepare_mdl_write as cc_prepare_mdl_write;
/// Acquires a Cc read chain; expected SEH failures release partial chains and
/// zero both outputs.
pub use crate::macros::ntifs::wdk_sys_cc_read_mdl as cc_read_mdl;
/// Checks an oplock under an SEH-restored recursion marker, including
/// synchronous callbacks.
pub use crate::macros::ntifs::wdk_sys_check_oplock as check_oplock;
/// Copies disjoint initialized bytes into the locked system mapping at
/// PASSIVE_LEVEL.
pub use crate::macros::ntifs::wdk_sys_copy_requestor_output as copy_requestor_output;
/// Transfers an already-pending incoming IRP using an independent lower stack.
/// Its current pending bit remains intact. Storage and ownership requirements
/// match forward_unpended_irp; no driver completion callback may remain
/// installed.
pub use crate::macros::ntifs::wdk_sys_forward_pending_irp as forward_pending_irp;
/// Transfers an unpended incoming IRP without a completion callback, reusing
/// its current stack. The lower device remains live through submission. Any
/// returned status consumes completion authority; callers must not use the IRP
/// afterward.
pub use crate::macros::ntifs::wdk_sys_forward_unpended_irp as forward_unpended_irp;
/// Initializes caller-owned stationary header/mutex storage and allocates its
/// `AePushLock`. Failure retains no allocation and publishes no initialized
/// header. The optional slot belongs to the owning per-file structure. Base
/// header fields must already be initialized. Caller runs at `APC_LEVEL` or
/// below.
pub use crate::macros::ntifs::wdk_sys_initialize_fcb_header as initialize_fcb_header;
/// Registers a watch under an SEH-restored FsRtl recursion marker; the WDK
/// subject transfer contract applies.
pub use crate::macros::ntifs::wdk_sys_notify_change_directory as notify_change_directory;
/// Retires one notification context under an SEH-restored recursion marker.
pub use crate::macros::ntifs::wdk_sys_notify_cleanup as notify_cleanup;
/// Drains a notification list under an SEH-restored recursion marker.
pub use crate::macros::ntifs::wdk_sys_notify_cleanup_all as notify_cleanup_all;
/// Reports a change under an SEH-restored FsRtl recursion marker.
pub use crate::macros::ntifs::wdk_sys_notify_report_change as notify_report_change;
/// Processes oplock control under an SEH-restored recursion marker.
pub use crate::macros::ntifs::wdk_sys_oplock_fsctrl as oplock_fsctrl;
/// Consumes one locked MDL after all mapping users have stopped; runs at
/// DISPATCH_LEVEL or below.
pub use crate::macros::ntifs::wdk_sys_release_requestor_output as release_requestor_output;
/// Consumes one successful acquisition and restores its normal-kernel-APC
/// scope. The resource must remain initialized and the current thread must own
/// it.
pub use crate::macros::ntifs::wdk_sys_release_resource as release_resource;
/// Consumes an initialized header's stream contexts and `AePushLock` after all
/// external users have stopped. Does not delete resources or uninitialize
/// oplocks.
pub use crate::macros::ntifs::wdk_sys_teardown_fcb_header as teardown_fcb_header;
/// Retires oplock storage under an SEH-restored recursion marker.
pub use crate::macros::ntifs::wdk_sys_uninitialize_oplock as uninitialize_oplock;
