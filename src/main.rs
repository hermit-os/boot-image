#![no_std] // don't link the Rust standard library
#![no_main]

#[macro_use]
extern crate log;
extern crate alloc;
extern crate hermit;

use alloc::string::String;
use hermit::fs::{self, readdir, File};
use hermit::io::{Read, Write};
use hermit::sys_shutdown;

#[no_mangle] // don't mangle the name of this function
pub extern "C" fn main(_argc: i32, _argv: *const *const u8, _env: *const *const u8) {
    info!("Enter main function");

    info!("Read content of / with");
    for i in readdir("/").expect("Unable to read /").iter() {
        info!("{:?}", *i);
    }

    info!("Read content of /proc with");
    for i in readdir("/proc").expect("Unable to read /proc").iter() {
        info!("{:?}", *i);
    }

    if let Ok(attr) = fs::file_attributes("/proc/version") {
        info!("Attributes of /etc/version {:?}", attr);
    } else {
        error!("Unable to get file attributes");
    }

    if let Ok(mut file) = File::open("/proc/version") {
        let mut version: String = String::new();
        if file.read_to_string(&mut version).is_err() {
            error!("Unable to read /proc/version");
        }
        info!("version: {}", version);
    } else {
        error!("Unable to open file");
    };

    info!("create file /host/test.txt");
    if let Ok(mut file) = File::create("/host/test.txt") {
        write!(file, "Hello Linux!").expect("Unable to write into /host/test.txt");
    } else {
        error!("Unable to create file");
    }

    info!("read file /host/test.txt");
    if let Ok(mut file) = File::open("/host/test.txt") {
        let mut content: String = String::new();
        if file.read_to_string(&mut content).is_err() {
            error!("Unable to read /proc/version");
        }
        info!("File content {}", content);
    } else {
        error!("Unable to open file");
    }

    sys_shutdown(0);
}
