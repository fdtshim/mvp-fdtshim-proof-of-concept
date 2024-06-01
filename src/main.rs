#![no_main]
#![no_std]

pub mod protocols;
use crate::protocols::dt_fixup::{DtFixup, DtFixupFlags};

extern crate alloc;

use alloc::vec::Vec;

extern crate flat_device_tree as fdt;
use core::ffi::c_void;
use log::info;
use uefi::prelude::*;
use uefi::{guid, Guid};
use uefi::table::boot::{SearchType, MemoryType};
use uefi::{Identify};

use uefi::CString16;
use uefi::fs::{FileSystem, FileSystemResult, PathBuf, Path};
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

unsafe fn dump_fdt_info(st: &SystemTable<Boot>) {
    let addr = get_efi_dtb_table(&st);
    let fdt = fdt::Fdt::from_ptr(addr as *const u8).unwrap();
    let compatible = fdt.root().expect("").compatible();
    info!("This is a devicetree representation of a {}", fdt.root().expect("").model());
    info!("...which is compatible with at least: {}", compatible.first().unwrap());
}


// https://docs.rs/uefi/latest/uefi/fs/index.html#use-str-as-path
fn read_file(bs: &BootServices, path: CString16) -> FileSystemResult<Vec<u8>> {
    info!("read_file({path})");
    let fs: ScopedProtocol<SimpleFileSystem> = bs.get_image_file_system(bs.image_handle()).unwrap();
    let mut fs = FileSystem::new(fs);
    fs.read(Path::new(&path))
}

fn path_for(path: &str) -> CString16 {
    // XXX this would look at the parameter-provided `dtbs` path instead of hardcoded `dtbs`
    let mut p = PathBuf::from(cstr16!(r"\dtbs"));
    p.push(PathBuf::from(CString16::try_from(path).unwrap()));
    p.to_cstr16().into()
}

#[entry]
unsafe fn main(_image_handle: Handle, mut system_table: SystemTable<Boot>) -> Status {
    uefi::helpers::init(&mut system_table).unwrap();
    uefi::allocator::init(&mut system_table);

    let boot_services = system_table.boot_services();

    info!("");
    info!("Reading mapping.dtb");
    let mapping_data = read_file(boot_services, path_for("mapping.dtb")).expect("Could not load mapping.dtb!!");
    info!("mapping.dtb size: {}", mapping_data.len());
    let mapping_fdt = fdt::Fdt::from_ptr(mapping_data.as_ptr()).unwrap();
    let mapping_info = mapping_fdt.find_node("/mapping").expect("No /mapping entry...");
    info!("");
    info!("Data in mapping.dtb:");
    for node in mapping_info.children() {
        info!(" - {}", node.name);
    }

    info!("");
    info!("Configuration tables found:");
    list_configuration_tables(&system_table);
    info!("Looking for DTB table");
    let addr = get_efi_dtb_table(&system_table);
    info!("");
    info!("EFI_DTB_TABLE at: {addr:?}");
    let fdt = fdt::Fdt::from_ptr(addr as *const u8).unwrap();

    dump_fdt_info(&system_table);

    let compatible = fdt.root().expect("").compatible();
    let compatibles: Vec<&str> = compatible.all().collect();

    let matched_by_fdt = mapping_fdt
        .find_compatible(&compatibles)
        .expect("Compatible not found")
        ;
    let dtb_path = matched_by_fdt.property("dtb").unwrap().as_str().unwrap();
    info!("-----------");
    info!("This device matches DTB path: {}", dtb_path);
    info!("-----------");

    let dtb = read_file(boot_services, path_for(dtb_path)).expect("Could not load device-specific dtb!!");

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

    let dtb_p = dtb.as_ptr() as *const c_void;

    let size = dtb.len();
    info!("Loaded dtb binary size: {size}");
    info!("    checking size required for fixups...");
    // NOTE: We're technically applying the fixup here, too, but we only want the `size` value.
    match dt_fixup.fixup(dtb_p, &size, DtFixupFlags::DtApplyFixups) {
        Ok(_) => {},
        Err(status) => {
            match status.status() {
                Status::BUFFER_TOO_SMALL => {}
                _ => { panic!("Error attempting to apply EFI_DT_FIXUP_PROTOCOL! {status}") }
            }
        },
    };
    info!("    => Required buffer size: {size}");

    let final_fdt = boot_services
        .allocate_pool(MemoryType::ACPI_RECLAIM, size)
        .expect("Failed to allocate ACPI_RECLAIM memory ({size} bytes) for final FDT")
    ;
    let final_fdt_p = final_fdt as *const c_void;

    final_fdt.copy_from(dtb.as_ptr(), dtb.len());

    //dtb.iter().map(|byte| {info!("!{byte}");} );

    info!("Applying DT Fixups to new and final FDT");
    match dt_fixup.fixup(final_fdt_p, &size, DtFixupFlags::DtApplyFixups) {
        Ok(_) => {
            info!("Succesfully applied fixups.")
        }
        Err(status) => {
            panic!("Error! {status}")
        }
    };

    boot_services
        .install_configuration_table(&EFI_DTB_TABLE_GUID, final_fdt_p)
        .expect("Failed to install updated EFI_DT_TABLE!")
    ;

    info!("");
    info!("Configuration tables found:");
    list_configuration_tables(&system_table);

    dump_fdt_info(&system_table);

    info!("");
    info!("[this is the end... stalling for 10s]");
    boot_services.stall(10_000_000);
    uefi::allocator::exit_boot_services(); // XXX
    Status::SUCCESS
}
