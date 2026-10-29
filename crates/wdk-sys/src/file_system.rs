// Copyright (c) Microsoft Corporation
// License: MIT OR Apache-2.0

//! Raw native ownership boundaries. The exported C header is supplied through
//! Cargo's `DEP_WDK_NATIVE_INCLUDE` metadata to direct consumers.

/// Acquires exclusive access with the same resource/APC ownership contract.
pub use crate::macros::ntifs::wdk_sys_acquire_resource_exclusive as acquire_resource_exclusive;
/// Acquires shared access after entering a critical region. Failure restores
/// the incoming APC state. Success requires one same-thread `release_resource`.
pub use crate::macros::ntifs::wdk_sys_acquire_resource_shared as acquire_resource_shared;
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
/// Consumes one successful acquisition and restores its normal-kernel-APC
/// scope. The resource must remain initialized and the current thread must own
/// it.
pub use crate::macros::ntifs::wdk_sys_release_resource as release_resource;
/// Consumes an initialized header's stream contexts and `AePushLock` after all
/// external users have stopped. Does not delete resources or uninitialize
/// oplocks.
pub use crate::macros::ntifs::wdk_sys_teardown_fcb_header as teardown_fcb_header;
