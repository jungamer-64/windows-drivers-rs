// Copyright (c) Microsoft Corporation
// License: MIT OR Apache-2.0

#pragma once
#include <ntifs.h>

/* Successful acquisition transfers one resource/APC scope to the caller.
 * Release must run on the acquiring thread, including callback handoffs. */
_IRQL_requires_max_(APC_LEVEL)
_When_(return != FALSE, _Acquires_lock_(_Global_critical_region_))
BOOLEAN NTAPI wdk_sys_acquire_resource_shared(
    _Inout_ _When_(return != FALSE, _Acquires_shared_lock_(*_Curr_)) PERESOURCE resource,
    _In_ BOOLEAN wait);
_IRQL_requires_max_(APC_LEVEL)
_When_(return != FALSE, _Acquires_lock_(_Global_critical_region_))
BOOLEAN NTAPI wdk_sys_acquire_resource_exclusive(
    _Inout_ _When_(return != FALSE, _Acquires_exclusive_lock_(*_Curr_)) PERESOURCE resource,
    _In_ BOOLEAN wait);
_IRQL_requires_max_(APC_LEVEL)
_Requires_lock_held_(_Global_critical_region_)
_Releases_lock_(_Global_critical_region_)
VOID NTAPI wdk_sys_release_resource(
    _Inout_ _Requires_lock_held_(*_Curr_) _Releases_lock_(*_Curr_) PERESOURCE resource);

/* Caller-owned header, mutex and optional per-file slot remain stationary until
 * teardown after external quiescence. Success owns one allocated AePushLock;
 * failure publishes no initialized header and retains no allocation. */
_IRQL_requires_max_(APC_LEVEL)
NTSTATUS NTAPI wdk_sys_initialize_fcb_header(
    PFSRTL_ADVANCED_FCB_HEADER header, PFAST_MUTEX mutex,
    PVOID *file_context_slot, ULONG pool_tag);
_IRQL_requires_max_(APC_LEVEL)
VOID NTAPI wdk_sys_teardown_fcb_header(PFSRTL_ADVANCED_FCB_HEADER header);

/* Both calls consume dispatch/completion authority even when the returned status
 * is a failure. A pending IRP preserves its own stack location during forwarding. */
_IRQL_requires_max_(DISPATCH_LEVEL)
NTSTATUS NTAPI wdk_sys_forward_unpended_irp(PDEVICE_OBJECT device, PIRP irp);
_IRQL_requires_max_(DISPATCH_LEVEL)
NTSTATUS NTAPI wdk_sys_forward_pending_irp(PDEVICE_OBJECT device, PIRP irp);
