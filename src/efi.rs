//! Helpers for standard EFI features.

use core::ffi::c_void;
use log::debug;
use uefi::prelude::*;
use uefi::Result;
use uefi::{guid, Guid};

const EFI_DTB_TABLE_GUID: Guid = guid!("b1b621d5-f19c-41a5-830b-d9152c69aae0");

pub fn get_efi_dtb_table(st: &SystemTable<Boot>) -> *const c_void {
    debug!("-> Getting EFI_DTB_TABLE...");
    st.config_table()
        .iter()
        .find(|config| config.guid == EFI_DTB_TABLE_GUID)
        .map(|config| config.address)
        .expect("Could not find EFI_DTB_TABLE")
}

pub unsafe fn install_efi_dtb_table(st: &SystemTable<Boot>, fdt: *const c_void) -> Result {
    debug!("-> Installing EFI_DTB_TABLE...");
    let boot_services = st.boot_services();
    boot_services.install_configuration_table(&EFI_DTB_TABLE_GUID, fdt)
}
