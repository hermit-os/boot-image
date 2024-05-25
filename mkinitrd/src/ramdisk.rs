extern crate alloc;
use alloc::string::String;
use alloc::vec::Vec;

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
	pub path: String,
	pub bin: Vec<u8>,
}

impl InitRamdiskFile {
	pub fn new(path: String, bin: Vec<u8>) -> Self {
		Self { path, bin }
	}
}
