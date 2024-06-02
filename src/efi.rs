//! Helpers for standard EFI features.

use crate::protocols::dt_fixup::DtFixup;
use crate::protocols::dt_fixup::DtFixupFlags;
use core::ffi::c_void;
use log::debug;
use uefi::prelude::*;
use uefi::table::boot::SearchType;
use uefi::Identify;
use uefi::Result;
use uefi::{guid, Guid};

const EFI_DTB_TABLE_GUID: Guid = guid!("b1b621d5-f19c-41a5-830b-d9152c69aae0");

/// Gets the currently installed FDT.
pub fn get_efi_dtb_table(st: &SystemTable<Boot>) -> Option<*const c_void> {
    debug!("-> Getting EFI_DTB_TABLE...");
    st.config_table()
        .iter()
        .find(|config| config.guid == EFI_DTB_TABLE_GUID)
        .map(|config| config.address)
}

/// Installs the given FDT pointer to the configuration tables
pub unsafe fn install_efi_dtb_table(st: &SystemTable<Boot>, fdt: *const c_void) -> Result {
    debug!("-> Installing EFI_DTB_TABLE...");
    let boot_services = st.boot_services();
    boot_services.install_configuration_table(&EFI_DTB_TABLE_GUID, fdt)
}

/// Calls the EFI_DT_FIXUP_PROTOCOL
pub fn efi_dt_fixup(
    st: &SystemTable<Boot>,
    dtb: *const c_void,
    buffer_size: *const usize,
    flags: DtFixupFlags,
) -> Result {
    debug!("-> Calling the EFI_DT_FIXUP_PROTOCOL...");
    let boot_services = st.boot_services();
    let dt_fixup_handle = *boot_services
        .locate_handle_buffer(SearchType::ByProtocol(&DtFixup::GUID))
        .expect("EFI_DT_FIXUP_PROTOCOL is missing")
        .first()
        .unwrap();
    let mut dt_fixup = boot_services
        .open_protocol_exclusive::<DtFixup>(dt_fixup_handle)
        .expect("EFI_DT_FIXUP_PROTOCOL could not be opened");

    dt_fixup.fixup(dtb, buffer_size, flags)
}
