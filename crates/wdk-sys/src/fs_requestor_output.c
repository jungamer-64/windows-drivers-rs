// License: MIT OR Apache-2.0

#include "fs_seh.h"

NTSTATUS NTAPI wdk_sys_capture_requestor_output(
    PVOID buffer, ULONG capacity, KPROCESSOR_MODE mode, PMDL *mdl, PVOID *mapping)
{
    PMDL pages;
    NTSTATUS status = STATUS_SUCCESS;
    if (mdl != NULL) { *mdl = NULL; }
    if (mapping != NULL) { *mapping = NULL; }
    if (mdl == NULL || mapping == NULL || capacity == 0 ||
        (mode != KernelMode && mode != UserMode)) { return STATUS_INVALID_PARAMETER; }
    if (buffer == NULL) { return STATUS_INVALID_USER_BUFFER; }
    pages = IoAllocateMdl(buffer, capacity, FALSE, FALSE, NULL);
    if (pages == NULL) { return STATUS_INSUFFICIENT_RESOURCES; }
    __try { MmProbeAndLockPages(pages, mode, IoWriteAccess); }
    __except (WDK_SYS_EXPECTED_EXCEPTION) {
        status = (NTSTATUS)GetExceptionCode();
        if (status != STATUS_ACCESS_VIOLATION && status != STATUS_DATATYPE_MISALIGNMENT &&
            status != STATUS_IN_PAGE_ERROR) { status = STATUS_INVALID_USER_BUFFER; }
    }
    if (!NT_SUCCESS(status)) { IoFreeMdl(pages); return status; }
    *mapping = MmGetSystemAddressForMdlSafe(pages,
        (MM_PAGE_PRIORITY)(NormalPagePriority | MdlMappingNoExecute));
    if (*mapping == NULL) {
        MmUnlockPages(pages);
        IoFreeMdl(pages);
        return STATUS_INSUFFICIENT_RESOURCES;
    }
    *mdl = pages;
    return STATUS_SUCCESS;
}

VOID NTAPI wdk_sys_release_requestor_output(PMDL mdl)
{
    MmUnlockPages(mdl);
    IoFreeMdl(mdl);
}

VOID NTAPI wdk_sys_copy_requestor_output(PVOID mapping, const VOID *source, ULONG length)
{
    RtlCopyVolatileMemory(mapping, source, length);
}
