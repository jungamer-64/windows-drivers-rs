// Copyright (c) Microsoft Corporation
// License: MIT OR Apache-2.0

#include "../include/wdk_file_system.h"

C_ASSERT(FIELD_OFFSET(FSRTL_ADVANCED_FCB_HEADER, NodeTypeCode) == 0);
C_ASSERT(FIELD_OFFSET(FSRTL_ADVANCED_FCB_HEADER, FastMutex) == sizeof(FSRTL_COMMON_FCB_HEADER));

VOID NTAPI wdk_sys_IoCompleteRequest(PIRP Irp, CCHAR PriorityBoost)
{
    IoCompleteRequest(Irp, PriorityBoost);
}

NTSTATUS NTAPI wdk_sys_IoCallDriver(PDEVICE_OBJECT DeviceObject, PIRP Irp)
{
    return IoCallDriver(DeviceObject, Irp);
}

VOID NTAPI wdk_sys_IoSetNextIrpStackLocation(PIRP Irp)
{
    IoSetNextIrpStackLocation(Irp);
}

VOID NTAPI wdk_sys_IoSkipCurrentIrpStackLocation(PIRP Irp)
{
    IoSkipCurrentIrpStackLocation(Irp);
}

VOID NTAPI wdk_sys_IoCopyCurrentIrpStackLocationToNext(PIRP Irp)
{
    IoCopyCurrentIrpStackLocationToNext(Irp);
}

VOID NTAPI wdk_sys_IoSetCompletionRoutine(
    PIRP Irp,
    PIO_COMPLETION_ROUTINE CompletionRoutine,
    PVOID Context,
    BOOLEAN InvokeOnSuccess,
    BOOLEAN InvokeOnError,
    BOOLEAN InvokeOnCancel)
{
    IoSetCompletionRoutine(Irp, CompletionRoutine, Context,
                           InvokeOnSuccess, InvokeOnError, InvokeOnCancel);
}

VOID NTAPI wdk_sys_ExInitializeFastMutex(PFAST_MUTEX FastMutex)
{
    ExInitializeFastMutex(FastMutex);
}

PIO_STACK_LOCATION NTAPI wdk_sys_IoGetCurrentIrpStackLocation(PIRP Irp)
{
    return IoGetCurrentIrpStackLocation(Irp);
}

PIO_STACK_LOCATION NTAPI wdk_sys_IoGetNextIrpStackLocation(PIRP Irp)
{
    return IoGetNextIrpStackLocation(Irp);
}

VOID NTAPI wdk_sys_IoMarkIrpPending(PIRP Irp)
{
    IoMarkIrpPending(Irp);
}

PDRIVER_CANCEL NTAPI wdk_sys_IoSetCancelRoutine(PIRP Irp, PDRIVER_CANCEL CancelRoutine)
{
    return IoSetCancelRoutine(Irp, CancelRoutine);
}

VOID NTAPI wdk_sys_FsRtlEnterFileSystem(VOID)
{
    FsRtlEnterFileSystem();
}

VOID NTAPI wdk_sys_FsRtlExitFileSystem(VOID)
{
    FsRtlExitFileSystem();
}

VOID NTAPI wdk_sys_FsRtlSetupAdvancedHeaderEx2(
    PFSRTL_ADVANCED_FCB_HEADER AdvancedHeader,
    PFAST_MUTEX FastMutex,
    PVOID *FileContextSupportPointer,
    PVOID AePushLock)
{
    FsRtlSetupAdvancedHeaderEx2(
        AdvancedHeader,
        FastMutex,
        FileContextSupportPointer,
        AePushLock);
}

BOOLEAN NTAPI wdk_sys_FsRtlAreThereCurrentFileLocks(PFILE_LOCK FileLock)
{
    return FsRtlAreThereCurrentFileLocks(FileLock);
}

BOOLEAN NTAPI wdk_sys_CcIsFileCached(PFILE_OBJECT FileObject)
{
    return CcIsFileCached(FileObject);
}

PLARGE_INTEGER NTAPI wdk_sys_CcGetFileSizePointer(PFILE_OBJECT FileObject)
{
    return CcGetFileSizePointer(FileObject);
}

VOID NTAPI wdk_sys_FsRtlInitPerFileContext(
    PFSRTL_PER_FILE_CONTEXT Context, PVOID OwnerId, PVOID InstanceId, PFREE_FUNCTION FreeCallback)
{
    FsRtlInitPerFileContext(Context, OwnerId, InstanceId, FreeCallback);
}

VOID NTAPI wdk_sys_FsRtlInitPerStreamContext(
    PFSRTL_PER_STREAM_CONTEXT Context, PVOID OwnerId, PVOID InstanceId, PFREE_FUNCTION FreeCallback)
{
    FsRtlInitPerStreamContext(Context, OwnerId, InstanceId, FreeCallback);
}

PVOID *NTAPI wdk_sys_FsRtlGetPerFileContextPointer(PFILE_OBJECT FileObject)
{
    return FsRtlGetPerFileContextPointer(FileObject);
}

PFSRTL_ADVANCED_FCB_HEADER NTAPI wdk_sys_FsRtlGetPerStreamContextPointer(PFILE_OBJECT FileObject)
{
    return FsRtlGetPerStreamContextPointer(FileObject);
}

BOOLEAN NTAPI wdk_sys_FsRtlSupportsPerFileContexts(PFILE_OBJECT FileObject)
{
    return FsRtlSupportsPerFileContexts(FileObject);
}

BOOLEAN NTAPI wdk_sys_FsRtlSupportsPerStreamContexts(PFILE_OBJECT FileObject)
{
    return FsRtlSupportsPerStreamContexts(FileObject);
}

BOOLEAN NTAPI wdk_sys_acquire_resource_shared(PERESOURCE resource, BOOLEAN wait)
{
    KeEnterCriticalRegion();
    if (ExAcquireResourceSharedLite(resource, wait)) { return TRUE; }
    KeLeaveCriticalRegion();
    return FALSE;
}

BOOLEAN NTAPI wdk_sys_acquire_resource_exclusive(PERESOURCE resource, BOOLEAN wait)
{
    BOOLEAN acquired;
    KeEnterCriticalRegion();
    acquired = wait ? ExAcquireResourceExclusiveLite(resource, TRUE)
                    : ExAcquireResourceExclusiveLite(resource, FALSE);
    if (acquired) { return TRUE; }
    KeLeaveCriticalRegion();
    return FALSE;
}

VOID NTAPI wdk_sys_release_resource(PERESOURCE resource)
{
    ExReleaseResourceLite(resource);
    KeLeaveCriticalRegion();
}

NTSTATUS NTAPI wdk_sys_initialize_fcb_header(
    PFSRTL_ADVANCED_FCB_HEADER header, PFAST_MUTEX mutex,
    PVOID *file_context_slot, ULONG pool_tag)
{
    PVOID lock = FsRtlAllocateAePushLock(NonPagedPoolNx, pool_tag);
    if (lock == NULL) { return STATUS_INSUFFICIENT_RESOURCES; }
    ExInitializeFastMutex(mutex);
    FsRtlSetupAdvancedHeaderEx2(header, mutex, file_context_slot, lock);
    return STATUS_SUCCESS;
}

VOID NTAPI wdk_sys_teardown_fcb_header(PFSRTL_ADVANCED_FCB_HEADER header)
{
    PVOID lock = header->AePushLock;
    FsRtlTeardownPerStreamContexts(header);
    FsRtlFreeAePushLock(lock);
    header->AePushLock = NULL;
}

NTSTATUS NTAPI wdk_sys_forward_unpended_irp(PDEVICE_OBJECT device, PIRP irp)
{
    IoSkipCurrentIrpStackLocation(irp);
    return IoCallDriver(device, irp);
}

NTSTATUS NTAPI wdk_sys_forward_pending_irp(PDEVICE_OBJECT device, PIRP irp)
{
    IoCopyCurrentIrpStackLocationToNext(irp);
    return IoCallDriver(device, irp);
}
