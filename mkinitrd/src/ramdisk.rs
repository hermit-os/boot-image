extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

pub const MAGIC_NUMBER: u64 = 0xc0de4711;

#[derive(Debug)]
pub struct InitRamdiskHeader {
	pub magic_number: u64,
}

impl Default for InitRamdiskHeader {
	fn default() -> Self {
		Self::new()
	}
}

impl InitRamdiskHeader {
	pub fn new() -> Self {
		Self {
			magic_number: MAGIC_NUMBER,
		}
	}

	pub fn encode(&self) -> Vec<u8> {
		self.magic_number.to_le_bytes().to_vec()
	}

	pub fn decode(data: &[u8]) -> Option<(Self, usize)> {
		let bytes: [u8; 8] = data.get(..8)?.try_into().ok()?;
		Some((
			Self {
				magic_number: u64::from_le_bytes(bytes),
			},
			8,
		))
	}
}

#[derive(Debug)]
pub struct InitRamdiskFile {
	pub path: String,
	pub bin: Vec<u8>,
}

impl InitRamdiskFile {
	pub fn new(path: String, bin: Vec<u8>) -> Self {
		Self { path, bin }
	}

	pub fn encode(&self) -> Vec<u8> {
		let path_bytes = self.path.as_bytes();
		let mut buf = Vec::with_capacity(8 + path_bytes.len() + 8 + self.bin.len());
		buf.extend_from_slice(&(path_bytes.len() as u64).to_le_bytes());
		buf.extend_from_slice(path_bytes);
		buf.extend_from_slice(&(self.bin.len() as u64).to_le_bytes());
		buf.extend_from_slice(&self.bin);
		buf
	}

	pub fn decode(data: &[u8]) -> Option<(Self, usize)> {
		let mut offset = 0;

		let path_len = u64::from_le_bytes(data.get(offset..offset + 8)?.try_into().ok()?) as usize;
		offset += 8;
		let path = core::str::from_utf8(data.get(offset..offset + path_len)?).ok()?;
		offset += path_len;

		let bin_len = u64::from_le_bytes(data.get(offset..offset + 8)?.try_into().ok()?) as usize;
		offset += 8;
		let bin = data.get(offset..offset + bin_len)?.to_vec();
		offset += bin_len;

		Some((
			Self {
				path: String::from(path),
				bin,
			},
			offset,
		))
	}
}
