use core::ffi::c_void;
use log::debug;
use uefi::proto::unsafe_protocol;
use uefi::{guid, Guid};
use uefi::{Result, Status, StatusExt};

bitflags::bitflags! {
    /// Flags that can be given to the EFI_DT_FIXUP_PROTOCOL.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
    #[repr(transparent)]
    pub struct DtFixupFlags: u32 {
        const DtApplyFixups = 1;
        const DtReserveMemory = 2;
    }
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
///
///  - https://github.com/U-Boot-EFI/EFI_DT_FIXUP_PROTOCOL?tab=readme-ov-file#parameters-1
///
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
        debug!("-> Calling EFI_DT_FIXUP...");
        unsafe { (self.0.fixup)(&mut self.0, fdt, buffer_size, flags.bits()).to_result() }
    }
}
