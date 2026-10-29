// Copyright (c) Microsoft Corporation
// License: MIT OR Apache-2.0

//! ABI observers for calls made by the production header exports.

use std::cell::{Cell, RefCell};

use wdk_sys::{
    CCHAR,
    EVENT_TYPE,
    NTSTATUS,
    PDEVICE_OBJECT,
    PERESOURCE,
    PFSRTL_ADVANCED_FCB_HEADER,
    PIRP,
    PKEVENT,
    PVOID,
    STATUS_PENDING,
    STATUS_SUCCESS,
    ULONG_PTR,
};

thread_local! {
    static EVENTS: RefCell<Vec<&'static str>> = const { RefCell::new(Vec::new()) };
    static COMPLETED: RefCell<Option<(NTSTATUS, ULONG_PTR, CCHAR)>> = const { RefCell::new(None) };
    static GRANT_ACCESS: Cell<bool> = const { Cell::new(true) };
    static LOWER_STATUS: Cell<NTSTATUS> = const { Cell::new(STATUS_PENDING) };
    static MUTATE_LOWER: Cell<bool> = const { Cell::new(false) };
    static APC_DEPTH: Cell<u32> = const { Cell::new(0) };
    static COMPLETION_CONTEXT: Cell<PIRP> = const { Cell::new(core::ptr::null_mut()) };
    static TOP_LEVEL: Cell<PIRP> = const { Cell::new(core::ptr::null_mut()) };
    static LOCK_AVAILABLE: Cell<bool> = const { Cell::new(true) };
    static LOCK: Cell<usize> = const { Cell::new(0) };
    static TORN_DOWN: Cell<PFSRTL_ADVANCED_FCB_HEADER> = const { Cell::new(core::ptr::null_mut()) };
}

pub fn grant_access(grant: bool) {
    GRANT_ACCESS.with(|access| access.set(grant));
}
pub fn take_teardown() -> PFSRTL_ADVANCED_FCB_HEADER {
    TORN_DOWN.with(|header| header.replace(core::ptr::null_mut()))
}

// SAFETY: The test binary supplies the exact generated kernel ABI.
#[unsafe(no_mangle)]
unsafe extern "C" fn ExAcquireResourceSharedLite(_: PERESOURCE, wait: u8) -> u8 {
    assert!(apc_depth() > 0);
    record(if wait == 0 {
        "shared-try"
    } else {
        "shared-wait"
    });
    GRANT_ACCESS.with(|access| u8::from(access.get()))
}
// SAFETY: The test binary supplies the exact generated kernel ABI.
#[unsafe(no_mangle)]
unsafe extern "C" fn ExAcquireResourceExclusiveLite(_: PERESOURCE, wait: u8) -> u8 {
    assert!(apc_depth() > 0);
    record(if wait == 0 {
        "exclusive-try"
    } else {
        "exclusive-wait"
    });
    GRANT_ACCESS.with(|access| u8::from(access.get()))
}
// SAFETY: The test binary supplies the exact generated kernel ABI.
#[unsafe(no_mangle)]
unsafe extern "C" fn ExReleaseResourceLite(_: PERESOURCE) {
    assert!(apc_depth() > 0);
    record("release");
}
// SAFETY: The test binary supplies the exact generated kernel ABI.
#[unsafe(no_mangle)]
unsafe extern "C" fn FsRtlTeardownPerStreamContexts(header: PFSRTL_ADVANCED_FCB_HEADER) {
    TORN_DOWN.with(|torn_down| torn_down.set(header));
    record("teardown");
}

pub fn take_events() -> Vec<&'static str> {
    EVENTS.with(|events| core::mem::take(&mut *events.borrow_mut()))
}

pub fn take_completion() -> Option<(NTSTATUS, ULONG_PTR, CCHAR)> {
    COMPLETED.with(|completed| completed.borrow_mut().take())
}

pub fn record(event: &'static str) {
    EVENTS.with(|events| events.borrow_mut().push(event));
}

unsafe extern "C" fn complete_request(irp: PIRP, boost: CCHAR) {
    COMPLETION_CONTEXT.with(|slot| slot.set(top_level()));
    // SAFETY: The test owns the IRP exclusively until completion.
    let irp = unsafe { &mut *irp };
    // SAFETY: The completion caller populated the status arm.
    let status = unsafe { irp.IoStatus.__bindgen_anon_1.Status };
    COMPLETED.with(|completed| {
        *completed.borrow_mut() = Some((status, irp.IoStatus.Information, boost))
    });
    irp.IoStatus.__bindgen_anon_1.Status = STATUS_PENDING;
    record("complete");
}

unsafe extern "C" fn call_driver(_: PDEVICE_OBJECT, irp: PIRP) -> NTSTATUS {
    if MUTATE_LOWER.with(Cell::get) {
        // SAFETY: The forwarding fixture supplies a valid next stack location.
        let next = unsafe { wdk_sys::ntddk::IoGetNextIrpStackLocation(irp) };
        // SAFETY: The observer models lower-driver changes to its own stack
        // only.
        unsafe {
            (*next).Control = 0;
        };
    }
    record("call-driver");
    LOWER_STATUS.with(Cell::get)
}

unsafe extern "C" fn enter_region() {
    APC_DEPTH.with(|depth| depth.set(depth.get() + 1));
    record("enter");
}
unsafe extern "C" fn leave_region() {
    APC_DEPTH.with(|depth| depth.set(depth.get().checked_sub(1).unwrap()));
    record("leave");
}

// SAFETY: These slots implement the exact WDK import ABIs in this test binary.
#[unsafe(no_mangle)]
static __imp_IofCompleteRequest: unsafe extern "C" fn(PIRP, CCHAR) = complete_request;
// SAFETY: This slot implements the exact WDK import ABI in this test binary.
#[unsafe(no_mangle)]
static __imp_IofCallDriver: unsafe extern "C" fn(PDEVICE_OBJECT, PIRP) -> NTSTATUS = call_driver;
// SAFETY: This slot implements the exact WDK import ABI in this test binary.
#[unsafe(no_mangle)]
static __imp_KeEnterCriticalRegion: unsafe extern "C" fn() = enter_region;
// SAFETY: This slot implements the exact WDK import ABI in this test binary.
#[unsafe(no_mangle)]
static __imp_KeLeaveCriticalRegion: unsafe extern "C" fn() = leave_region;

pub unsafe extern "C" fn completion(_: PDEVICE_OBJECT, _: PIRP, _: PVOID) -> NTSTATUS {
    STATUS_SUCCESS
}

pub unsafe extern "C" fn free_context(_: PVOID) {}

unsafe extern "C" fn initialize_event(event: PKEVENT, kind: EVENT_TYPE, state: u8) {
    // SAFETY: The kernel caller supplies live event storage to initialize.
    let event = unsafe { &mut *event };
    event.Header.SignalState = i32::from(state);
    event.Header.__bindgen_anon_1.__bindgen_anon_2.Type = u8::try_from(kind).unwrap();
}

// SAFETY: This test binary supplies the import slot expected by the WDK header;
// the pointer has exactly the ABI declared for KeInitializeEvent.
#[unsafe(no_mangle)]
static __imp_KeInitializeEvent: unsafe extern "C" fn(PKEVENT, EVENT_TYPE, u8) = initialize_event;

pub fn apc_depth() -> u32 {
    APC_DEPTH.with(Cell::get)
}
pub fn top_level() -> PIRP {
    TOP_LEVEL.with(Cell::get)
}
pub fn set_top_level(value: PIRP) {
    TOP_LEVEL.with(|slot| slot.set(value));
}
pub fn allow_lock(value: bool) {
    LOCK_AVAILABLE.with(|slot| slot.set(value));
}
// SAFETY: This slot implements the generated WDK resource ABI.
#[unsafe(no_mangle)]
static __imp_ExAcquireResourceSharedLite: unsafe extern "C" fn(PERESOURCE, u8) -> u8 =
    ExAcquireResourceSharedLite;
// SAFETY: This slot implements the generated WDK resource ABI.
#[unsafe(no_mangle)]
static __imp_ExAcquireResourceExclusiveLite: unsafe extern "C" fn(PERESOURCE, u8) -> u8 =
    ExAcquireResourceExclusiveLite;
// SAFETY: This slot implements the generated WDK release ABI.
#[unsafe(no_mangle)]
static __imp_ExReleaseResourceLite: unsafe extern "C" fn(PERESOURCE) = ExReleaseResourceLite;
// SAFETY: This slot implements the generated WDK teardown ABI.
#[unsafe(no_mangle)]
static __imp_FsRtlTeardownPerStreamContexts: unsafe extern "C" fn(PFSRTL_ADVANCED_FCB_HEADER) =
    FsRtlTeardownPerStreamContexts;
unsafe extern "C" fn allocate_lock(_: wdk_sys::POOL_TYPE, _: wdk_sys::ULONG) -> PVOID {
    record("allocate-lock");
    if !LOCK_AVAILABLE.with(Cell::get) {
        return core::ptr::null_mut();
    }
    LOCK.with(|slot| slot.as_ptr().cast())
}
unsafe extern "C" fn free_lock(lock: PVOID) {
    LOCK.with(|slot| assert_eq!(lock, slot.as_ptr().cast()));
    record("free-lock");
}
// SAFETY: This slot implements the WDK allocation ABI with stationary test
// storage.
#[unsafe(no_mangle)]
static __imp_FsRtlAllocateAePushLock: unsafe extern "C" fn(
    wdk_sys::POOL_TYPE,
    wdk_sys::ULONG,
) -> PVOID = allocate_lock;
// SAFETY: This slot implements the exact WDK free ABI.
#[unsafe(no_mangle)]
static __imp_FsRtlFreeAePushLock: unsafe extern "C" fn(PVOID) = free_lock;
// SAFETY: This stores only the execution thread's opaque recursion value.
#[unsafe(no_mangle)]
unsafe extern "C" fn IoGetTopLevelIrp() -> PIRP {
    top_level()
}
// SAFETY: This stores only the execution thread's opaque recursion value.
#[unsafe(no_mangle)]
unsafe extern "C" fn IoSetTopLevelIrp(value: PIRP) {
    set_top_level(value);
}

pub fn completion_context() -> PIRP {
    COMPLETION_CONTEXT.with(Cell::get)
}

pub fn mutate_lower_stack(value: bool) {
    MUTATE_LOWER.with(|slot| slot.set(value));
}

pub fn lower_status(status: NTSTATUS) {
    LOWER_STATUS.with(|slot| slot.set(status));
}
