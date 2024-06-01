use crate::efi::*;
use alloc::vec::Vec;
use flat_device_tree::Fdt;
use log::debug;
use log::info;
use uefi::prelude::*;
use uefi::Result;

pub unsafe fn try_matching<'a>(st: &SystemTable<Boot>, mapping_fdt: &'a Fdt) -> Result<&'a str> {
    debug!("-> Attempting to match device from ambiant data...");

    let ambiant_fdt = fdt::Fdt::from_ptr(get_efi_dtb_table(&st) as *const u8).unwrap();

    let compatible = ambiant_fdt.root().expect("").compatible();
    let compatibles: Vec<&str> = compatible.all().collect();

    let matched_by_fdt = mapping_fdt
        .find_compatible(&compatibles)
        .expect("Compatible not found");

    let dtb_path = matched_by_fdt.property("dtb").unwrap().as_str().unwrap();
    info!("Found a `compatible`-based match:");
    info!("    This device matches DTB path: {}", dtb_path);

    Ok(dtb_path)
}
