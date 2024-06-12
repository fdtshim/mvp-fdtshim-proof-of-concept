use crate::efi::*;
use crate::smbios::*;
use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use flat_device_tree::Fdt;
use log::debug;
use log::info;
use log::warn;
use uefi::prelude::*;

struct MatchedDTB<'a> {
    rank: usize,
    dtb_path: &'a str,
}
impl MatchedDTB<'_> {
    pub fn new() -> Self {
        Self {
            rank: usize::MAX,
            dtb_path: "",
        }
    }
}

pub unsafe fn try_matching<'a>(st: &SystemTable<Boot>, mapping_fdt: &'a Fdt) -> Option<&'a str> {
    debug!("-> Attempting to match device from ambiant data...");

    // An ambiant FDT compatible match is always preferred.
    if let Some(fdt) = get_efi_dtb_table(st) {
        let mut matched_dtb = MatchedDTB::new();
        let ambiant_fdt = fdt::Fdt::from_ptr(fdt as *const u8).unwrap();

        let compatible = ambiant_fdt.root().expect("").compatible();
        let ambiant_compatibles: Vec<&str> = compatible.all().collect();

        if let Some(mappings) = mapping_fdt.find_node("/mapping") {
            // For all `/mapping` nodes
            for device in mappings.children() {
                debug!("-- {:?}", device.name);
                // Assuming there's a `compatible` string
                if let Some(dtb_compatible) = device.property("compatible") {
                    // We try to find the rank of a matched compatible
                    if let Some(candidate_rank) =
                        // NOTE: `find_map` returns the value of...
                        dtb_compatible.iter_str().find_map(|dtb_compatible_string| {
                                // ... the `position` in ambiant_compatibles of ...
                                ambiant_compatibles
                                    .iter()
                                    .position(|ambiant_compatible_string| {
                                        // ... the matched string.
                                        *ambiant_compatible_string == dtb_compatible_string
                                    })
                            })
                    {
                        // Is this candidate better ranked?
                        if candidate_rank < matched_dtb.rank {
                            // Save as the match!
                            matched_dtb.rank = candidate_rank;
                            matched_dtb.dtb_path =
                                device.property("dtb").unwrap().as_str().unwrap();
                        }
                        // We can't match anything else, so bail out...
                        if matched_dtb.rank == 0 {
                            break;
                        }
                    }
                } else {
                    warn!("    No compatible property for {:?}?", device.name);
                }
            }
        }

        if matched_dtb.rank != usize::MAX {
            info!("");
            info!("Found a `compatible`-based match:");
            info!("    This device matches DTB path: {}", matched_dtb.dtb_path);
            info!("");

            return Some(matched_dtb.dtb_path);
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
            if let Some(table) = smbios.get_table(1) {
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
        }

        // Type02 data
        dmi.insert("board_vendor", "");
        dmi.insert("board_name", "");
        dmi.insert("board_version", "");
        if let Some(board_information) = smbios.get_board_information() {
            if let Some(table) = smbios.get_table(2) {
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
        }

        // Type03 data
        dmi.insert("chassis_vendor", "");
        dmi.insert("chassis_version", "");
        if let Some(chassis_information) = smbios.get_chassis_information() {
            if let Some(table) = smbios.get_table(3) {
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
        }

        debug!("DMI information to check:\n{:?}", dmi);

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
                        debug!("     MAP: {:?}", field.as_str().unwrap_or("<invalid>"));
                        // Print all values of the prop
                        if let Some(dmi_value) = dmi.get(field.name) {
                            debug!("     DMI: {:?}", dmi_value);
                            if field.iter_str().all(|map_value| {
                                debug!("     MAP: {:?}", map_value);
                                *dmi_value != map_value
                            }) {
                                debug!("       DID NOT MATCH!");
                                valid = false;
                                break;
                            }
                        }
                    }
                    if valid {
                        let dtb_path = device.property("dtb").unwrap().as_str().unwrap();
                        info!("");
                        info!("Found a `dmi-match`-based match:");
                        info!("    This device matches DTB path: {}", dtb_path);
                        info!("");

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
