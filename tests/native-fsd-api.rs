// Copyright (c) Microsoft Corporation
// License: MIT OR Apache-2.0

//! API and header-macro contracts shared by the WDM and KMDF fixtures.

#[path = "native-fsd-support.rs"]
mod support;

use wdk::file_system::{
    CompletionStatus,
    CriticalRegionGuard,
    DispatchIrp,
    FcbHeader,
    InvalidCompletionStatus,
    InvalidDispatchIrp,
    InvalidPointer,
    ResourceAcquireError,
    ResourceRef,
};
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
    // SAFETY: Index 1 is inside the live stack array.
    let current = unsafe { stack.as_mut_ptr().add(1) };
    set_stack(&mut irp, current);
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
        let expected = (u32::from(success) * SL_INVOKE_ON_SUCCESS)
            | (u32::from(error) * SL_INVOKE_ON_ERROR)
            | (u32::from(cancel) * SL_INVOKE_ON_CANCEL);
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

const _: unsafe fn(PIRP) -> Result<DispatchIrp, InvalidDispatchIrp> = DispatchIrp::from_raw;
const _: unsafe fn(DispatchIrp, CompletionStatus, wdk_sys::ULONG_PTR, CCHAR) -> NTSTATUS =
    DispatchIrp::complete;
const _: fn(&DispatchIrp) -> &wdk_sys::IO_STACK_LOCATION = DispatchIrp::current_stack;
const _: unsafe fn(wdk_sys::PERESOURCE) -> Result<ResourceRef<'static>, InvalidPointer> =
    ResourceRef::from_raw;
const _: unsafe fn(FcbHeader<'static>) = FcbHeader::teardown;

fn set_stack(irp: &mut wdk_sys::IRP, stack: PIO_STACK_LOCATION) {
    // SAFETY: The test initializes the overlay arm of this fixture IRP.
    let overlay = unsafe { &mut irp.Tail.Overlay };
    let location = &mut overlay.__bindgen_anon_2.__bindgen_anon_1;
    location.CurrentStackLocation = stack;
}

#[test]
fn dispatch_completion_preserves_recursion_and_returns_saved_status() {
    use wdk_sys::{IO_STACK_LOCATION, IRP, STATUS_SUCCESS, STATUS_UNSUCCESSFUL};
    let mut previous_irp = IRP::default();
    for previous in [
        core::ptr::null_mut(),
        core::ptr::from_mut(&mut previous_irp),
    ] {
        for raw_status in [STATUS_SUCCESS, STATUS_UNSUCCESSFUL] {
            let mut stack = IO_STACK_LOCATION {
                MajorFunction: 3,
                ..IO_STACK_LOCATION::default()
            };
            let mut irp = IRP {
                StackCount: 1,
                CurrentLocation: 1,
                ..IRP::default()
            };
            set_stack(&mut irp, &mut stack);
            let irp_address = core::ptr::from_mut(&mut irp);
            support::set_top_level(previous);
            // SAFETY: This fixture exclusively owns live IRP and stack storage.
            let dispatch = unsafe { DispatchIrp::from_raw(irp_address) }.unwrap();
            assert_eq!(dispatch.current_stack().MajorFunction, 3);
            let status = CompletionStatus::try_from(raw_status).unwrap();
            // SAFETY: The observer accepts final values at simulated passive
            // IRQL.
            let returned = unsafe { dispatch.complete(status, 4096, 0) };
            assert_eq!(returned, raw_status);
            assert_eq!(support::take_completion(), Some((raw_status, 4096, 0)));
            assert_eq!(
                support::completion_context(),
                if previous.is_null() {
                    irp_address
                } else {
                    previous
                }
            );
            assert_eq!(support::top_level(), previous);
            assert_eq!(support::take_events(), ["complete"]);
        }
    }
    support::set_top_level(core::ptr::null_mut());
}

#[test]
fn dispatch_admission_checks_status_and_pointer_boundaries() {
    use wdk_sys::{IO_STACK_LOCATION, IRP, STATUS_MORE_PROCESSING_REQUIRED, STATUS_PENDING};
    assert_eq!(
        CompletionStatus::try_from(STATUS_PENDING),
        Err(InvalidCompletionStatus::Pending)
    );
    assert_eq!(
        CompletionStatus::try_from(STATUS_MORE_PROCESSING_REQUIRED),
        Err(InvalidCompletionStatus::MoreProcessingRequired)
    );
    // SAFETY: Null is rejected before accessing storage.
    assert!(matches!(
        unsafe { DispatchIrp::from_raw(core::ptr::null_mut()) },
        Err(InvalidDispatchIrp::Pointer(InvalidPointer::Null))
    ));
    let mut irp = IRP::default();
    let misaligned = core::ptr::from_mut(&mut irp)
        .cast::<u8>()
        .wrapping_add(1)
        .cast();
    // SAFETY: Misalignment is rejected before accessing storage.
    assert!(matches!(
        unsafe { DispatchIrp::from_raw(misaligned) },
        Err(InvalidDispatchIrp::Pointer(InvalidPointer::Misaligned))
    ));
    for location in [0, 2] {
        irp.StackCount = 1;
        irp.CurrentLocation = location;
        // SAFETY: The initialized IRP has no admissible current stack.
        assert!(matches!(
            unsafe { DispatchIrp::from_raw(&mut irp) },
            Err(InvalidDispatchIrp::NoCurrentStack)
        ));
    }
    irp.CurrentLocation = 1;
    set_stack(&mut irp, core::ptr::null_mut());
    // SAFETY: Null stack storage is rejected before dereference.
    assert!(matches!(
        unsafe { DispatchIrp::from_raw(&mut irp) },
        Err(InvalidDispatchIrp::StackPointer(InvalidPointer::Null))
    ));
    let mut stack = IO_STACK_LOCATION::default();
    set_stack(
        &mut irp,
        core::ptr::from_mut(&mut stack)
            .cast::<u8>()
            .wrapping_add(1)
            .cast(),
    );
    // SAFETY: Misaligned stack storage is rejected before dereference.
    assert!(matches!(
        unsafe { DispatchIrp::from_raw(&mut irp) },
        Err(InvalidDispatchIrp::StackPointer(InvalidPointer::Misaligned))
    ));
}

#[test]
fn resource_guards_preserve_apc_nesting_and_native_handoff() {
    let mut storage = wdk_sys::ERESOURCE::default();
    // SAFETY: The observer models stationary, initialized resource storage.
    let resource = unsafe { ResourceRef::from_raw(&mut storage) }.unwrap();
    support::grant_access(true);
    // SAFETY: This scope preserves simulated passive IRQL and thread.
    let outer = unsafe { resource.acquire_shared() }.unwrap();
    assert_eq!(support::apc_depth(), 1);
    // SAFETY: Shared recursive acquisition is permitted on this thread.
    let inner = unsafe { resource.try_acquire_shared() }.unwrap();
    assert_eq!(support::apc_depth(), 2);
    let native = inner.into_raw();
    // SAFETY: The native protocol now owns this exact same-thread acquisition.
    unsafe { wdk_sys::file_system::release_resource(native) };
    assert_eq!(support::apc_depth(), 1);
    drop(outer);
    assert_eq!(support::apc_depth(), 0);
    assert_eq!(
        support::take_events(),
        [
            "enter",
            "shared-wait",
            "enter",
            "shared-try",
            "release",
            "leave",
            "release",
            "leave"
        ]
    );
    for exclusive in [false, true] {
        support::grant_access(false);
        // SAFETY: Nonblocking acquisition preserves this thread and passive
        // IRQL.
        let result = if exclusive {
            unsafe { resource.try_acquire_exclusive() }
        } else {
            unsafe { resource.try_acquire_shared() }
        };
        assert!(matches!(result, Err(ResourceAcquireError)));
        assert_eq!(support::apc_depth(), 0);
        assert_eq!(
            support::take_events(),
            [
                "enter",
                if exclusive {
                    "exclusive-try"
                } else {
                    "shared-try"
                },
                "leave"
            ]
        );
    }
    support::grant_access(true);
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        // SAFETY: Rust host unwinding preserves the thread until guard cleanup.
        let _guard = unsafe { resource.acquire_exclusive() }.unwrap();
        panic!("callback unwinds");
    }));
    assert!(panic.is_err());
    assert_eq!(support::apc_depth(), 0);
    assert_eq!(
        support::take_events(),
        ["enter", "exclusive-wait", "release", "leave"]
    );
}

#[test]
fn borrowed_fcb_initializes_and_tears_down_driver_storage() {
    use core::pin::Pin;

    use wdk_sys::{
        FAST_MUTEX,
        FILE_OBJECT,
        FSRTL_ADVANCED_FCB_HEADER,
        FSRTL_COMMON_FCB_HEADER,
        ntddk,
    };
    for supports_file in [false, true] {
        let mut header = FSRTL_ADVANCED_FCB_HEADER::default();
        let address = core::ptr::from_mut(&mut header);
        // SAFETY: The C layout assertions establish this independent fixture's
        // common prefix, which the driver populates before initializing stream
        // support.
        let base = unsafe { &mut *address.cast::<FSRTL_COMMON_FCB_HEADER>() };
        base.NodeTypeCode = 0x1234;
        base.AllocationSize.QuadPart = 8192;
        base.FileSize.QuadPart = 4096;
        base.ValidDataLength.QuadPart = 2048;
        let mut mutex = FAST_MUTEX::default();
        let mutex_address = core::ptr::from_mut(&mut mutex);
        // SAFETY: The production macro initializes the fixture's stationary
        // mutex.
        unsafe { ntddk::ExInitializeFastMutex(&mut mutex) };
        let mut slot = core::ptr::null_mut();
        let slot_address = core::ptr::from_mut(&mut slot);
        let slot_pin = if supports_file {
            Some(Pin::new(&mut slot))
        } else {
            None
        };
        // SAFETY: Driver-owned fixture storage remains stationary until
        // explicit teardown.
        let fcb = unsafe {
            FcbHeader::initialize(
                Pin::new(&mut header),
                Pin::new(&mut mutex),
                slot_pin,
                0x74657354,
            )
        }
        .unwrap();
        assert_eq!(fcb.as_ptr(), address);
        let common = fcb.as_common_ptr();
        assert_eq!(common.cast::<()>(), address.cast::<()>());
        // SAFETY: The initialized header remains live and access is serialized.
        let base = unsafe { &*common };
        assert_eq!(base.NodeTypeCode, 0x1234);
        // SAFETY: The driver initialized the QuadPart arm before stream setup.
        assert_eq!(unsafe { base.AllocationSize.QuadPart }, 8192);
        // SAFETY: The driver initialized the QuadPart arm before stream setup.
        assert_eq!(unsafe { base.FileSize.QuadPart }, 4096);
        // SAFETY: The driver initialized the QuadPart arm before stream setup.
        assert_eq!(unsafe { base.ValidDataLength.QuadPart }, 2048);
        // SAFETY: The view owns live header storage and the test serializes all
        // access.
        let initialized = unsafe { &*fcb.as_ptr() };
        assert_eq!(initialized.FastMutex, mutex_address);
        assert_eq!(
            initialized.FileContextSupportPointer,
            if supports_file {
                slot_address
            } else {
                core::ptr::null_mut()
            }
        );
        let list = core::ptr::addr_of!(initialized.FilterContexts).cast_mut();
        assert_eq!(
            (
                initialized.FilterContexts.Flink,
                initialized.FilterContexts.Blink
            ),
            (list, list)
        );
        let mut file = FILE_OBJECT {
            FsContext: fcb.as_ptr().cast(),
            ..FILE_OBJECT::default()
        };
        // SAFETY: The file object borrows the initialized FCB until teardown.
        assert_ne!(
            unsafe { ntddk::FsRtlSupportsPerStreamContexts(&mut file) },
            0
        );
        // SAFETY: The file object borrows the initialized FCB until teardown.
        assert_eq!(
            unsafe { ntddk::FsRtlSupportsPerFileContexts(&mut file) } != 0,
            supports_file
        );
        file.FsContext = core::ptr::null_mut();
        assert!(file.FsContext.is_null());
        // SAFETY: The file reference was cleared and no context or cache users
        // remain.
        unsafe { fcb.teardown() };
        assert_eq!(support::take_teardown(), address);
        assert_eq!(
            support::take_events(),
            ["allocate-lock", "teardown", "free-lock"]
        );
        assert_eq!(header.FastMutex, mutex_address);
    }
}

#[test]
fn fcb_allocation_failure_publishes_no_header_or_cleanup_obligation() {
    use core::pin::Pin;
    let mut header = wdk_sys::FSRTL_ADVANCED_FCB_HEADER::default();
    let mut mutex = wdk_sys::FAST_MUTEX::default();
    support::allow_lock(false);
    // SAFETY: The empty caller-owned storage is stationary and has no external
    // users.
    let result = unsafe {
        FcbHeader::initialize(
            Pin::new(&mut header),
            Pin::new(&mut mutex),
            None,
            0x74657354,
        )
    };
    assert!(matches!(
        result,
        Err(wdk_sys::STATUS_INSUFFICIENT_RESOURCES)
    ));
    assert_eq!(support::take_events(), ["allocate-lock"]);
    support::allow_lock(true);
}

#[test]
fn pending_forwarding_preserves_the_upper_pending_bit() {
    use wdk_sys::{IO_STACK_LOCATION, IRP, SL_PENDING_RETURNED, STATUS_PENDING};
    for lower_status in [
        wdk_sys::STATUS_SUCCESS,
        STATUS_PENDING,
        wdk_sys::STATUS_UNSUCCESSFUL,
    ] {
        support::lower_status(lower_status);
        for pending in [false, true] {
            let mut stack = [IO_STACK_LOCATION::default(); 3];
            let mut irp = IRP {
                StackCount: 3,
                CurrentLocation: 2,
                ..IRP::default()
            };
            set_stack(&mut irp, core::ptr::from_mut(&mut stack[1]));
            support::mutate_lower_stack(true);
            let status = if pending {
                // SAFETY: The live fixture stack is exclusively owned before
                // publication.
                unsafe { wdk_sys::ntddk::IoMarkIrpPending(&mut irp) };
                // SAFETY: The pended fixture has a next stack and no
                // completion/cancel callback.
                unsafe {
                    wdk_sys::file_system::forward_pending_irp(core::ptr::null_mut(), &mut irp)
                }
            } else {
                // SAFETY: The unpended fixture has a next stack and no
                // completion/cancel callback.
                unsafe {
                    wdk_sys::file_system::forward_unpended_irp(core::ptr::null_mut(), &mut irp)
                }
            };
            assert_eq!(status, lower_status);
            assert_eq!(
                u32::from(stack[1].Control) & SL_PENDING_RETURNED != 0,
                pending
            );
            support::mutate_lower_stack(false);
            assert_eq!(support::take_events(), ["call-driver"]);
        }
    }
    support::lower_status(STATUS_PENDING);
}

// SAFETY: The fixture compiles the native observers with the installed WDK ABI.
#[link(name = "native_fsd_boundary_support", kind = "static")]
unsafe extern "C" {
    fn wdk_test_reset_output();
    fn wdk_test_output_balance() -> u32;
    fn wdk_test_output_faults();
    fn wdk_test_cache_mdl();
    fn wdk_test_fsrtl_recursion();
}

#[test]
fn locked_output_owns_mapping_until_publication_or_drop() {
    use wdk::file_system::LockedRequestorOutput;
    let mut buffer = [0_u8; 32];
    // SAFETY: Each native observer is isolated to the current test thread.
    unsafe { wdk_test_reset_output() };
    // SAFETY: No Rust reference aliases the granted output range while
    // captured.
    let output =
        unsafe { LockedRequestorOutput::capture(buffer.as_mut_ptr().cast(), 32, 1) }.unwrap();
    assert_eq!(output.capacity(), 32);
    // SAFETY: The host fixture models PASSIVE_LEVEL and source is disjoint.
    unsafe { output.copy_from(&[7; 16]) }.unwrap();
    assert_eq!(&buffer[..16], &[7; 16]);
    // SAFETY: Observer checks balanced lock/MDL ownership after publication.
    assert_eq!(unsafe { wdk_test_output_balance() }, 1);

    // SAFETY: Exclusive range ownership is granted until the output is dropped.
    let output =
        unsafe { LockedRequestorOutput::capture(buffer.as_mut_ptr().cast(), 32, 0) }.unwrap();
    drop(output);
    // SAFETY: Observer checks that cancellation teardown performed no copy.
    assert_eq!(unsafe { wdk_test_output_balance() }, 1);

    // SAFETY: Exclusive range ownership is granted through consuming rejection.
    let output =
        unsafe { LockedRequestorOutput::capture(buffer.as_mut_ptr().cast(), 32, 1) }.unwrap();
    // SAFETY: The host fixture models PASSIVE_LEVEL with a disjoint oversized
    // source.
    assert_eq!(
        unsafe { output.copy_from(&[8; 33]) },
        Err(wdk_sys::STATUS_INVALID_PARAMETER)
    );
    // SAFETY: Oversized publication still consumes and releases its output.
    assert_eq!(unsafe { wdk_test_output_balance() }, 1);
    assert_eq!(&buffer[..16], &[7; 16]);
}

#[test]
fn output_probe_and_mapping_failures_release_exactly_the_acquired_resources() {
    // SAFETY: This isolated host oracle contains all raised faults in C SEH
    // frames.
    unsafe { wdk_test_output_faults() };
}

#[test]
fn cache_mdl_partial_acquisitions_and_write_completion_failures_consume_ownership() {
    // SAFETY: Native oracle retains FILE_OBJECT/MDL storage and contains SEH in
    // C.
    unsafe { wdk_test_cache_mdl() };
}

#[test]
fn fsrtl_callbacks_preserve_recursion_context_through_native_unwinding() {
    // SAFETY: Native oracle owns all callback storage and catches SEH before
    // returning.
    unsafe { wdk_test_fsrtl_recursion() };
}
