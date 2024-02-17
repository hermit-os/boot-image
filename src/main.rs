#![no_std] // don't link the Rust standard library
#![no_main]

#[macro_use]
extern crate log;
extern crate alloc;
extern crate hermit;

use align_address::Align;
use alloc::vec;
use goblin::elf::program_header::{PT_DYNAMIC, PT_GNU_RELRO, PT_LOAD, PT_TLS};
use goblin::elf64::dynamic::{DT_RELA, DT_RELAENT, DT_RELASZ};
use goblin::elf64::reloc::R_386_RELATIVE;
use goblin::{elf, elf64};
use hermit::arch::{jump_to_user_land, load_application, BasePageSize, PageSize};
use hermit::fd::AccessPermission;
use hermit::fs::{self, create_file, File};
use hermit::io::Read;
use hermit::scheduler::task::NORMAL_PRIO;
use hermit::scheduler::{join, spawn};
use hermit::sys_shutdown;

static INITD: &[u8] = include_bytes!("../data/hello_world");

#[derive(Debug, PartialEq)]
pub enum LoaderError {
    IoError(i32),
    ParseError,
    InvalidElfFile,
    LoadingError,
}

fn loader() -> Result<(), LoaderError> {
    let meta = fs::metadata("/initd")
        .map_err(|e| LoaderError::IoError(num::ToPrimitive::to_i32(&e).unwrap()))?;
    let len = meta.len();
    let mut file = File::open("/initd")
        .map_err(|e| LoaderError::IoError(num::ToPrimitive::to_i32(&e).unwrap()))?;

    let mut buffer = vec![0; len];
    file.read(&mut buffer)
        .map_err(|e| LoaderError::IoError(num::ToPrimitive::to_i32(&e).unwrap()))?;
    let elf = match elf::Elf::parse(&buffer) {
        Ok(n) => n,
        _ => return Err(LoaderError::ParseError),
    };

    if !elf.is_64 {
        return Err(LoaderError::InvalidElfFile);
    }

    if elf.libraries.is_empty() {
        error!(
            "Error: file depends on following libraries: {:?}",
            elf.libraries
        );
        return Err(LoaderError::InvalidElfFile);
    }

    // Determine the memory size of the executable and
    // the thread local storage
    let mut exec_size: u64 = 0;
    let mut vstart: u64 = 0;
    let mut tls_size: u64 = 0;
    for i in &elf.program_headers {
        if i.p_type == PT_LOAD {
            // the first loadable segment defines the start address of the program
            if exec_size == 0 {
                vstart = i.p_vaddr;
            }

            let size = (i.p_vaddr - vstart + i.p_memsz).align_up(BasePageSize::SIZE);
            exec_size = core::cmp::max(exec_size, size);
        } else if i.p_type == PT_TLS {
            tls_size = i.p_memsz.align_up(i.p_align);
        }
    }
    debug!("Start address of the application 0x{:x}", vstart);
    debug!("Memory size 0x{:x}", exec_size);
    debug!("ELF entry point 0x{:x}", elf.entry);
    assert!(vstart == 0, "Invalid start address!");

    if exec_size == 0 {
        error!("Error: unable to find PT_LOAD",);
        return Err(LoaderError::InvalidElfFile);
    }

    let entry = elf.entry;

    let elf_reader = |code_slice: &mut [u8], mut tls_slice: Option<&mut [u8]>| {
        let user_start = code_slice.as_ptr() as u64;
        let mut rela_addr: u64 = 0;
        let mut relasz: u64 = 0;

        for i in &elf.program_headers {
            match i.p_type {
                PT_LOAD => {
                    debug!("Load code at address 0x{:x}", i.p_vaddr);

                    let size = i.p_vaddr as usize;
                    code_slice[size..size + i.p_filesz as usize].clone_from_slice(
                        &buffer[(i.p_offset as usize)..(i.p_offset + i.p_filesz) as usize],
                    );
                }
                PT_GNU_RELRO => {
                    debug!(
                        "PT_GNU_RELRO at 0x{:x} (size 0x{:x})",
                        i.p_vaddr, i.p_filesz
                    );
                }
                PT_TLS => {
                    debug!("Found TLS at 0x{:x} (size {})", i.p_vaddr, i.p_memsz);

                    if let Some(ref mut tls) = tls_slice {
                        tls[..i.p_filesz as usize].clone_from_slice(
                            &buffer[(i.p_offset as usize)..(i.p_offset + i.p_filesz) as usize],
                        );
                    }
                }
                PT_DYNAMIC => {
                    debug!("PT_DYNAMIC at 0x{:x} (size 0x{:x})", i.p_vaddr, i.p_filesz);

                    let mem = unsafe { code_slice.as_mut_ptr().offset(i.p_vaddr as isize) };
                    let r#dyn = unsafe { elf::dynamic::dyn64::from_raw(0, mem as usize) };

                    for j in r#dyn {
                        if j.d_tag == DT_RELA {
                            rela_addr = user_start + j.d_val;
                        } else if j.d_tag == DT_RELASZ {
                            relasz = j.d_val;
                        } else if j.d_tag == DT_RELAENT {
                            debug!("Size of the relocation entry: {}", j.d_val);
                        }
                    }
                }
                _ => {}
            }
        }

        if rela_addr != 0 && relasz != 0 {
            let rela = unsafe {
                elf64::reloc::from_raw_rela(rela_addr as *const elf64::reloc::Rela, relasz as usize)
            };
            for j in rela {
                let offset =
                    unsafe { code_slice.as_mut_ptr().offset(j.r_offset as isize) as *mut u64 };

                if (j.r_info & 0xF) == R_386_RELATIVE as u64 {
                    unsafe {
                        *offset = user_start + j.r_addend as u64;
                    }
                } else {
                    error!("Unsupported relocation type {}", j.r_info & 0xF);
                    return Err(());
                }
            }
        }

        Ok(())
    };

    load_application(exec_size, tls_size, elf_reader).map_err(|_| LoaderError::LoadingError)?;

    // After a jump to the user space, the application will
    // never comeback => release buffers
    drop(elf);
    drop(buffer);

    unsafe {
        jump_to_user_land(entry, exec_size);
    }
}

extern "C" fn init_loader(_: usize) {
    let _ = loader().map_err(|e| error!("Unable to load initd: {:?}", e));
}

#[no_mangle] // don't mangle the name of this function
pub extern "C" fn main(_argc: i32, _argv: *const *const u8, _env: *const *const u8) {
    info!("Start user-level process to initialize the HermitOS");

    // Mount in-memory file
    unsafe {
        if create_file(
            "/initd",
            INITD.as_ptr(),
            INITD.len(),
            AccessPermission::S_IRUSR
                | AccessPermission::S_IRGRP
                | AccessPermission::S_IROTH
                | AccessPermission::S_IXUSR
                | AccessPermission::S_IXGRP
                | AccessPermission::S_IXOTH,
        )
        .is_err()
        {
            error!("Unable to mount file");
        }
    }

    let id = spawn(init_loader, 0, NORMAL_PRIO, hermit::DEFAULT_STACK_SIZE, -1);
    let _ = join(id);

    sys_shutdown(0);
}
