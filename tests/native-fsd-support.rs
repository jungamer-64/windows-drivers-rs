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

unsafe extern "C" fn call_driver(_: PDEVICE_OBJECT, _: PIRP) -> NTSTATUS {
    record("call-driver");
    STATUS_PENDING
}

unsafe extern "C" fn enter_region() {
    record("enter");
}
unsafe extern "C" fn leave_region() {
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
