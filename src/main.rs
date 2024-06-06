#![no_main]
#![no_std]

mod efi;
mod matching;
mod protocols;
pub mod smbios;
mod utils;
use crate::efi::*;
use crate::matching::*;
use crate::protocols::dt_fixup::DtFixupFlags;
use crate::utils::*;

extern crate alloc;
extern crate flat_device_tree as fdt;
use alloc::string::ToString;
use core::ffi::c_void;
use log::debug;
use log::error;
use log::info;
use log::warn;
use uefi::prelude::*;
use uefi::table::boot::MemoryType;

// TODO: make this a global; replace with argv[1] when present.
pub const PREFIX: &str = r"\EFI\dtbs";
pub const MAPPING: &str = r"mapping.dtb";

#[entry]
unsafe fn main(_image_handle: Handle, mut system_table: SystemTable<Boot>) -> Status {
    uefi::helpers::init(&mut system_table).unwrap();
    uefi::allocator::init(&mut system_table);

    log::set_max_level(log::LevelFilter::Info);

    let boot_services = system_table.boot_services();

    debug!("");
    debug!("Reading {:?}", path_for(MAPPING).to_string());

    if let Ok(mapping_data) = read_file(boot_services, path_for(MAPPING)) {
        if let Ok(mapping_fdt) = fdt::Fdt::from_ptr(mapping_data.as_ptr()) {
            match try_matching(&system_table, &mapping_fdt) {
                // Found a device tree to apply?
                Some(dtb_path) => {
                    // Load the matched dtb file
                    let dtb = read_file(boot_services, path_for(dtb_path))
                        .expect("Could not load device-specific dtb!!");
                    // Value for the final EFI_DT_TABLE
                    let size = dtb.len();

                    debug!("Determining required buffer size for the final FDT...");
                    // We're using this call to get the appropriate final size of the EFI_DT_TABLE
                    match efi_dt_fixup(
                        &system_table,
                        dtb.as_ptr() as *const c_void,
                        &size,
                        DtFixupFlags::DtApplyFixups,
                    ) {
                        Ok(_) => {}
                        Err(status) => match status.status() {
                            Status::BUFFER_TOO_SMALL => {}
                            _ => {
                                error!("Unexpected error attempting to apply EFI_DT_FIXUP_PROTOCOL! {status}");
                                return Status::ABORTED;
                            }
                        },
                    };
                    debug!("    (Final FDT buffer size: {size})");

                    // Copy the FDT to its final manually allocated location.
                    let final_fdt = boot_services
                        .allocate_pool(MemoryType::ACPI_RECLAIM, size)
                        .expect(
                            "Failed to allocate ACPI_RECLAIM memory ({size} bytes) for final FDT",
                        );
                    final_fdt.copy_from(dtb.as_ptr(), dtb.len());
                    let final_fdt_p = final_fdt as *const c_void;

                    debug!("Applying DT Fixups to new and final FDT...");
                    match efi_dt_fixup(
                        &system_table,
                        final_fdt_p,
                        &size,
                        DtFixupFlags::DtApplyFixups | DtFixupFlags::DtReserveMemory,
                    ) {
                        Ok(_) => {
                            info!("Succesfully applied fixups.")
                        }
                        Err(status) => {
                            error!("Error calling EFI_DT_FIXUP_PROTOCOL ({status})");
                            return Status::ABORTED;
                        }
                    };

                    debug!("Installing new and final FDT...");
                    match install_efi_dtb_table(&system_table, final_fdt_p) {
                        Ok(_) => {
                            info!("Succesfully installed new EFI_DT_TABLE.")
                        }
                        Err(status) => {
                            error!("Error installing new EFI_DT_TABLE ({status})");
                            return Status::ABORTED;
                        }
                    }
                }
                None => {
                    warn!(
                        "No DTB could be matched from ambiant data. (This may not be a problem.)"
                    );
                }
            };

            info!("");
            info!("");
            info!("Final state:");
            if let Some(fdt) = get_efi_dtb_table(&system_table) {
                let ambiant_fdt = fdt::Fdt::from_ptr(fdt as *const u8).unwrap();
                let compatible = ambiant_fdt.root().expect("").compatible().first().unwrap();
                let model = ambiant_fdt.root().expect("").model();
                info!("Ambiant FDT: compatible = {compatible:?};");
                info!("                  model = {model:?};");
            } else {
                info!("No ambiant FDT. (This may not be a problem.)");
            }
            info!("");
            info!("");

            info!("NOTE: fdtshim.efi ran likely successfully to the end.");
        } else {
            // FIXME: const to mapping file path.
            error!("Could not parse {:?}.", path_for(MAPPING).to_string())
        }
    } else {
        // FIXME: const to mapping file path.
        error!("Could not read {:?}.", path_for(MAPPING).to_string())
    }

    // XXX loadImage and exec into /EFI/boot/grub.efi for POC

    info!("Stalling for 10s.");
    boot_services.stall(10_000_000);

    Status::SUCCESS
}
