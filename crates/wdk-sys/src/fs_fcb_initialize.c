// Copyright (c) Microsoft Corporation
// License: MIT OR Apache-2.0

#include "../include/wdk_file_system.h"

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
