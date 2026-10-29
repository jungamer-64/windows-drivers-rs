// Copyright (c) Microsoft Corporation
// License: MIT OR Apache-2.0

//! Uses pure WDK stack operations without a kernel runtime or import observers.

use wdk_sys::{IO_STACK_LOCATION, IRP, SL_PENDING_RETURNED, ntddk};

#[test]
fn stack_operations_link_and_run_in_an_ordinary_host_process() {
    let mut stack = [IO_STACK_LOCATION::default(); 2];
    let current = core::ptr::from_mut(&mut stack[1]);
    let mut irp = IRP {
        StackCount: 2,
        CurrentLocation: 2,
        ..IRP::default()
    };
    // SAFETY: The fixture initializes the overlay for its live stack storage.
    let overlay = unsafe { &mut irp.Tail.Overlay };
    overlay
        .__bindgen_anon_2
        .__bindgen_anon_1
        .CurrentStackLocation = current;
    // SAFETY: This fixture owns a valid current stack for the entire call.
    assert_eq!(
        unsafe { ntddk::IoGetCurrentIrpStackLocation(&mut irp) },
        current
    );
    // SAFETY: The unqueued fixture exclusively owns its pending transition.
    unsafe { ntddk::IoMarkIrpPending(&mut irp) };
    assert_ne!(u32::from(stack[1].Control) & SL_PENDING_RETURNED, 0);
}
