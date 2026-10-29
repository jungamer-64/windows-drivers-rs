// Copyright (c) Microsoft Corporation
// License: MIT OR Apache-2.0

#pragma once
#include <ntifs.h>

C_ASSERT(FIELD_OFFSET(FSRTL_ADVANCED_FCB_HEADER, NodeTypeCode) == 0);
C_ASSERT(FIELD_OFFSET(FSRTL_ADVANCED_FCB_HEADER, FastMutex) == sizeof(FSRTL_COMMON_FCB_HEADER));

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

VOID NTAPI wdk_sys_IoCompleteRequest(PIRP Irp, CCHAR PriorityBoost);

NTSTATUS NTAPI wdk_sys_IoCallDriver(PDEVICE_OBJECT DeviceObject, PIRP Irp);

VOID NTAPI wdk_sys_IoSetNextIrpStackLocation(PIRP Irp);

VOID NTAPI wdk_sys_IoSkipCurrentIrpStackLocation(PIRP Irp);

VOID NTAPI wdk_sys_IoCopyCurrentIrpStackLocationToNext(PIRP Irp);

VOID NTAPI wdk_sys_IoSetCompletionRoutine(
    PIRP Irp,
    PIO_COMPLETION_ROUTINE CompletionRoutine,
    PVOID Context,
    BOOLEAN InvokeOnSuccess,
    BOOLEAN InvokeOnError,
    BOOLEAN InvokeOnCancel);

VOID NTAPI wdk_sys_ExInitializeFastMutex(PFAST_MUTEX FastMutex);

PIO_STACK_LOCATION NTAPI wdk_sys_IoGetCurrentIrpStackLocation(PIRP Irp);

PIO_STACK_LOCATION NTAPI wdk_sys_IoGetNextIrpStackLocation(PIRP Irp);

VOID NTAPI wdk_sys_IoMarkIrpPending(PIRP Irp);

PDRIVER_CANCEL NTAPI wdk_sys_IoSetCancelRoutine(PIRP Irp, PDRIVER_CANCEL CancelRoutine);

VOID NTAPI wdk_sys_FsRtlEnterFileSystem(VOID);

VOID NTAPI wdk_sys_FsRtlExitFileSystem(VOID);

VOID NTAPI wdk_sys_FsRtlSetupAdvancedHeaderEx2(
    PFSRTL_ADVANCED_FCB_HEADER AdvancedHeader,
    PFAST_MUTEX FastMutex,
    PVOID *FileContextSupportPointer,
    PVOID AePushLock);

BOOLEAN NTAPI wdk_sys_FsRtlAreThereCurrentFileLocks(PFILE_LOCK FileLock);

BOOLEAN NTAPI wdk_sys_CcIsFileCached(PFILE_OBJECT FileObject);

PLARGE_INTEGER NTAPI wdk_sys_CcGetFileSizePointer(PFILE_OBJECT FileObject);

VOID NTAPI wdk_sys_FsRtlInitPerFileContext(
    PFSRTL_PER_FILE_CONTEXT Context, PVOID OwnerId, PVOID InstanceId, PFREE_FUNCTION FreeCallback);

VOID NTAPI wdk_sys_FsRtlInitPerStreamContext(
    PFSRTL_PER_STREAM_CONTEXT Context, PVOID OwnerId, PVOID InstanceId, PFREE_FUNCTION FreeCallback);

PVOID *NTAPI wdk_sys_FsRtlGetPerFileContextPointer(PFILE_OBJECT FileObject);

PFSRTL_ADVANCED_FCB_HEADER NTAPI wdk_sys_FsRtlGetPerStreamContextPointer(PFILE_OBJECT FileObject);

BOOLEAN NTAPI wdk_sys_FsRtlSupportsPerFileContexts(PFILE_OBJECT FileObject);

BOOLEAN NTAPI wdk_sys_FsRtlSupportsPerStreamContexts(PFILE_OBJECT FileObject);

/* Capture runs in the original requestor process at APC_LEVEL or below. Success
 * owns locked pages and an NX system mapping until release; no Rust references
 * may alias the writable range. Failure publishes neither output. */
_IRQL_requires_max_(APC_LEVEL)
NTSTATUS NTAPI wdk_sys_capture_requestor_output(
    PVOID buffer, ULONG capacity, KPROCESSOR_MODE mode, PMDL *mdl, PVOID *mapping);
_IRQL_requires_max_(DISPATCH_LEVEL)
VOID NTAPI wdk_sys_release_requestor_output(PMDL mdl);
/* Caller owns the mapping and a disjoint initialized source at PASSIVE_LEVEL.
 * The copy does not access the requestor virtual address. */
_IRQL_requires_max_(PASSIVE_LEVEL)
VOID NTAPI wdk_sys_copy_requestor_output(PVOID mapping, const VOID *source, ULONG length);

/* These synchronous Cc boundaries catch only FsRtl-expected exceptions.
 * Acquisition failure completes/aborts partial chains and clears both outputs.
 * A successful chain transfers to the caller's MDL completion protocol.
 * Failed write completion consumes the chain by aborting it; dirty cache or
 * lower writes are not rolled back. File/cache-map lifetime belongs to caller. */
_IRQL_requires_max_(APC_LEVEL)
NTSTATUS NTAPI wdk_sys_cc_read_mdl(PFILE_OBJECT file, LONGLONG position,
    ULONG length, PMDL *chain, ULONG_PTR *information);
_IRQL_requires_max_(APC_LEVEL)
NTSTATUS NTAPI wdk_sys_cc_prepare_mdl_write(PFILE_OBJECT file, LONGLONG position,
    ULONG length, PMDL *chain, ULONG_PTR *information);
_IRQL_requires_max_(APC_LEVEL)
NTSTATUS NTAPI wdk_sys_cc_complete_mdl_write(PFILE_OBJECT file, LONGLONG position, PMDL chain);

/* FsRtl can synchronously complete requests and invoke driver callbacks. These
 * boundaries preserve incoming TopLevelIrp values; an empty value is marked as
 * FSP recursion until native return or SEH unwind. Callback exceptions must
 * remain within native frames; this boundary does not catch FsRtl failures.
 * The caller owns notification/oplock storage, callbacks and request lifetime.
 * Watch subject ownership transfers to FsRtl, as in FullChangeDirectory. */
_IRQL_requires_max_(PASSIVE_LEVEL)
VOID NTAPI wdk_sys_notify_change_directory(PNOTIFY_SYNC sync, PLIST_ENTRY list,
    PVOID context, PSTRING directory, BOOLEAN subtree, BOOLEAN ignore_buffer,
    ULONG filter, PIRP irp, PCHECK_FOR_TRAVERSE_ACCESS traverse, PSECURITY_SUBJECT_CONTEXT subject);
_IRQL_requires_max_(PASSIVE_LEVEL)
VOID NTAPI wdk_sys_notify_report_change(PNOTIFY_SYNC sync, PLIST_ENTRY list, PSTRING target,
    USHORT name_offset, PSTRING stream, PSTRING parent, ULONG filter, ULONG action, PVOID target_context);
_IRQL_requires_max_(APC_LEVEL)
VOID NTAPI wdk_sys_notify_cleanup(PNOTIFY_SYNC sync, PLIST_ENTRY list, PVOID context);
_IRQL_requires_max_(APC_LEVEL)
VOID NTAPI wdk_sys_notify_cleanup_all(PNOTIFY_SYNC sync, PLIST_ENTRY list);
_IRQL_requires_max_(APC_LEVEL)
NTSTATUS NTAPI wdk_sys_oplock_fsctrl(POPLOCK oplock, PIRP irp, ULONG count, ULONG flags);
_IRQL_requires_max_(APC_LEVEL)
NTSTATUS NTAPI wdk_sys_check_oplock(POPLOCK oplock, PIRP irp, ULONG flags,
    PVOID context, POPLOCK_WAIT_COMPLETE_ROUTINE completion, POPLOCK_FS_PREPOST_IRP prepost);
_IRQL_requires_max_(APC_LEVEL)
VOID NTAPI wdk_sys_uninitialize_oplock(POPLOCK oplock);
