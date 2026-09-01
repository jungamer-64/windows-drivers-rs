// Copyright (c) Microsoft Corporation
// License: MIT OR Apache-2.0

#include <ntifs.h>

VOID NTAPI wdk_sys_IoCompleteRequest(PIRP Irp, CCHAR PriorityBoost)
{
    IoCompleteRequest(Irp, PriorityBoost);
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
