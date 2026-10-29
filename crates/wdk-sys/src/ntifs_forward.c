// Copyright (c) Microsoft Corporation
// License: MIT OR Apache-2.0

#include "../include/wdk_file_system.h"

NTSTATUS NTAPI wdk_sys_IoCallDriver(PDEVICE_OBJECT DeviceObject, PIRP Irp)
{
    return IoCallDriver(DeviceObject, Irp);
}

NTSTATUS NTAPI wdk_sys_forward_unpended_irp(PDEVICE_OBJECT device, PIRP irp)
{
    IoSkipCurrentIrpStackLocation(irp);
    return IoCallDriver(device, irp);
}

NTSTATUS NTAPI wdk_sys_forward_pending_irp(PDEVICE_OBJECT device, PIRP irp)
{
    IoCopyCurrentIrpStackLocationToNext(irp);
    return IoCallDriver(device, irp);
}
