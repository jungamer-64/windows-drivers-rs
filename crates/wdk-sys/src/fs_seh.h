// License: MIT OR Apache-2.0

#pragma once
#include "../include/wdk_file_system.h"

/* Cache Manager, Memory Manager and FsRtl raises: access violations and misalignment are driver faults. */
#define WDK_SYS_EXPECTED_EXCEPTION \
    (FsRtlIsNtstatusExpected((NTSTATUS)GetExceptionCode()) ? EXCEPTION_EXECUTE_HANDLER : EXCEPTION_CONTINUE_SEARCH)

/* Requestor memory can be invalid or change after any probe, so its access violations and
 * misalignment are request failures. Instruction faults still continue the exception search. */
#define WDK_SYS_REQUESTOR_ACCESS_EXCEPTION \
    (((NTSTATUS)GetExceptionCode() == STATUS_ACCESS_VIOLATION || \
      (NTSTATUS)GetExceptionCode() == STATUS_DATATYPE_MISALIGNMENT || \
      FsRtlIsNtstatusExpected((NTSTATUS)GetExceptionCode())) \
        ? EXCEPTION_EXECUTE_HANDLER : EXCEPTION_CONTINUE_SEARCH)
