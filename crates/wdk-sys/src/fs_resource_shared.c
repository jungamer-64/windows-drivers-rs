// Copyright (c) Microsoft Corporation
// License: MIT OR Apache-2.0

#include "../include/wdk_file_system.h"

BOOLEAN NTAPI wdk_sys_acquire_resource_shared(PERESOURCE resource, BOOLEAN wait)
{
    KeEnterCriticalRegion();
    if (ExAcquireResourceSharedLite(resource, wait)) { return TRUE; }
    KeLeaveCriticalRegion();
    return FALSE;
}
