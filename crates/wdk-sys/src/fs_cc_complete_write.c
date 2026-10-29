// License: MIT OR Apache-2.0

#include "fs_seh.h"

NTSTATUS NTAPI wdk_sys_cc_complete_mdl_write(PFILE_OBJECT file, LONGLONG position, PMDL chain)
{
    LARGE_INTEGER offset;
    offset.QuadPart = position;
    __try { CcMdlWriteComplete(file, &offset, chain); }
    __except (WDK_SYS_EXPECTED_EXCEPTION) {
        NTSTATUS status = (NTSTATUS)GetExceptionCode();
        CcMdlWriteAbort(file, chain);
        return status;
    }
    return STATUS_SUCCESS;
}
