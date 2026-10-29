// Copyright (c) Microsoft Corporation
// License: MIT OR Apache-2.0

#include "../include/wdk_file_system.h"

VOID NTAPI wdk_sys_teardown_fcb_header(PFSRTL_ADVANCED_FCB_HEADER header)
{
    PVOID lock = header->AePushLock;
    FsRtlTeardownPerStreamContexts(header);
    FsRtlFreeAePushLock(lock);
    header->AePushLock = NULL;
}
