// License: MIT OR Apache-2.0

#include "fs_recursion.h"

VOID NTAPI wdk_sys_notify_change_directory(PNOTIFY_SYNC sync, PLIST_ENTRY list, PVOID context,
    PSTRING directory, BOOLEAN subtree, BOOLEAN ignore_buffer, ULONG filter, PIRP irp,
    PCHECK_FOR_TRAVERSE_ACCESS traverse, PSECURITY_SUBJECT_CONTEXT subject)
{
    PIRP previous = wdk_sys_enter_fsrtl();
    __try {
        FsRtlNotifyFullChangeDirectory(sync, list, context, directory,
            subtree, ignore_buffer, filter, irp, traverse, subject);
    }
    __finally { IoSetTopLevelIrp(previous); }
}

VOID NTAPI wdk_sys_notify_report_change(PNOTIFY_SYNC sync, PLIST_ENTRY list, PSTRING target,
    USHORT name_offset, PSTRING stream, PSTRING parent, ULONG filter, ULONG action, PVOID target_context)
{
    PIRP previous = wdk_sys_enter_fsrtl();
    __try {
        FsRtlNotifyFullReportChange(sync, list, target, name_offset,
            stream, parent, filter, action, target_context);
    }
    __finally { IoSetTopLevelIrp(previous); }
}

VOID NTAPI wdk_sys_notify_cleanup(PNOTIFY_SYNC sync, PLIST_ENTRY list, PVOID context)
{
    PIRP previous = wdk_sys_enter_fsrtl();
    __try { FsRtlNotifyCleanup(sync, list, context); }
    __finally { IoSetTopLevelIrp(previous); }
}

VOID NTAPI wdk_sys_notify_cleanup_all(PNOTIFY_SYNC sync, PLIST_ENTRY list)
{
    PIRP previous = wdk_sys_enter_fsrtl();
    __try { FsRtlNotifyCleanupAll(sync, list); }
    __finally { IoSetTopLevelIrp(previous); }
}

