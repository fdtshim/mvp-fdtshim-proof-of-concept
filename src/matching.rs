use crate::efi::*;
use crate::smbios::*;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use flat_device_tree::Fdt;
use log::debug;
use log::info;
use log::warn;
use uefi::prelude::*;

pub unsafe fn try_matching<'a>(st: &SystemTable<Boot>, mapping_fdt: &'a Fdt) -> Option<&'a str> {
    debug!("-> Attempting to match device from ambiant data...");

    // An ambiant FDT compatible match is always preferred.
    if let Some(fdt) = get_efi_dtb_table(st) {
        let ambiant_fdt = fdt::Fdt::from_ptr(fdt as *const u8).unwrap();

        let compatible = ambiant_fdt.root().expect("").compatible();
        let compatibles: Vec<&str> = compatible.all().collect();

        match mapping_fdt.find_compatible(&compatibles) {
            Some(matched_by_fdt) => {
                let dtb_path = matched_by_fdt.property("dtb").unwrap().as_str().unwrap();
                info!("Found a `compatible`-based match:");
                info!("    This device matches DTB path: {}", dtb_path);

                return Some(dtb_path);
            }
            None => { /* Fall through */ }
        }
    }

    // Falling back to DMI data
    if let Some(smbios) = get_efi_smbios3_table(st) {
        let smbios = SMBios3::from_ptr(smbios as *const u8).unwrap();

        //
        // First, we collate data to compare against in a map.
        //

        let mut dmi: BTreeMap<&str, &str> = BTreeMap::new();

        // Type01 data
        dmi.insert("sys_vendor", "");
        dmi.insert("product_name", "");
        dmi.insert("product_version", "");
        dmi.insert("product_sku", "");
        dmi.insert("product_family", "");
        if let Some(system_information) = smbios.get_system_information() {
            let table = smbios.get_table(1).unwrap();
            dmi.insert(
                "sys_vendor",
                table
                    .get_string(system_information.sys_vendor)
                    .unwrap_or(""),
            );
            dmi.insert(
                "product_name",
                table
                    .get_string(system_information.product_name)
                    .unwrap_or(""),
            );
            dmi.insert(
                "product_version",
                table
                    .get_string(system_information.product_version)
                    .unwrap_or(""),
            );
            dmi.insert(
                "product_sku",
                table
                    .get_string(system_information.product_sku)
                    .unwrap_or(""),
            );
            dmi.insert(
                "product_family",
                table
                    .get_string(system_information.product_family)
                    .unwrap_or(""),
            );
        }

        // Type02 data
        dmi.insert("board_vendor", "");
        dmi.insert("board_name", "");
        dmi.insert("board_version", "");
        if let Some(board_information) = smbios.get_board_information() {
            let table = smbios.get_table(2).unwrap();
            dmi.insert(
                "board_vendor",
                table
                    .get_string(board_information.board_vendor)
                    .unwrap_or(""),
            );
            dmi.insert(
                "board_name",
                table.get_string(board_information.board_name).unwrap_or(""),
            );
            dmi.insert(
                "board_version",
                table
                    .get_string(board_information.board_version)
                    .unwrap_or(""),
            );
        }

        // Type03 data
        dmi.insert("chassis_vendor", "");
        dmi.insert("chassis_version", "");
        if let Some(chassis_information) = smbios.get_chassis_information() {
            let table = smbios.get_table(3).unwrap();
            dmi.insert(
                "chassis_vendor",
                table
                    .get_string(chassis_information.chassis_vendor)
                    .unwrap_or(""),
            );
            dmi.insert(
                "chassis_version",
                table
                    .get_string(chassis_information.chassis_version)
                    .unwrap_or(""),
            );
        }

        //
        // Then, loop on all nodes with `dmi-match`, and if **all** fields of the node match
        // against the collated information, that's our match.
        //

        if let Some(mappings) = mapping_fdt.find_node("/mapping") {
            for device in mappings.children() {
                debug!("-- {:?}", device.name);
                if let Some(dmi_match) = device.children().find(|node| node.name == "dmi-match") {
                    let mut valid = true;
                    for field in dmi_match.properties() {
                        debug!("---- {:?}", field.name);
                        debug!("     {:?}", field.as_str());
                        if let Some(value) = dmi.get(field.name) {
                            debug!("     {:?}", value);
                            if *value != field.as_str().unwrap_or("<invalid>") {
                                debug!("       DID NOT MATCH!");
                                valid = false;
                                break;
                            }
                        }
                    }
                    if valid {
                        let dtb_path = device.property("dtb").unwrap().as_str().unwrap();
                        debug!("Found a `dmi-match`-based match:");
                        debug!("    This device matches DTB path: {}", dtb_path);

                        return Some(dtb_path);
                    }
                }
            }
            None
        } else {
            warn!("No `/mapping` node in mapping dtb. (This might be a problem...)");
            None
        }
    } else {
        // Nothing could be matched... oh well...
        None
    }
}
