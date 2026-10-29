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
        wdk_sys::file_system::forward_pending_irp as *const (),
        wdk_sys::file_system::forward_unpended_irp as *const (),
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
        wdk::file_system::LockedRequestorOutput::capture as *const (),
        wdk::file_system::LockedRequestorOutput::copy_from as *const (),
        wdk_sys::file_system::cc_read_mdl as *const (),
        wdk_sys::file_system::cc_prepare_mdl_write as *const (),
        wdk_sys::file_system::cc_complete_mdl_write as *const (),
        wdk_sys::file_system::notify_change_directory as *const (),
        wdk_sys::file_system::notify_report_change as *const (),
        wdk_sys::file_system::notify_cleanup as *const (),
        wdk_sys::file_system::notify_cleanup_all as *const (),
        wdk_sys::file_system::oplock_fsctrl as *const (),
        wdk_sys::file_system::check_oplock as *const (),
        wdk_sys::file_system::uninitialize_oplock as *const (),
    ));
    STATUS_UNSUCCESSFUL
}

unsafe fn link_resource_scopes(resource: &ResourceRef<'_>) -> Result<(), ResourceAcquireError> {
    // SAFETY: This link anchor requires same-thread shared acquisition.
    drop(unsafe { resource.acquire_shared() }?);
    // SAFETY: This link anchor requires exclusive acquisition.
    drop(unsafe { resource.acquire_exclusive() }?);
    // SAFETY: This link anchor requires nonblocking shared acquisition.
    drop(unsafe { resource.try_acquire_shared() }?);
    // SAFETY: This link anchor requires nonblocking exclusive acquisition.
    drop(unsafe { resource.try_acquire_exclusive() }?);
    Ok(())
}
