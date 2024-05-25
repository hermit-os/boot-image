use std::io::Read;
use std::mem::size_of;
use std::path::Path;
use std::{fs, io};

use crate::ramdisk::*;

pub fn list(path: &Path) -> io::Result<()> {
	let mut file = fs::File::open(path)?;
	let config = bincode::config::standard();

	let mut data = vec![];
	file.read_to_end(&mut data)?;

	let (header, len): (InitRamdiskHeader, usize) =
		bincode::decode_from_slice(&data[..size_of::<InitRamdiskHeader>()], config).unwrap();
	if header.magic_number != MAGIC_NUMBER {
		panic!("File isn't a initrd");
	}

	let mut counter = len;
	while counter < data.len() {
		let (ramdisk_file, len): (InitRamdiskFile, usize) =
			bincode::decode_from_slice(&data[counter..], config).unwrap();
		counter += len;

		println!(
			"Found file {:?} ({} bytes)",
			ramdisk_file.path, ramdisk_file.bin.len()
		);
	}

	Ok(())
}
