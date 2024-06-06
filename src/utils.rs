//! Higher-level order helpers

use crate::PREFIX;

use alloc::vec::Vec;
use log::debug;
use uefi::fs::{FileSystem, FileSystemResult, Path, PathBuf};
use uefi::prelude::*;
use uefi::proto::media::fs::SimpleFileSystem;
use uefi::table::boot::ScopedProtocol;
use uefi::CString16;

// https://docs.rs/uefi/latest/uefi/fs/index.html#use-str-as-path
pub fn read_file(bs: &BootServices, path: CString16) -> FileSystemResult<Vec<u8>> {
    debug!("-> read_file({path});");
    let fs: ScopedProtocol<SimpleFileSystem> = bs.get_image_file_system(bs.image_handle()).unwrap();
    let mut fs = FileSystem::new(fs);
    fs.read(Path::new(&path))
}

// TODO: generic "join" with vec input?
pub fn path_for(path: &str) -> CString16 {
    let mut p = PathBuf::from(CString16::try_from(PREFIX).unwrap());
    p.push(PathBuf::from(CString16::try_from(path).unwrap()));
    p.to_cstr16().into()
}
