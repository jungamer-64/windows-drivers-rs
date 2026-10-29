// License: MIT OR Apache-2.0

#include "fs_recursion.h"

NTSTATUS NTAPI wdk_sys_oplock_fsctrl(POPLOCK oplock, PIRP irp, ULONG count, ULONG flags)
{
    NTSTATUS status;
    PIRP previous = wdk_sys_enter_fsrtl();
    __try { status = FsRtlOplockFsctrlEx(oplock, irp, count, flags); }
    __finally { IoSetTopLevelIrp(previous); }
    return status;
}

NTSTATUS NTAPI wdk_sys_check_oplock(POPLOCK oplock, PIRP irp, ULONG flags,
    PVOID context, POPLOCK_WAIT_COMPLETE_ROUTINE completion, POPLOCK_FS_PREPOST_IRP prepost)
{
    NTSTATUS status;
    PIRP previous = wdk_sys_enter_fsrtl();
    __try { status = FsRtlCheckOplockEx(oplock, irp, flags, context, completion, prepost); }
    __finally { IoSetTopLevelIrp(previous); }
    return status;
}

VOID NTAPI wdk_sys_uninitialize_oplock(POPLOCK oplock)
{
    PIRP previous = wdk_sys_enter_fsrtl();
    __try { FsRtlUninitializeOplock(oplock); }
    __finally { IoSetTopLevelIrp(previous); }
}
