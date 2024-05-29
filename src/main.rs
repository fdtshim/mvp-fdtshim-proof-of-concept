#![no_main]
#![no_std]

use log::info;
use uefi::prelude::*;
use uefi::{guid, Guid};

pub const EFI_DTB_TABLE_GUID: Guid = guid!("b1b621d5-f19c-41a5-830b-d9152c69aae0");

fn list_configuration_tables(st: &SystemTable<Boot>) {
    st.config_table()
        .iter()
        .for_each(|config| info!(" - {}", config.guid))
}

fn get_efi_dtb_table(st: &SystemTable<Boot>) -> u64 {
    st.config_table()
        .iter()
        .find(|config| config.guid == EFI_DTB_TABLE_GUID)
        .map(|config| config.address as u64)
        .expect("Could not find EFI_DTB_TABLE")
}

#[entry]
fn main(_image_handle: Handle, mut system_table: SystemTable<Boot>) -> Status {
    uefi::helpers::init(&mut system_table).unwrap();
    info!("Configuration tables found:");
    list_configuration_tables(&system_table);
    info!("Looking for DTB table");
    info!("EFI_DTB_TABLE at: 0x{:x}", get_efi_dtb_table(&system_table));
    system_table.boot_services().stall(10_000_000);
    Status::SUCCESS
}
