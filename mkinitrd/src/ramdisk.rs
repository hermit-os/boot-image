extern crate alloc;
use alloc::string::String;

use bincode::{Decode, Encode};

pub const MAGIC_NUMBER: u64 = 0xC0DE4711;

#[derive(Encode, Decode, Debug)]
pub struct InitRamdiskHeader {
	pub magic_number: u64,
}

impl InitRamdiskHeader {
	pub fn new() -> Self {
		Self {
			magic_number: MAGIC_NUMBER,
		}
	}
}

#[derive(Encode, Decode, Debug)]
pub struct InitRamdiskFile {
	pub len: u64,
	pub path: String,
}

impl InitRamdiskFile {
	pub fn new(path: String, len: u64) -> Self {
		Self { len, path }
	}
}
