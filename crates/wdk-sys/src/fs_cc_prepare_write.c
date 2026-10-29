// License: MIT OR Apache-2.0

#include "fs_seh.h"

NTSTATUS NTAPI wdk_sys_cc_prepare_mdl_write(PFILE_OBJECT file, LONGLONG position,
    ULONG length, PMDL *chain, ULONG_PTR *information)
{
    LARGE_INTEGER offset;
    IO_STATUS_BLOCK result = {0};
    NTSTATUS status;
    offset.QuadPart = position;
    *chain = NULL;
    *information = 0;
    __try {
        CcPrepareMdlWrite(file, &offset, length, chain, &result);
        status = result.Status;
    }
    __except (WDK_SYS_EXPECTED_EXCEPTION) { status = (NTSTATUS)GetExceptionCode(); }
    if (!NT_SUCCESS(status) && *chain != NULL) {
        CcMdlWriteAbort(file, *chain);
        *chain = NULL;
    }
    if (NT_SUCCESS(status)) { *information = result.Information; }
    return status;
}
