use std::io::{Read, Write};
use std::path::Path;
use std::{fs, io};

use crate::ramdisk::*;

fn visit_dirs(dir: &Path, cb: &mut dyn FnMut(&fs::DirEntry) -> io::Result<()>) -> io::Result<()> {
	if dir.is_dir() {
		for entry in fs::read_dir(dir)? {
			let entry = entry?;
			let path = entry.path();
			if path.is_dir() {
				visit_dirs(&path, cb)?;
			} else {
				cb(&entry)?;
			}
		}
	}

	Ok(())
}

pub fn create(path: &Path) -> io::Result<()> {
	if !path.is_dir() {
		error!("{} must be a directory!", path.display());
	} else {
		let config = bincode::config::standard();
		let mut file = fs::File::create("initrd.img")?;

		let ramdisk = InitRamdiskHeader::new();
		let buf: Vec<u8> = bincode::encode_to_vec(&ramdisk, config).unwrap();
		file.write_all(&buf[..])?;

		visit_dirs(path, &mut |entry| {
			let binding = entry.path();
			let fname = binding
				.to_str()
				.unwrap()
				.strip_prefix(path.to_str().unwrap())
				.unwrap();

			let mut fexec = fs::File::open(entry.path())?;
			let mut data = vec![];
			fexec.read_to_end(&mut data)?;

			let ramdisk_file = InitRamdiskFile::new(fname.to_string(), data);
			let buf = bincode::encode_to_vec(&ramdisk_file, config).unwrap();
			file.write_all(&buf[..])
		})?;
	}

	Ok(())
}
