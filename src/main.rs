#![no_std] // don't link the Rust standard library
#![no_main]

use hermit::sys_shutdown;

#[macro_use]
extern crate log;
#[macro_use]
extern crate hermit;

#[no_mangle] // don't mangle the name of this function
pub extern "C" fn main() -> ! {
	info!("Enter main function");
	sys_shutdown(0);

	// we should never reach this point
	loop {}
}