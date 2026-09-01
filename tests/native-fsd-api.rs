// Copyright (c) Microsoft Corporation
// License: MIT OR Apache-2.0

//! Compile-time API contract shared by the WDM and KMDF fixtures.

use wdk::file_system::CriticalRegionGuard;
use wdk_sys::{
    BOOLEAN,
    CCHAR,
    PDRIVER_CANCEL,
    PFAST_MUTEX,
    PFILE_LOCK,
    PFSRTL_ADVANCED_FCB_HEADER,
    PIO_STACK_LOCATION,
    PIRP,
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
