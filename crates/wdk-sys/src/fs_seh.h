// License: MIT OR Apache-2.0

#pragma once
#include "../include/wdk_file_system.h"

#define WDK_SYS_EXPECTED_EXCEPTION \
    (FsRtlIsNtstatusExpected((NTSTATUS)GetExceptionCode()) ? EXCEPTION_EXECUTE_HANDLER : EXCEPTION_CONTINUE_SEARCH)
