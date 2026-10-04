// Copyright (c) Microsoft Corporation
// License: MIT OR Apache-2.0

//! Links retained FSD API references against WDK kernel import libraries.
//! This fixture rejects driver loading and performs no kernel operations.

#![no_std]

extern crate wdk_panic;

use core::hint::black_box;

use wdk::file_system::{DispatchIrp, FcbHeader, ResourceAcquireError, ResourceRef};
use wdk_sys::{NTSTATUS, PCUNICODE_STRING, PDRIVER_OBJECT, STATUS_UNSUCCESSFUL, ntddk};

// SAFETY: This binary provides the unique required driver entry symbol.
#[unsafe(export_name = "DriverEntry")]
pub extern "system" fn driver_entry(_: PDRIVER_OBJECT, _: PCUNICODE_STRING) -> NTSTATUS {
    // Escaping addresses from the entry point keeps the actual production
    // implementations reachable through optimization and linker collection.
    black_box((
        ntddk::IoCallDriver as *const (),
        ntddk::IoCompleteRequest as *const (),
        ntddk::IoGetCurrentIrpStackLocation as *const (),
        ntddk::IoGetNextIrpStackLocation as *const (),
        ntddk::IoSetNextIrpStackLocation as *const (),
        ntddk::IoSkipCurrentIrpStackLocation as *const (),
        ntddk::IoCopyCurrentIrpStackLocationToNext as *const (),
        ntddk::IoSetCompletionRoutine as *const (),
        ntddk::IoSetCancelRoutine as *const (),
        ntddk::IoMarkIrpPending as *const (),
        ntddk::ExInitializeFastMutex as *const (),
        ntddk::CcIsFileCached as *const (),
        ntddk::CcGetFileSizePointer as *const (),
        ntddk::FsRtlInitPerFileContext as *const (),
        ntddk::FsRtlInitPerStreamContext as *const (),
        ntddk::FsRtlGetPerFileContextPointer as *const (),
        ntddk::FsRtlGetPerStreamContextPointer as *const (),
        ntddk::FsRtlSupportsPerFileContexts as *const (),
        ntddk::FsRtlSupportsPerStreamContexts as *const (),
        DispatchIrp::from_raw as *const (),
        DispatchIrp::current_stack as *const (),
        DispatchIrp::complete as *const (),
        FcbHeader::initialize as *const (),
        FcbHeader::as_common_ptr as *const (),
        FcbHeader::teardown as *const (),
        link_resource_scopes as *const (),
    ));
    STATUS_UNSUCCESSFUL
}

unsafe fn link_resource_scopes(resource: &ResourceRef<'_>) -> Result<(), ResourceAcquireError> {
    // SAFETY: This uncalled link anchor requires the shared scoped execution
    // contract.
    unsafe { resource.with_shared(|| ()) }?;
    // SAFETY: This uncalled link anchor requires the exclusive scoped execution
    // contract.
    unsafe { resource.with_exclusive(|| ()) }?;
    // SAFETY: This uncalled link anchor requires the shared scoped execution
    // contract.
    unsafe { resource.try_with_shared(|| ()) }?;
    // SAFETY: This uncalled link anchor requires the exclusive scoped execution
    // contract.
    unsafe { resource.try_with_exclusive(|| ()) }
}
