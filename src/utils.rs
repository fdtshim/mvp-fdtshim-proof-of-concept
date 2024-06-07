//! Higher-level order helpers

use crate::PREFIX;

use alloc::string::ToString;
use alloc::vec::Vec;
use log::debug;
use log::error;
use uefi::fs::{FileSystem, FileSystemResult, Path, PathBuf};
use uefi::prelude::*;
use uefi::proto::device_path::build::{self, DevicePathBuilder};
use uefi::proto::device_path::{DevicePath, DeviceSubType, DeviceType, LoadedImageDevicePath};
//use uefi::proto::loaded_image::LoadedImage;
use uefi::proto::media::fs::SimpleFileSystem;
use uefi::table::boot::LoadImageSource;
use uefi::table::boot::ScopedProtocol;
use uefi::CString16;
use uefi::Result;

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

/// Wrapper around load_image and start_image to "simply" launch an EFI program from path.
pub fn exec(bs: &BootServices, path: CString16) -> Status {
    let mut storage = Vec::new();
    if let Ok(image_path) = get_image_path_for(bs, &mut storage, &path) {
        if let Ok(image_handle) = bs.load_image(
            bs.image_handle(),
            LoadImageSource::FromDevicePath {
                device_path: image_path,
                from_boot_manager: false,
            },
        ) {
            // FIXME: take in params too for "execline"-like support
            /*
            let mut loaded_image = bs
                .open_protocol_exclusive::<LoadedImage>(image_handle)
                .expect("failed to open LoadedImage protocol");
            let load_options = cstr16!(r"{path} *args"); // XXX
            unsafe {
                loaded_image.set_load_options(
                    load_options.as_ptr().cast(),
                    load_options.num_bytes() as u32,
                );
            }
            */

            debug!("Launching image {:?}...", path.to_string());
            if let Err(err) = bs.start_image(image_handle) {
                err.status()
            } else {
                Status::LOAD_ERROR
            }
        } else {
            error!("failed to load image {:?}", path.to_string());
            Status::LOAD_ERROR
        }
    } else {
        Status::LOAD_ERROR
    }
}

fn get_image_path_for<'a>(
    bs: &BootServices,
    buf: &'a mut Vec<u8>,
    path_name: &CString16,
) -> Result<&'a DevicePath> {
    match bs.open_protocol_exclusive::<LoadedImageDevicePath>(bs.image_handle()) {
        Err(st) => {
            error!("failed to open LoadedImageDevicePath protocol");
            Err(st)
        }
        Ok(loaded_image_device_path) => {
            let mut builder = DevicePathBuilder::with_vec(buf);
            for node in loaded_image_device_path.node_iter() {
                if node.full_type() == (DeviceType::MEDIA, DeviceSubType::MEDIA_FILE_PATH) {
                    break;
                }
                builder = builder.push(&node).unwrap();
            }
            builder = builder
                .push(&build::media::FilePath {
                    path_name: &path_name,
                })
                .unwrap();

            Ok(builder.finalize().unwrap())
        }
    }
}
