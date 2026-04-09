use std::io::Write;
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
		let mut file = fs::File::create("initrd.img")?;

		let ramdisk = InitRamdiskHeader::new();
		file.write_all(&ramdisk.encode())?;

		visit_dirs(path, &mut |entry| {
			let binding = entry.path();
			let fname = binding
				.to_str()
				.unwrap()
				.strip_prefix(path.to_str().unwrap())
				.unwrap();

			let data = fs::read(entry.path())?;
			let compressed = lz4_flex::compress_prepend_size(&data);

			info!(
				"Adding {} ({} -> {} bytes, {:.0}%)",
				fname,
				data.len(),
				compressed.len(),
				compressed.len() as f64 / data.len() as f64 * 100.0
			);

			let ramdisk_file = InitRamdiskFile::new(fname.to_string(), compressed);
			file.write_all(&ramdisk_file.encode())
		})?;
	}

	Ok(())
}
