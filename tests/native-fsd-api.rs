// Copyright (c) Microsoft Corporation
// License: MIT OR Apache-2.0

//! API and header-macro contracts shared by the WDM and KMDF fixtures.

#[path = "native-fsd-support.rs"]
mod support;

use wdk::file_system::CriticalRegionGuard;
use wdk_sys::{
    BOOLEAN,
    CCHAR,
    NTSTATUS,
    PDEVICE_OBJECT,
    PDRIVER_CANCEL,
    PFAST_MUTEX,
    PFILE_LOCK,
    PFILE_OBJECT,
    PFREE_FUNCTION,
    PFSRTL_ADVANCED_FCB_HEADER,
    PFSRTL_PER_FILE_CONTEXT,
    PFSRTL_PER_STREAM_CONTEXT,
    PIO_COMPLETION_ROUTINE,
    PIO_STACK_LOCATION,
    PIRP,
    PLARGE_INTEGER,
    PVOID,
};

const _: unsafe extern "C" fn(PIRP, CCHAR) = wdk_sys::ntddk::IoCompleteRequest;
const _: unsafe extern "C" fn(PIRP) -> PIO_STACK_LOCATION =
    wdk_sys::ntddk::IoGetCurrentIrpStackLocation;
const _: unsafe extern "C" fn(PIRP) -> PIO_STACK_LOCATION =
    wdk_sys::ntddk::IoGetNextIrpStackLocation;
const _: unsafe extern "C" fn(PIRP) = wdk_sys::ntddk::IoMarkIrpPending;
const _: unsafe extern "C" fn(PIRP, PDRIVER_CANCEL) -> PDRIVER_CANCEL =
    wdk_sys::ntddk::IoSetCancelRoutine;
const _: unsafe extern "C" fn() = wdk_sys::ntddk::FsRtlEnterFileSystem;
const _: unsafe extern "C" fn() = wdk_sys::ntddk::FsRtlExitFileSystem;
const _: unsafe extern "C" fn(PFSRTL_ADVANCED_FCB_HEADER, PFAST_MUTEX, *mut PVOID, PVOID) =
    wdk_sys::ntddk::FsRtlSetupAdvancedHeaderEx2;
const _: unsafe extern "C" fn(PFILE_LOCK) -> BOOLEAN =
    wdk_sys::ntddk::FsRtlAreThereCurrentFileLocks;
const _: unsafe fn() -> CriticalRegionGuard = CriticalRegionGuard::enter;
const _: unsafe extern "C" fn(PDEVICE_OBJECT, PIRP) -> NTSTATUS = wdk_sys::ntddk::IoCallDriver;
const _: unsafe extern "C" fn(PIRP) = wdk_sys::ntddk::IoSetNextIrpStackLocation;
const _: unsafe extern "C" fn(PIRP) = wdk_sys::ntddk::IoSkipCurrentIrpStackLocation;
const _: unsafe extern "C" fn(PIRP) = wdk_sys::ntddk::IoCopyCurrentIrpStackLocationToNext;
const _: unsafe extern "C" fn(PIRP, PIO_COMPLETION_ROUTINE, PVOID, BOOLEAN, BOOLEAN, BOOLEAN) =
    wdk_sys::ntddk::IoSetCompletionRoutine;
const _: unsafe extern "C" fn(PFAST_MUTEX) = wdk_sys::ntddk::ExInitializeFastMutex;
const _: unsafe extern "C" fn(PFILE_OBJECT) -> BOOLEAN = wdk_sys::ntddk::CcIsFileCached;
const _: unsafe extern "C" fn(PFILE_OBJECT) -> PLARGE_INTEGER =
    wdk_sys::ntddk::CcGetFileSizePointer;
const _: unsafe extern "C" fn(PFSRTL_PER_FILE_CONTEXT, PVOID, PVOID, PFREE_FUNCTION) =
    wdk_sys::ntddk::FsRtlInitPerFileContext;
const _: unsafe extern "C" fn(PFSRTL_PER_STREAM_CONTEXT, PVOID, PVOID, PFREE_FUNCTION) =
    wdk_sys::ntddk::FsRtlInitPerStreamContext;
const _: unsafe extern "C" fn(PFILE_OBJECT) -> *mut PVOID =
    wdk_sys::ntddk::FsRtlGetPerFileContextPointer;
const _: unsafe extern "C" fn(PFILE_OBJECT) -> PFSRTL_ADVANCED_FCB_HEADER =
    wdk_sys::ntddk::FsRtlGetPerStreamContextPointer;
const _: unsafe extern "C" fn(PFILE_OBJECT) -> BOOLEAN =
    wdk_sys::ntddk::FsRtlSupportsPerFileContexts;
const _: unsafe extern "C" fn(PFILE_OBJECT) -> BOOLEAN =
    wdk_sys::ntddk::FsRtlSupportsPerStreamContexts;

#[test]
fn header_exports_call_the_kernel_abi() {
    use wdk_sys::{IRP, STATUS_PENDING, STATUS_SUCCESS, ntddk};
    assert!(support::take_events().is_empty());
    // SAFETY: The observer implements the thread-local critical-region ABI.
    unsafe { ntddk::FsRtlEnterFileSystem() };
    // SAFETY: This pairs the preceding entry on this execution thread.
    unsafe { ntddk::FsRtlExitFileSystem() };
    let mut irp = IRP::default();
    // SAFETY: The observer does not dereference the device or IRP.
    assert_eq!(
        unsafe { ntddk::IoCallDriver(core::ptr::null_mut(), &mut irp) },
        STATUS_PENDING
    );
    irp.IoStatus.__bindgen_anon_1.Status = STATUS_SUCCESS;
    irp.IoStatus.Information = 32;
    // SAFETY: This initialized, exclusively owned fixture IRP can be completed.
    unsafe { ntddk::IoCompleteRequest(&mut irp, 1) };
    assert_eq!(support::take_completion(), Some((STATUS_SUCCESS, 32, 1)));
    assert_eq!(
        support::take_events(),
        ["enter", "leave", "call-driver", "complete"]
    );
}

#[test]
fn stack_copy_and_completion_registration() {
    use wdk_sys::{
        IO_STACK_LOCATION,
        IRP,
        SL_INVOKE_ON_CANCEL,
        SL_INVOKE_ON_ERROR,
        SL_INVOKE_ON_SUCCESS,
        ntddk,
    };
    let mut stack = [IO_STACK_LOCATION::default(); 3];
    stack[1].MajorFunction = 3;
    stack[1].MinorFunction = 7;
    stack[1].Flags = 5;
    stack[1].Control = 0xFF;
    let mut irp = IRP {
        StackCount: 3,
        CurrentLocation: 2,
        ..IRP::default()
    };
    // SAFETY: The initialized overlay describes the live stack array.
    unsafe {
        irp.Tail
            .Overlay
            .__bindgen_anon_2
            .__bindgen_anon_1
            .CurrentStackLocation = stack.as_mut_ptr().add(1)
    };
    // SAFETY: Both current and next stack locations are valid.
    unsafe { ntddk::IoCopyCurrentIrpStackLocationToNext(&mut irp) };
    assert_eq!(
        (
            stack[0].MajorFunction,
            stack[0].MinorFunction,
            stack[0].Flags,
            stack[0].Control
        ),
        (3, 7, 5, 0)
    );
    let mut context = 0_u8;
    let context = core::ptr::from_mut(&mut context).cast();
    for (success, error, cancel) in [(1, 0, 0), (0, 1, 0), (0, 0, 1), (1, 1, 1), (0, 0, 0)] {
        // SAFETY: Callback and context are live, and the IRP has a next
        // location.
        unsafe {
            ntddk::IoSetCompletionRoutine(
                &mut irp,
                Some(support::completion),
                context,
                success,
                error,
                cancel,
            )
        };
        assert_eq!(stack[0].Context, context);
        assert!(core::ptr::fn_addr_eq(
            stack[0].CompletionRoutine.unwrap(),
            support::completion as unsafe extern "C" fn(_, _, _) -> _
        ));
        let expected = u32::from(success) * SL_INVOKE_ON_SUCCESS
            | u32::from(error) * SL_INVOKE_ON_ERROR
            | u32::from(cancel) * SL_INVOKE_ON_CANCEL;
        assert_eq!(u32::from(stack[0].Control), expected);
    }
    // SAFETY: This unpended IRP has room for the skip operation.
    unsafe { ntddk::IoSkipCurrentIrpStackLocation(&mut irp) };
    assert_eq!(irp.CurrentLocation, 3);
    // SAFETY: The current stack pointer still denotes the live array.
    assert_eq!(
        unsafe { ntddk::IoGetCurrentIrpStackLocation(&mut irp) },
        core::ptr::from_mut(&mut stack[2])
    );
    // SAFETY: The IRP can advance back to its preceding location.
    unsafe { ntddk::IoSetNextIrpStackLocation(&mut irp) };
    assert_eq!(irp.CurrentLocation, 2);
    // SAFETY: This IRP has a valid current stack location.
    unsafe { ntddk::IoMarkIrpPending(&mut irp) };
    assert_ne!(
        stack[1].Control & u8::try_from(wdk_sys::SL_PENDING_RETURNED).unwrap(),
        0
    );
}

#[test]
fn cache_macros_follow_the_section_object_pointer() {
    use wdk_sys::{FILE_OBJECT, LARGE_INTEGER, SECTION_OBJECT_POINTERS, ntddk};
    let mut file = FILE_OBJECT::default();
    // SAFETY: The file object is initialized and its section pointer is null.
    assert_eq!(unsafe { ntddk::CcIsFileCached(&mut file) }, 0);
    let mut sections = SECTION_OBJECT_POINTERS::default();
    file.SectionObjectPointer = &mut sections;
    // SAFETY: The live section object has no shared cache map.
    assert_eq!(unsafe { ntddk::CcIsFileCached(&mut file) }, 0);
    let mut map = [LARGE_INTEGER::default(); 2];
    map[1].QuadPart = 8192;
    // SAFETY: The section object is live and exclusively used by this test.
    unsafe { (*file.SectionObjectPointer).SharedCacheMap = map.as_mut_ptr().cast() };
    // SAFETY: The file and section objects remain live.
    assert_ne!(unsafe { ntddk::CcIsFileCached(&mut file) }, 0);
    // SAFETY: This fixture supplies the WDK-defined file-size slot after the
    // first LARGE_INTEGER.
    let size = unsafe { ntddk::CcGetFileSizePointer(&mut file) };
    assert_eq!(size, core::ptr::from_mut(&mut map[1]));
}

#[test]
fn context_macros_initialize_identity_and_report_support() {
    use wdk_sys::{
        FAST_MUTEX,
        FILE_OBJECT,
        FSRTL_ADVANCED_FCB_HEADER,
        FSRTL_PER_FILE_CONTEXT,
        FSRTL_PER_STREAM_CONTEXT,
        ntddk,
    };
    let mut file = FILE_OBJECT::default();
    // SAFETY: The initialized file has no FCB.
    assert_eq!(unsafe { ntddk::FsRtlSupportsPerFileContexts(&mut file) }, 0);
    // SAFETY: The initialized file has no FCB.
    assert_eq!(
        unsafe { ntddk::FsRtlSupportsPerStreamContexts(&mut file) },
        0
    );
    let mut header = FSRTL_ADVANCED_FCB_HEADER::default();
    let mut mutex = FAST_MUTEX::default();
    // SAFETY: The ABI observer initializes the event without accessing kernel
    // memory.
    unsafe { ntddk::ExInitializeFastMutex(&mut mutex) };
    assert_eq!(mutex.Count, 1);
    let mut slot = core::ptr::null_mut();
    // SAFETY: All fixture storage is initialized and stationary until the test
    // ends.
    unsafe {
        ntddk::FsRtlSetupAdvancedHeaderEx2(
            &mut header,
            &mut mutex,
            &mut slot,
            core::ptr::null_mut(),
        )
    };
    file.FsContext = core::ptr::from_mut(&mut header).cast();
    // SAFETY: The file points to the initialized header.
    assert_ne!(unsafe { ntddk::FsRtlSupportsPerFileContexts(&mut file) }, 0);
    // SAFETY: The file points to the initialized header.
    assert_ne!(
        unsafe { ntddk::FsRtlSupportsPerStreamContexts(&mut file) },
        0
    );
    // SAFETY: The file points to the live header and slot.
    assert_eq!(
        unsafe { ntddk::FsRtlGetPerFileContextPointer(&mut file) },
        core::ptr::from_mut(&mut slot)
    );
    // SAFETY: The file points to the live header.
    assert_eq!(
        unsafe { ntddk::FsRtlGetPerStreamContextPointer(&mut file) },
        core::ptr::from_mut(&mut header)
    );
    let owner = file.FsContext;
    let instance = core::ptr::from_mut(&mut mutex).cast();
    let mut per_file = FSRTL_PER_FILE_CONTEXT::default();
    let mut per_stream = FSRTL_PER_STREAM_CONTEXT::default();
    // SAFETY: The contexts are unlinked and the callback stays live.
    unsafe {
        ntddk::FsRtlInitPerFileContext(&mut per_file, owner, instance, Some(support::free_context))
    };
    // SAFETY: The contexts are unlinked and the callback stays live.
    unsafe {
        ntddk::FsRtlInitPerStreamContext(
            &mut per_stream,
            owner,
            instance,
            Some(support::free_context),
        )
    };
    assert_eq!((per_file.OwnerId, per_file.InstanceId), (owner, instance));
    assert_eq!(
        (per_stream.OwnerId, per_stream.InstanceId),
        (owner, instance)
    );
    assert!(per_file.FreeCallback.is_some());
    assert!(per_stream.FreeCallback.is_some());
    // SAFETY: The live header remains exclusively owned by this test.
    unsafe {
        (*file.FsContext.cast::<FSRTL_ADVANCED_FCB_HEADER>()).FileContextSupportPointer =
            core::ptr::null_mut()
    };
    // SAFETY: The file still points to the live header.
    assert_eq!(
        unsafe { ntddk::FsRtlGetPerFileContextPointer(&mut file) },
        core::ptr::null_mut()
    );
}

trait AmbiguousIfSend<Marker> {
    fn marker() {}
}

impl<T: ?Sized> AmbiguousIfSend<()> for T {}

struct ImplementsSend;

impl<T: ?Sized + Send> AmbiguousIfSend<ImplementsSend> for T {}

const _: fn() = <CriticalRegionGuard as AmbiguousIfSend<_>>::marker;

trait AmbiguousIfSync<Marker> {
    fn marker() {}
}

impl<T: ?Sized> AmbiguousIfSync<()> for T {}

struct ImplementsSync;

impl<T: ?Sized + Sync> AmbiguousIfSync<ImplementsSync> for T {}

const _: fn() = <CriticalRegionGuard as AmbiguousIfSync<_>>::marker;
