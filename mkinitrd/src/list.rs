use std::path::Path;
use std::{fs, io};

use crate::ramdisk::*;

pub fn list(path: &Path) -> io::Result<()> {
	let data = fs::read(path)?;

	let (header, mut offset) =
		InitRamdiskHeader::decode(&data).expect("Failed to decode initrd header");
	if header.magic_number != MAGIC_NUMBER {
		panic!("File isn't a initrd");
	}

	while offset < data.len() {
		let (ramdisk_file, len) =
			InitRamdiskFile::decode(&data[offset..]).expect("Failed to decode initrd entry");
		offset += len;

		println!(
			"Found file {:?} ({} bytes)",
			ramdisk_file.path,
			ramdisk_file.bin.len()
		);
	}

	Ok(())
}
