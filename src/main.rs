#![no_main]
#![no_std]

mod efi;
use crate::efi::*;
mod matching;
use crate::matching::*;
mod utils;
use crate::utils::*;
mod protocols;
use crate::protocols::dt_fixup::{DtFixup, DtFixupFlags};

extern crate alloc;
extern crate flat_device_tree as fdt;
use core::ffi::c_void;
use log::info;
use uefi::prelude::*;
use uefi::table::boot::{MemoryType, SearchType};
use uefi::Identify;

#[entry]
unsafe fn main(_image_handle: Handle, mut system_table: SystemTable<Boot>) -> Status {
    uefi::helpers::init(&mut system_table).unwrap();
    uefi::allocator::init(&mut system_table);

    log::set_max_level(log::LevelFilter::Trace);

    let boot_services = system_table.boot_services();

    info!("");
    info!("Reading mapping.dtb");
    let mapping_data =
        read_file(boot_services, path_for("mapping.dtb")).expect("Could not load mapping.dtb!!");
    info!("mapping.dtb size: {}", mapping_data.len());
    let mapping_fdt = fdt::Fdt::from_ptr(mapping_data.as_ptr()).unwrap();
    let dtb_path = try_matching(&system_table, &mapping_fdt).expect("Could not match device from ambiant data...");

    let dtb =
        read_file(boot_services, path_for(dtb_path)).expect("Could not load device-specific dtb!!");

    let dt_fixup_handle = *boot_services
        .locate_handle_buffer(SearchType::ByProtocol(&DtFixup::GUID))
        .expect("EFI_DT_FIXUP_PROTOCOL is missing")
        .first()
        .unwrap();
    let mut dt_fixup = boot_services
        .open_protocol_exclusive::<DtFixup>(dt_fixup_handle)
        .expect("EFI_DT_FIXUP_PROTOCOL could not be opened");
    info!("Found EFI_DT_FIXUP_PROTOCOL!");

    let dtb_p = dtb.as_ptr() as *const c_void;

    let size = dtb.len();
    info!("Loaded dtb binary size: {size}");
    info!("    checking size required for fixups...");
    // NOTE: We're technically applying the fixup here, too, but we only want the resulting `size` value.
    match dt_fixup.fixup(dtb_p, &size, DtFixupFlags::DtApplyFixups) {
        Ok(_) => {}
        Err(status) => match status.status() {
            Status::BUFFER_TOO_SMALL => {}
            _ => {
                panic!("Error attempting to apply EFI_DT_FIXUP_PROTOCOL! {status}")
            }
        },
    };
    info!("    => Required buffer size: {size}");

    let final_fdt = boot_services
        .allocate_pool(MemoryType::ACPI_RECLAIM, size)
        .expect("Failed to allocate ACPI_RECLAIM memory ({size} bytes) for final FDT");
    let final_fdt_p = final_fdt as *const c_void;

    final_fdt.copy_from(dtb.as_ptr(), dtb.len());

    info!("Applying DT Fixups to new and final FDT");
    match dt_fixup.fixup(
        final_fdt_p,
        &size,
        DtFixupFlags::DtApplyFixups | DtFixupFlags::DtReserveMemory,
    ) {
        Ok(_) => {
            info!("Succesfully applied fixups.")
        }
        Err(status) => {
            panic!("Error! {status}")
        }
    };

    install_efi_dtb_table(&system_table, final_fdt_p)
        .expect("Failed to install updated EFI_DT_TABLE!");

    info!("");
    info!("NOTE: successfully ran to the end.");
    info!("Staling for 10s.");
    boot_services.stall(10_000_000);
    uefi::allocator::exit_boot_services();
    Status::SUCCESS
}
