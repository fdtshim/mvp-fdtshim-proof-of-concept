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
    if let Some(fdt) = get_efi_dtb_table(&st) {
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
    if let Some(smbios) = get_efi_smbios3_table(&st) {
        let smbios = SMBios3::from_ptr(smbios as *const u8).unwrap();

        //
        // First, we collate data to compare against in a map.
        //

        // FIXME: re-check mapping between modalias names and DMI information.
        // TODO: Add other fields as needed.
        let mut dmi: BTreeMap<&str, &str> = BTreeMap::new();
        dmi.insert("svn", "");
        dmi.insert("pn", "");
        dmi.insert("sku", "");
        if let Some(system_information) = smbios.get_system_information() {
            let table = smbios.get_table(1).unwrap();
            dmi.insert(
                "svn",
                table
                    .get_string(system_information.manufacturer)
                    .unwrap_or(""),
            );
            dmi.insert(
                "pn",
                table
                    .get_string(system_information.product_name)
                    .unwrap_or(""),
            );
            dmi.insert(
                "sku",
                table
                    .get_string(system_information.sku_number)
                    .unwrap_or(""),
            );
        }

        // XXX Is this correct?
        //dmi.insert("rvn", "");
        //if let Some(enclosure_information) = smbios.get_enclosure_information() {
        //    let table = smbios.get_table(3).unwrap();
        //    dmi.insert("rvn", table.get_string(enclosure_information.manufacturer).unwrap());
        //}

        //
        // Then, loop on all nodes with `dmi-match`, and if **all** fields of the node match
        // against the collated information, that's our match.
        //

        if let Some(mappings) = mapping_fdt.find_node("/mapping") {
            for device in mappings.children() {
                info!("-- {:?}", device.name);
                if let Some(dmi_match) = device.children().find(|node| node.name == "dmi-match") {
                    let mut valid = true;
                    for field in dmi_match.properties() {
                        info!("---- {:?}", field.name);
                        info!("     {:?}", field.as_str());
                        if let Some(value) = dmi.get(field.name) {
                            info!("     {:?}", value);
                            if *value != field.as_str().unwrap_or("<invalid>") {
                                info!("       DID NOT MATCH!");
                                valid = false;
                                break;
                            }
                        }
                    }
                    if valid {
                        let dtb_path = device.property("dtb").unwrap().as_str().unwrap();
                        info!("Found a `dmi-match`-based match:");
                        info!("    This device matches DTB path: {}", dtb_path);

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
