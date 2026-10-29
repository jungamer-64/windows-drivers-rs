// License: MIT OR Apache-2.0

#pragma once
#include "../include/wdk_file_system.h"

/* The value is opaque: it may be an IRP or a Cc/MM/FsRtl sentinel. */
static PIRP wdk_sys_enter_fsrtl(VOID)
{
    PIRP previous = IoGetTopLevelIrp();
    if (previous == NULL) { IoSetTopLevelIrp((PIRP)FSRTL_FSP_TOP_LEVEL_IRP); }
    return previous;
}
