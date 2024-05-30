use core::ffi::c_void;
use uefi::{guid, Guid};
use uefi::{Result, Status, StatusExt};
use uefi::proto::unsafe_protocol;

pub enum DtFixupFlags {
    DtApplyFixups = 1,
    DtReserveMemory = 2,
}

/// EFI_DT_FIXUP_PROTOCOL
///
///  - https://github.com/U-Boot-EFI/EFI_DT_FIXUP_PROTOCOL
///
#[derive(Debug)]
#[repr(C)]
pub struct DtFixupProtocol {
    pub revision: u64,
    pub fixup: unsafe extern "efiapi" fn(
        this: *mut DtFixupProtocol,
        fdt: *const c_void,
        buffer_size: *const usize,
        flags: u32,
    ) -> Status,
}

impl DtFixupProtocol {
    pub const GUID: Guid = guid!("e617d64c-fe08-46da-f4dc-bbd5870c7300");
}

/// DtFixup protocol
#[derive(Debug)]
#[repr(transparent)]
#[unsafe_protocol(DtFixupProtocol::GUID)]
pub struct DtFixup(DtFixupProtocol);

impl DtFixup {
    pub fn fixup(
        &mut self,
        fdt: *const c_void,
        buffer_size: *const usize,
        flags: DtFixupFlags,
    ) -> Result {
        unsafe {
            (self.0.fixup)(&mut self.0, fdt, buffer_size, flags as u32)
                .to_result()
        }

    }
}
