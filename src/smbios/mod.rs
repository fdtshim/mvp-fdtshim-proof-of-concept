use alloc::collections::BTreeMap;
use alloc::vec::Vec;
use core::ffi::c_char;
use uefi::Error;
use uefi::Result;
use uefi::Status;
use zero::{read, read_str, Pod};

/// The type of the actual structure the header is from.
type SMBiosTableType = u8;
/// An indice (1-based) for a string in the table's strings.
type SMBiosTableStringRef = u8;

pub struct SMBios3<'a> {
    data: &'a [u8],
    tables: BTreeMap<SMBiosTableType, SMBiosTable<'a>>,
    pub entry_point: &'a SMBios3EntryPoint,
}

unsafe fn points_to_end(ptr: *const c_char) -> bool {
    (*(ptr as *const u8)) == 0
}

#[repr(C)]
pub struct SMBios3EntryPoint {
    /// "_SM3_"; not NUL-terminated
    pub anchor: [c_char; 5],
    pub checksum: u8,
    pub length: u8,
    pub major_ver: u8,
    pub minor_ver: u8,
    pub doc_rev: u8,
    pub entry_point_rev: u8,
    /// Must be 0
    pub reserved: u8,
    pub table_maximum_size: u32,
    pub struct_table_address: *const u8,
}
unsafe impl Pod for SMBios3EntryPoint {}

#[repr(C, packed(1))]
pub struct SMBiosTableHeader {
    r#type: SMBiosTableType,
    length: u8,
    handle: u16,
}
unsafe impl Pod for SMBiosTableHeader {}

pub struct SMBiosTable<'a> {
    /// Raw formatted area (including header)
    pub data: &'a [u8],
    /// Header struct, references the data
    pub header: &'a SMBiosTableHeader,
    /// String set for the table
    pub strings: Vec<&'a str>,
    /// Pointer to end of the table data (to the following table)
    end: *const u8,
}
impl<'a> SMBiosTable<'a> {
    pub fn new(data: &'a [u8]) -> Result<Self> {
        let header = read::<SMBiosTableHeader>(data);

        let mut strings = Vec::new();
        strings.push(""); // index 0 is not a real string...

        // "Work" pointer
        let start = data.as_ptr() as *const u8;

        // We first skip the structured data
        let strings_section: *const c_char =
            unsafe { start.byte_add(header.length as usize) as *const c_char };

        // Then pick out the strings from the strings section
        let mut wip = strings_section;
        unsafe {
            // No strings in this table...
            if points_to_end(wip) {
                // Skip ahead
                wip = wip.byte_add(1);
            }
            while !points_to_end(wip) {
                let start = wip;
                let mut length = 0;
                while !points_to_end(wip) {
                    length += 1;
                    wip = wip.byte_add(1);
                }
                // To get the NUL byte
                length += 1;
                wip = wip.byte_add(1);
                let buf = core::slice::from_raw_parts(start as *const u8, length);
                let s = read_str(buf);
                strings.push(s);
            }
            if !points_to_end(wip) {
                panic!("End of strings section is unexpectedly not a NUL byte.")
            }

            // Make it point one past...
            wip = wip.byte_add(1);
        }

        Ok(Self {
            data,
            header,
            strings,
            end: wip as *const u8,
        })
    }

    pub unsafe fn from_ptr(ptr: *const u8) -> Result<Self> {
        if ptr.is_null() {
            return Err(Error::new(Status::ABORTED, ()));
        }

        let header_data =
            core::slice::from_raw_parts(ptr, core::mem::size_of::<SMBiosTableHeader>());

        let header = read::<SMBiosTableHeader>(header_data);

        Self::new(core::slice::from_raw_parts(ptr, header.length as usize))
    }

    pub fn get_string(&self, number: SMBiosTableStringRef) -> Option<&str> {
        self.strings.get(number as usize).copied()
    }
}

impl<'a> SMBios3<'a> {
    pub fn new(data: &'a [u8]) -> Result<Self> {
        let entry_point = read::<SMBios3EntryPoint>(data);
        // TODO: validate data (e.g. _SM3_, length, entry point revision, reserved)

        let mut tables = BTreeMap::new();
        unsafe {
            let mut ptr = entry_point.struct_table_address;
            loop {
                let table = SMBiosTable::from_ptr(ptr as *const u8).unwrap();
                if table.header.r#type == Type127::TYPE {
                    break;
                }
                ptr = table.end;
                tables.insert(table.header.r#type, table);
            }
        }

        Ok(Self {
            data,
            entry_point,
            tables,
        })
    }

    pub unsafe fn from_ptr(ptr: *const u8) -> Result<Self> {
        if ptr.is_null() {
            return Err(Error::new(Status::ABORTED, ()));
        }

        Self::new(core::slice::from_raw_parts(
            ptr,
            core::mem::size_of::<SMBios3EntryPoint>(),
        ))
    }

    pub fn raw_data(&self) -> &'a [u8] {
        self.data
    }

    pub fn get_table(&self, number: SMBiosTableType) -> Option<&SMBiosTable> {
        self.tables.get(&number)
    }

    // Temporary until I somehow get how to do this with sum types :/

    pub fn get_bios_information(&self) -> Option<&Type00> {
        if let Some(table) = self.tables.get(&1) {
            unsafe {
                let table_data = core::slice::from_raw_parts(
                    table
                        .data
                        .as_ptr()
                        .byte_add(core::mem::size_of::<SMBiosTableHeader>()),
                    core::mem::size_of::<Type00>(),
                );
                Some(read::<Type00>(table_data))
            }
        } else {
            None
        }
    }

    pub fn get_system_information(&self) -> Option<&Type01> {
        if let Some(table) = self.tables.get(&1) {
            unsafe {
                let table_data = core::slice::from_raw_parts(
                    table
                        .data
                        .as_ptr()
                        .byte_add(core::mem::size_of::<SMBiosTableHeader>()),
                    core::mem::size_of::<Type01>(),
                );
                Some(read::<Type01>(table_data))
            }
        } else {
            None
        }
    }

    pub fn get_enclosure_information(&self) -> Option<&Type03> {
        if let Some(table) = self.tables.get(&1) {
            unsafe {
                let table_data = core::slice::from_raw_parts(
                    table
                        .data
                        .as_ptr()
                        .byte_add(core::mem::size_of::<SMBiosTableHeader>()),
                    core::mem::size_of::<Type03>(),
                );
                Some(read::<Type03>(table_data))
            }
        } else {
            None
        }
    }
}

/// BIOS Information
#[repr(C, packed(1))]
pub struct Type00 {
    pub vendor: SMBiosTableStringRef,
    pub bios_ver: SMBiosTableStringRef,
    pub bios_start_segment: u16,
    pub bios_release_date: SMBiosTableStringRef,
    pub bios_rom_size: u8,
    pub bios_characteristics: [u8; 8],
    pub bios_characteristics_ext1: u8,
    pub bios_characteristics_ext2: u8,
    pub bios_major_release: u8,
    pub bios_minor_release: u8,
    pub ec_major_release: u8,
    pub ec_minor_release: u8,
}
unsafe impl Pod for Type00 {}
impl Type00 {
    pub const TYPE: u8 = 0;
}

/// System Information
#[repr(C, packed(1))]
pub struct Type01 {
    pub manufacturer: SMBiosTableStringRef,
    pub product_name: SMBiosTableStringRef,
    pub version: SMBiosTableStringRef,
    pub serial_number: SMBiosTableStringRef,
    pub UUID: [u8; 16],
    pub wakeup_type: u8,
    pub sku_number: SMBiosTableStringRef,
    pub family: SMBiosTableStringRef,
}
unsafe impl Pod for Type01 {}
impl Type01 {
    pub const TYPE: u8 = 1;
}

/// System Information
#[repr(C, packed(1))]
pub struct Type03 {
    pub manufacturer: SMBiosTableStringRef,
    pub r#type: u8,
    pub version: SMBiosTableStringRef,
    pub serial_number: SMBiosTableStringRef,
    pub asset_tag: SMBiosTableStringRef,
    pub bootup_state: u8,
    pub power_supply_state: u8,
    // And others we won't be using...
}
unsafe impl Pod for Type03 {}
impl Type03 {
    pub const TYPE: u8 = 3;
}

/// Represents the end of the tables list.
/// Not an actual table.
#[repr(C, packed(1))]
pub struct Type127 {}
unsafe impl Pod for Type127 {}
impl Type127 {
    pub const TYPE: u8 = 127;
}

pub enum SMBiosTableTypes {
    Type00,
    Type01,
    Type127,
}
