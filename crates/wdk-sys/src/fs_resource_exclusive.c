// Copyright (c) Microsoft Corporation
// License: MIT OR Apache-2.0

#include "../include/wdk_file_system.h"

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
