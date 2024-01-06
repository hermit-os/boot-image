#![no_std] // don't link the Rust standard library
#![no_main]

#[macro_use]
extern crate log;
extern crate alloc;
extern crate hermit;

use alloc::string::String;
use alloc::vec::Vec;
use goblin::elf::program_header::{PT_DYNAMIC, PT_GNU_RELRO, PT_LOAD};
use goblin::elf64::dynamic::{DT_RELA, DT_RELASZ};
use goblin::elf64::reloc::{R_386_GLOB_DAT, R_386_RELATIVE};
use goblin::{elf, elf64};
use hermit::fd::IoError;
use hermit::fs::{self, File};
use hermit::io::Read;
use hermit::sys_shutdown;

fn boot_system() -> Result<(), IoError> {
    let mut version: String = String::new();
    let mut file = File::open("/proc/version")?;
    file.read_to_string(&mut version)?;
    info!("{version}");

    let mut file = File::open("/host/data/hello_world")?;
    let metadata = file.metadata()?;
    let mut buffer: Vec<u8> = Vec::new();

    buffer.resize(metadata.len(), 0);
    file.read(&mut buffer)?;
    let elf = match elf::Elf::parse(&buffer) {
        Ok(n) => n,
        _ => return Err(IoError::EINVAL),
    };
    debug!("elf information: {:#?}", &elf);

    if !elf.is_64 {
        return Err(IoError::EINVAL);
    }

    if elf.libraries.len() > 0 {
        error!(
            "Error: file depends on following libraries: {:?}",
            elf.libraries
        );
        return Err(IoError::EINVAL);
    }

    let meta = fs::metadata("/proc/version").unwrap();
    info!("metadata of /proc/version: {:?}", meta);
    info!("access time of /proc/version: {:?}", meta.accessed()?);

    let meta = fs::metadata("/host/data/hello_world").unwrap();
    info!("metadata of /host/data/hello_world: {:?}", meta);
    info!("access time of /host/data/hello_world: {:?}", meta.accessed()?);

    /*info!("Read content of / with");
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
    }*/

    Ok(())
}

#[no_mangle] // don't mangle the name of this function
pub extern "C" fn main(_argc: i32, _argv: *const *const u8, _env: *const *const u8) {
    if boot_system().is_err() {
        error!("Unable to boot system!");
    }

    sys_shutdown(0);
}
