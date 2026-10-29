// License: MIT OR Apache-2.0
/* Host oracles call the linked production exports using the installed WDK ABI.
 * Faults stay entirely in C between the raising stub and its native SEH frame. */
#include <wdk_file_system.h>
#include <assert.h>

__declspec(dllimport) VOID NTAPI RaiseException(ULONG code, ULONG flags, ULONG count, const ULONG_PTR *arguments);

static __declspec(thread) MDL output_mdl;
static __declspec(thread) PVOID output_buffer;
static __declspec(thread) ULONG mdl_allocations, mdl_frees, page_locks, page_unlocks, copies;
static __declspec(thread) BOOLEAN allocate_failure, mapping_failure;
static __declspec(thread) NTSTATUS probe_exception;

static PMDL NTAPI test_allocate(PVOID buffer, ULONG length, BOOLEAN secondary, BOOLEAN quota, PIRP irp)
{
    assert(length != 0 && !secondary && !quota && irp == NULL);
    if (allocate_failure) { return NULL; }
    RtlZeroMemory(&output_mdl, sizeof(output_mdl));
    output_buffer = buffer;
    mdl_allocations++;
    return &output_mdl;
}
static VOID NTAPI test_free(PMDL mdl) { assert(mdl == &output_mdl); mdl_frees++; }
static VOID NTAPI test_probe(PMDL mdl, KPROCESSOR_MODE mode, LOCK_OPERATION access)
{
    assert(mdl == &output_mdl && (mode == UserMode || mode == KernelMode) && access == IoWriteAccess);
    if (probe_exception == STATUS_ACCESS_VIOLATION) {
        volatile ULONG *invalid = NULL;
        *invalid = 0;
    }
    if (probe_exception != STATUS_SUCCESS) { RaiseException((ULONG)probe_exception, 0, 0, NULL); }
    page_locks++;
}
static VOID NTAPI test_unlock(PMDL mdl) { assert(mdl == &output_mdl); page_unlocks++; }
static PVOID NTAPI test_map(PMDL mdl, KPROCESSOR_MODE mode, MEMORY_CACHING_TYPE cache,
    PVOID address, ULONG bugcheck, ULONG priority)
{
    assert(mdl == &output_mdl && mode == KernelMode && cache == MmCached && address == NULL && !bugcheck);
    assert((priority & MdlMappingNoExecute) != 0);
    return mapping_failure ? NULL : output_buffer;
}
static BOOLEAN NTAPI test_expected(NTSTATUS status) { return status != (NTSTATUS)0xE0000001UL; }

PMDL (NTAPI *__imp_IoAllocateMdl)(PVOID, ULONG, BOOLEAN, BOOLEAN, PIRP) = test_allocate;
VOID (NTAPI *__imp_IoFreeMdl)(PMDL) = test_free;
VOID (NTAPI *__imp_MmProbeAndLockPages)(PMDL, KPROCESSOR_MODE, LOCK_OPERATION) = test_probe;
VOID (NTAPI *__imp_MmUnlockPages)(PMDL) = test_unlock;
PVOID (NTAPI *__imp_MmMapLockedPagesSpecifyCache)(PMDL, KPROCESSOR_MODE, MEMORY_CACHING_TYPE,
    PVOID, ULONG, ULONG) = test_map;
BOOLEAN (NTAPI *__imp_FsRtlIsNtstatusExpected)(NTSTATUS) = test_expected;

volatile VOID *RtlCopyVolatileMemory(volatile VOID *destination, const volatile VOID *source, SIZE_T length)
{
    SIZE_T index;
    for (index = 0; index < length; index++) {
        ((volatile UCHAR *)destination)[index] = ((const volatile UCHAR *)source)[index];
    }
    copies++;
    return destination;
}

VOID wdk_test_reset_output(VOID)
{
    mdl_allocations = mdl_frees = page_locks = page_unlocks = copies = 0;
    allocate_failure = mapping_failure = FALSE;
    probe_exception = STATUS_SUCCESS;
}
ULONG wdk_test_output_balance(VOID)
{
    assert(mdl_allocations == mdl_frees && page_locks == page_unlocks);
    return copies;
}
VOID wdk_test_output_faults(VOID)
{
    UCHAR buffer[32];
    PMDL mdl;
    PVOID mapping;
    ULONG index;
    const NTSTATUS raised[] = { STATUS_ACCESS_VIOLATION, STATUS_DATATYPE_MISALIGNMENT,
        STATUS_IN_PAGE_ERROR, STATUS_INSUFFICIENT_RESOURCES };
    wdk_test_reset_output();
    assert(wdk_sys_capture_requestor_output(NULL, 32, UserMode, &mdl, &mapping) == STATUS_INVALID_USER_BUFFER);
    assert(wdk_sys_capture_requestor_output(buffer, 0, UserMode, &mdl, &mapping) == STATUS_INVALID_PARAMETER);
    assert(wdk_sys_capture_requestor_output(buffer, 32, (KPROCESSOR_MODE)3, &mdl, &mapping) == STATUS_INVALID_PARAMETER);
    assert(wdk_sys_capture_requestor_output(buffer, 32, UserMode, NULL, &mapping) == STATUS_INVALID_PARAMETER);
    assert(wdk_sys_capture_requestor_output(buffer, 32, UserMode, &mdl, NULL) == STATUS_INVALID_PARAMETER);
    allocate_failure = TRUE;
    assert(wdk_sys_capture_requestor_output(buffer, 32, UserMode, &mdl, &mapping) == STATUS_INSUFFICIENT_RESOURCES);
    assert(mdl == NULL && mapping == NULL);
    allocate_failure = FALSE;
    for (index = 0; index < RTL_NUMBER_OF(raised); index++) {
        probe_exception = raised[index];
        assert(wdk_sys_capture_requestor_output(buffer, 32, UserMode, &mdl, &mapping) ==
            (index == 3 ? STATUS_INVALID_USER_BUFFER : raised[index]));
        assert(mdl == NULL && mapping == NULL && page_unlocks == 0);
        assert(mdl_allocations == mdl_frees);
    }
    probe_exception = STATUS_SUCCESS;
    mapping_failure = TRUE;
    assert(wdk_sys_capture_requestor_output(buffer, 32, UserMode, &mdl, &mapping) == STATUS_INSUFFICIENT_RESOURCES);
    assert(mdl == NULL && mapping == NULL && page_unlocks == 1);
    assert(wdk_test_output_balance() == 0);
}

static __declspec(thread) MDL cache_mdl;
static __declspec(thread) FILE_OBJECT cache_file;
static __declspec(thread) NTSTATUS cache_status, cache_exception, write_exception;
static __declspec(thread) ULONG read_releases, write_aborts, write_completions;
static VOID NTAPI test_cc_acquire(PFILE_OBJECT file, PLARGE_INTEGER position, ULONG length,
    PMDL *chain, PIO_STATUS_BLOCK result)
{
    assert(file == &cache_file && position->QuadPart == 80 && length == 20 && *chain == NULL);
    *chain = &cache_mdl;
    result->Status = cache_status;
    result->Information = length;
    if (cache_exception != STATUS_SUCCESS) { RaiseException((ULONG)cache_exception, 0, 0, NULL); }
}
static VOID NTAPI test_cc_read_complete(PFILE_OBJECT file, PMDL chain)
{ assert(file == &cache_file && chain == &cache_mdl); read_releases++; }
static VOID NTAPI test_cc_abort(PFILE_OBJECT file, PMDL chain)
{ assert(file == &cache_file && chain == &cache_mdl); write_aborts++; }
static VOID NTAPI test_cc_write_complete(PFILE_OBJECT file, PLARGE_INTEGER position, PMDL chain)
{
    assert(file == &cache_file && position->QuadPart == 80 && chain == &cache_mdl);
    write_completions++;
    if (write_exception != STATUS_SUCCESS) { RaiseException((ULONG)write_exception, 0, 0, NULL); }
}
VOID (NTAPI *__imp_CcMdlRead)(PFILE_OBJECT, PLARGE_INTEGER, ULONG, PMDL *, PIO_STATUS_BLOCK) = test_cc_acquire;
VOID (NTAPI *__imp_CcPrepareMdlWrite)(PFILE_OBJECT, PLARGE_INTEGER, ULONG, PMDL *, PIO_STATUS_BLOCK) = test_cc_acquire;
VOID (NTAPI *__imp_CcMdlReadComplete)(PFILE_OBJECT, PMDL) = test_cc_read_complete;
VOID (NTAPI *__imp_CcMdlWriteAbort)(PFILE_OBJECT, PMDL) = test_cc_abort;
VOID (NTAPI *__imp_CcMdlWriteComplete)(PFILE_OBJECT, PLARGE_INTEGER, PMDL) = test_cc_write_complete;

VOID wdk_test_cache_mdl(VOID)
{
    ULONG index;
    PMDL chain = NULL;
    ULONG_PTR information = 99;
    read_releases = write_aborts = write_completions = 0;
    cache_exception = write_exception = STATUS_SUCCESS;
    cache_status = STATUS_SUCCESS;
    assert(wdk_sys_cc_read_mdl(&cache_file, 80, 20, &chain, &information) == STATUS_SUCCESS);
    assert(chain == &cache_mdl && information == 20);
    CcMdlReadComplete(&cache_file, chain); chain = NULL;
    assert(wdk_sys_cc_prepare_mdl_write(&cache_file, 80, 20, &chain, &information) == STATUS_SUCCESS);
    assert(chain == &cache_mdl && information == 20);
    assert(wdk_sys_cc_complete_mdl_write(&cache_file, 80, chain) == STATUS_SUCCESS);
    chain = NULL;
    for (index = 0; index != 2; index++) {
        cache_status = index == 0 ? STATUS_UNSUCCESSFUL : STATUS_SUCCESS;
        cache_exception = index == 1 ? STATUS_UNSUCCESSFUL : STATUS_SUCCESS;
        assert(wdk_sys_cc_read_mdl(&cache_file, 80, 20, &chain, &information) == STATUS_UNSUCCESSFUL);
        assert(chain == NULL && information == 0);
        assert(wdk_sys_cc_prepare_mdl_write(&cache_file, 80, 20, &chain, &information) == STATUS_UNSUCCESSFUL);
        assert(chain == NULL && information == 0);
    }
    cache_status = STATUS_SUCCESS; cache_exception = STATUS_SUCCESS;
    assert(wdk_sys_cc_prepare_mdl_write(&cache_file, 80, 20, &chain, &information) == STATUS_SUCCESS);
    write_exception = STATUS_UNSUCCESSFUL;
    assert(wdk_sys_cc_complete_mdl_write(&cache_file, 80, chain) == STATUS_UNSUCCESSFUL);
    assert(read_releases == 3 && write_aborts == 3 && write_completions == 2);
}

static __declspec(thread) NTSTATUS recursion_exception;
static __declspec(thread) PIRP expected_marker;
static __declspec(thread) ULONG observed, nesting;
static __declspec(thread) NTSTATUS oplock_status;
static __declspec(thread) LIST_ENTRY notify_list;
static __declspec(thread) IRP notify_irp;
static __declspec(thread) OPLOCK oplock;
static __declspec(thread) SECURITY_SUBJECT_CONTEXT subject;
static __declspec(thread) ULONG callback_context;
static __declspec(thread) STRING name;

static VOID observe(VOID)
{
    assert(IoGetTopLevelIrp() == expected_marker && expected_marker != NULL);
    observed++;
    if (nesting != 0) {
        nesting = 0;
        wdk_sys_notify_cleanup_all(NULL, &notify_list);
        assert(IoGetTopLevelIrp() == expected_marker);
    }
    if (recursion_exception != STATUS_SUCCESS) { RaiseException((ULONG)recursion_exception, 0, 0, NULL); }
}
static BOOLEAN traverse(PVOID context, PVOID target, PSECURITY_SUBJECT_CONTEXT original)
{ assert(context == &callback_context && target == &name && original == &subject); observe(); return TRUE; }
static VOID wait_complete(PVOID context, PIRP irp)
{ assert(context == &callback_context && irp == &notify_irp); observe(); }
static VOID prepost(PVOID context, PIRP irp)
{ assert(context == &callback_context && irp == &notify_irp); observe(); }
static VOID NTAPI test_register(PNOTIFY_SYNC sync, PLIST_ENTRY list, PVOID context, PSTRING directory,
    BOOLEAN subtree, BOOLEAN ignore, ULONG filter, PIRP irp, PCHECK_FOR_TRAVERSE_ACCESS callback,
    PSECURITY_SUBJECT_CONTEXT original)
{
    assert(sync == NULL && list == &notify_list && context == &callback_context && directory == &name);
    assert(subtree && !ignore && filter == 0x13 && irp == &notify_irp);
    if (original != NULL) { assert(callback == traverse && original == &subject); callback(context, directory, original); }
    else { assert(callback == NULL); }
    observe();
}
static VOID NTAPI test_report(PNOTIFY_SYNC sync, PLIST_ENTRY list, PSTRING target, USHORT offset,
    PSTRING stream, PSTRING parent, ULONG filter, ULONG action, PVOID context)
{
    assert(sync == NULL && list == &notify_list && target == &name && offset == 6);
    assert(stream == NULL && parent == NULL && filter == 0x13 && action == 2 && context == &callback_context);
    observe();
}
static VOID NTAPI test_cleanup(PNOTIFY_SYNC sync, PLIST_ENTRY list, PVOID context)
{ assert(sync == NULL && list == &notify_list && context == &callback_context); observe(); }
static VOID NTAPI test_cleanup_all(PNOTIFY_SYNC sync, PLIST_ENTRY list)
{ assert(sync == NULL && list == &notify_list); observe(); }
static NTSTATUS NTAPI test_fsctrl(POPLOCK state, PIRP irp, ULONG count, ULONG flags)
{ assert(state == &oplock && irp == &notify_irp && count == 3 && flags == 0x10); observe(); return oplock_status; }
static NTSTATUS NTAPI test_check(POPLOCK state, PIRP irp, ULONG flags, PVOID context,
    POPLOCK_WAIT_COMPLETE_ROUTINE completion, POPLOCK_FS_PREPOST_IRP before_post)
{
    assert(state == &oplock && irp == &notify_irp && flags == 0x20);
    if (context != NULL) {
        assert(context == &callback_context && completion == wait_complete && before_post == prepost);
        before_post(context, irp); completion(context, irp);
    } else { assert(completion == NULL && before_post == NULL); }
    observe(); return oplock_status;
}
static VOID NTAPI test_uninitialize(POPLOCK state) { assert(state == &oplock); observe(); }
VOID (NTAPI *__imp_FsRtlNotifyFullChangeDirectory)(PNOTIFY_SYNC, PLIST_ENTRY, PVOID, PSTRING,
    BOOLEAN, BOOLEAN, ULONG, PIRP, PCHECK_FOR_TRAVERSE_ACCESS, PSECURITY_SUBJECT_CONTEXT) = test_register;
VOID (NTAPI *__imp_FsRtlNotifyFullReportChange)(PNOTIFY_SYNC, PLIST_ENTRY, PSTRING, USHORT,
    PSTRING, PSTRING, ULONG, ULONG, PVOID) = test_report;
VOID (NTAPI *__imp_FsRtlNotifyCleanup)(PNOTIFY_SYNC, PLIST_ENTRY, PVOID) = test_cleanup;
VOID (NTAPI *__imp_FsRtlNotifyCleanupAll)(PNOTIFY_SYNC, PLIST_ENTRY) = test_cleanup_all;
NTSTATUS (NTAPI *__imp_FsRtlOplockFsctrlEx)(POPLOCK, PIRP, ULONG, ULONG) = test_fsctrl;
NTSTATUS (NTAPI *__imp_FsRtlCheckOplockEx)(POPLOCK, PIRP, ULONG, PVOID,
    POPLOCK_WAIT_COMPLETE_ROUTINE, POPLOCK_FS_PREPOST_IRP) = test_check;
VOID (NTAPI *__imp_FsRtlUninitializeOplock)(POPLOCK) = test_uninitialize;

static VOID invoke(ULONG operation)
{
    switch (operation) {
    case 0: wdk_sys_notify_change_directory(NULL, &notify_list, &callback_context, &name, TRUE,
        FALSE, 0x13, &notify_irp, traverse, &subject); break;
    case 1: wdk_sys_notify_report_change(NULL, &notify_list, &name, 6, NULL, NULL, 0x13, 2, &callback_context); break;
    case 2: wdk_sys_notify_cleanup(NULL, &notify_list, &callback_context); break;
    case 3: wdk_sys_notify_cleanup_all(NULL, &notify_list); break;
    case 4: assert(wdk_sys_oplock_fsctrl(&oplock, &notify_irp, 3, 0x10) == oplock_status); break;
    case 5: assert(wdk_sys_check_oplock(&oplock, &notify_irp, 0x20, &callback_context, wait_complete, prepost) == oplock_status); break;
    case 6: assert(wdk_sys_check_oplock(&oplock, &notify_irp, 0x20, NULL, NULL, NULL) == oplock_status); break;
    case 7: wdk_sys_uninitialize_oplock(&oplock); break;
    default: assert(FALSE);
    }
}
static VOID check_recursion(ULONG operation, PIRP incoming, NTSTATUS exception)
{
    ULONG before = observed;
    IoSetTopLevelIrp(incoming);
    expected_marker = incoming == NULL ? (PIRP)FSRTL_FSP_TOP_LEVEL_IRP : incoming;
    recursion_exception = exception;
    __try { invoke(operation); assert(exception == STATUS_SUCCESS); }
    __except ((NTSTATUS)GetExceptionCode() == exception ? EXCEPTION_EXECUTE_HANDLER : EXCEPTION_CONTINUE_SEARCH) {
        assert(exception != STATUS_SUCCESS);
    }
    assert(observed > before && IoGetTopLevelIrp() == incoming);
}
VOID wdk_test_fsrtl_recursion(VOID)
{
    ULONG operation;
    observed = nesting = 0;
    oplock_status = STATUS_PENDING;
    for (operation = 0; operation != 8; operation++) {
        check_recursion(operation, NULL, STATUS_SUCCESS);
        check_recursion(operation, (PIRP)FSRTL_CACHE_TOP_LEVEL_IRP, STATUS_SUCCESS);
        check_recursion(operation, &notify_irp, STATUS_SUCCESS);
        check_recursion(operation, NULL, STATUS_INSUFFICIENT_RESOURCES);
        check_recursion(operation, &notify_irp, (NTSTATUS)0xE0000001UL);
    }
    oplock_status = STATUS_INSUFFICIENT_RESOURCES;
    check_recursion(4, NULL, STATUS_SUCCESS);
    check_recursion(5, &notify_irp, STATUS_SUCCESS);
    nesting = 1; check_recursion(1, NULL, STATUS_SUCCESS);
    nesting = 1; check_recursion(1, NULL, (NTSTATUS)0xE0000001UL);
    recursion_exception = STATUS_SUCCESS;
    wdk_sys_notify_change_directory(NULL, &notify_list, &callback_context, &name, TRUE,
        FALSE, 0x13, &notify_irp, NULL, NULL);
    IoSetTopLevelIrp(NULL);
}
