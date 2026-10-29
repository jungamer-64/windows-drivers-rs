// Copyright (c) Microsoft Corporation
// License: MIT OR Apache-2.0

#include "../include/wdk_file_system.h"

VOID NTAPI wdk_sys_FsRtlExitFileSystem(VOID)
{
    FsRtlExitFileSystem();
}
