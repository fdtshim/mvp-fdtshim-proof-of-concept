#![no_main]
#![no_std]

pub mod protocols;
use crate::protocols::dt_fixup::{DtFixup, DtFixupFlags};

extern crate alloc;

use alloc::vec;
use alloc::vec::Vec;

extern crate flat_device_tree as fdt;
use core::ffi::c_void;
use log::info;
use uefi::prelude::*;
use uefi::{guid, Guid};
use uefi::table::boot::SearchType;
use uefi::{Identify, Result};

use uefi::CString16;
use uefi::fs::{FileSystem, FileSystemResult};
use uefi::proto::media::fs::SimpleFileSystem;
use uefi::table::boot::ScopedProtocol;


pub const EFI_DTB_TABLE_GUID: Guid = guid!("b1b621d5-f19c-41a5-830b-d9152c69aae0");

fn list_configuration_tables(st: &SystemTable<Boot>) {
    st.config_table()
        .iter()
        .for_each(|config| info!(" - {}", config.guid))
}

fn get_efi_dtb_table(st: &SystemTable<Boot>) -> *const c_void {
    st.config_table()
        .iter()
        .find(|config| config.guid == EFI_DTB_TABLE_GUID)
        .map(|config| config.address)
        .expect("Could not find EFI_DTB_TABLE")
}



// https://docs.rs/uefi/latest/uefi/fs/index.html#use-str-as-path
fn read_file(bs: &BootServices, path: &str) -> FileSystemResult<Vec<u8>> {
    let path: CString16 = CString16::try_from(path).unwrap();
    let fs: ScopedProtocol<SimpleFileSystem> = bs.get_image_file_system(bs.image_handle()).unwrap();
    let mut fs = FileSystem::new(fs);
    fs.read(path.as_ref())
}

#[entry]
unsafe fn main(_image_handle: Handle, mut system_table: SystemTable<Boot>) -> Status {
    uefi::helpers::init(&mut system_table).unwrap();
    uefi::allocator::init(&mut system_table);

    let boot_services = system_table.boot_services();

    info!("");
    info!("Reading mapping.dtb");
    let mapping_data = read_file(boot_services, r"dtbs\mapping.dtb").expect("Could not load mapping.dtb!!");
    info!("mapping.dtb size: {}", mapping_data.len());
    let mapping_fdt = fdt::Fdt::from_ptr(mapping_data.as_ptr()).unwrap();
    info!("");
    //info!("{mapping_fdt:?}");
    let mapping_info = mapping_fdt.find_node("/mapping").expect("No /mapping entry...");
    info!("");
    info!("Data in mapping.dtb:");
    for node in mapping_info.children() {
        info!(" - {}", node.name);
    }
    let matched_by_fdt = mapping_fdt
        .find_compatible(&["linux,dummy-virt"]) // XXX
        .expect("dummy-virt not found")
        ;
    info!("");
    info!("Found entry: {}", matched_by_fdt.property("dtb").unwrap().as_str().unwrap());
    info!("-----------");

    info!("");
    info!("Configuration tables found:");
    list_configuration_tables(&system_table);
    info!("Looking for DTB table");
    let addr = get_efi_dtb_table(&system_table);
    info!("EFI_DTB_TABLE at: {addr:?}");
    let fdt = fdt::Fdt::from_ptr(addr as *const u8).unwrap();

    //  info!("");
    //  info!("{fdt:?}");
    //  info!("");

    info!("This is a devicetree representation of a {}", fdt.root().expect("").model());
    info!("...which is compatible with at least: {}", fdt.root().expect("").compatible().first().expect(""));
    info!("...and has {} CPU(s)", fdt.cpus().count());
    info!(
        "...and has at least one memory location at: {:#X}\n",
        fdt.memory().expect("").regions().next().unwrap().starting_address as usize
    );

    let dt_fixup_handle = *boot_services
        .locate_handle_buffer(SearchType::ByProtocol(&DtFixup::GUID))
        .expect("EFI_DT_FIXUP_PROTOCOL is missing")
        .first()
        .unwrap()
        ;
    let mut dt_fixup = boot_services
        .open_protocol_exclusive::<DtFixup>(
            dt_fixup_handle
        )
        .expect("EFI_DT_FIXUP_PROTOCOL could not be opened")
        ;
    info!("Found EFI_DT_FIXUP_PROTOCOL!");
    let x = 1; // XXX should be the read size...
    // XXX should not use the innate FDT `addr`!!!!
    let result =
        match dt_fixup.fixup(addr, &x, DtFixupFlags::DtApplyFixups) {
            Ok(result) => result,
            Err(status) => {
                match status.status() {
                    Status::BUFFER_TOO_SMALL => {
                        // XXX -> should allocate a new buffer, copy, get rid of the old one.
                        info!("Required buffer size: {x}");
                        info!("re-trying!");
                        dt_fixup.fixup(addr, &x, DtFixupFlags::DtApplyFixups)
                    }
                    _ => { panic!("Error! {status}") }
                }
            }.unwrap(),
        }
        ;


    info!("Success???");
    boot_services.stall(10_000_000);
    uefi::allocator::exit_boot_services(); // XXX
    Status::SUCCESS
}
