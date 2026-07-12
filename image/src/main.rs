#![no_std] // don't link the Rust standard library
#![no_main]

#[macro_use]
extern crate log;
extern crate alloc;
extern crate hermit;

use alloc::borrow::ToOwned;
use alloc::boxed::Box;
use alloc::ffi::CString;
use alloc::vec;
use alloc::vec::Vec;
use core::ffi::{c_char, CStr};

use align_address::Align;
use embedded_io::Read;
use goblin::elf::program_header::{PT_DYNAMIC, PT_GNU_RELRO, PT_LOAD, PT_TLS};
#[cfg(target_arch = "aarch64")]
use goblin::elf::reloc::{R_AARCH64_NONE, R_AARCH64_RELATIVE};
#[cfg(target_arch = "x86_64")]
use goblin::elf::reloc::{R_X86_64_NONE, R_X86_64_RELATIVE};
use goblin::elf64::dynamic::{DT_RELA, DT_RELAENT, DT_RELASZ};
use goblin::{elf, elf64};
use hermit::common_os::{jump_to_user_land, load_application};
use hermit::fd::AccessPermission;
use hermit::fs::{self, create_dir, create_file, File};
use hermit::scheduler::task::NORMAL_PRIO;
use hermit::scheduler::{join, shutdown, spawn};
use hermit::arch::{PageSize, BasePageSize};
use ramdisk::*;

#[cfg(target_arch = "x86_64")]
const EXPECTED_MACHINE: u16 = goblin::elf::header::EM_X86_64;
#[cfg(target_arch = "aarch64")]
const EXPECTED_MACHINE: u16 = goblin::elf::header::EM_AARCH64;
#[cfg(target_arch = "riscv64")]
const EXPECTED_MACHINE: u16 = goblin::elf::header::EM_RISCV;

static INITD: &[u8] = include_bytes!("../../initrd.img");

#[path = "../../mkinitrd/src/ramdisk.rs"]
pub mod ramdisk;

#[derive(Debug, PartialEq)]
pub enum LoaderError {
	IoError(i32),
	ParseError,
	InvalidElfFile,
	UnsupportedArchitecture,
	LoadingError,
}

fn loader(app: &CStr, args: Vec<CString>, envs: Vec<CString>) -> Result<(), LoaderError> {
	let app = app.to_str().expect("Invalid UTF-8 in application path");

	debug!("Load application {app}");

	let meta = fs::metadata(app).map_err(|e| LoaderError::IoError(e.into()))?;
	let len = meta.len();

	// Scope the `File` so it is dropped (closing the loader's fd) *before*
	// `load_application` installs the new process's object_map. Otherwise
	// the loader's file would happen to occupy fd 0 in the still-empty
	// kernel-loader map; after `load_application` replaces the map with
	// `[stdin=0, stdout=1, stderr=2]`, the deferred `File::drop` would
	// `sys_close(0)` and silently kill the freshly installed stdin slot.
	let mut buffer = vec![0; len];
	{
		let mut file = File::open(app).map_err(|e| LoaderError::IoError(e.into()))?;
		file.read(&mut buffer)
			.map_err(|e| LoaderError::IoError(e.into()))?;
	}
	let elf = match elf::Elf::parse(&buffer) {
		Ok(n) => n,
		_ => return Err(LoaderError::ParseError),
	};

	if elf.header.e_machine != EXPECTED_MACHINE {
		error!(
			"Wrong architecture: e_machine = 0x{:x}, expected 0x{:x}",
			elf.header.e_machine, EXPECTED_MACHINE
		);
		return Err(LoaderError::UnsupportedArchitecture);
	}

	if !elf.is_64 {
		return Err(LoaderError::InvalidElfFile);
	}

	if !elf.libraries.is_empty() {
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
		let mut tls_init_image: Option<alloc::vec::Vec<u8>> = None;
		let mut bss_start: usize = 0;

		for i in &elf.program_headers {
			match i.p_type {
				PT_LOAD => {
					debug!("Load code at address 0x{:x}", i.p_vaddr);

					let size = i.p_vaddr as usize;
					bss_start = core::cmp::max(bss_start, size + i.p_filesz as usize);
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

					let elf_tls_data =
						&buffer[(i.p_offset as usize)..(i.p_offset + i.p_filesz) as usize];

					if let Some(ref mut tls) = tls_slice {
						tls[..i.p_filesz as usize].clone_from_slice(elf_tls_data);
					}

					let tls_memsz = i.p_memsz.align_up(i.p_align) as usize;
					let mut init = alloc::vec![0u8; tls_memsz];
					init[..elf_tls_data.len()].copy_from_slice(elf_tls_data);
					tls_init_image = Some(init);
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

		// clear BBS section
		code_slice[bss_start..].iter_mut().for_each(|x| *x = 0);

		if rela_addr != 0 && relasz != 0 {
			let rela = unsafe {
				elf64::reloc::from_raw_rela(rela_addr as *const elf64::reloc::Rela, relasz as usize)
			};
			for j in rela {
				let offset =
					unsafe { code_slice.as_mut_ptr().offset(j.r_offset as isize) as *mut u64 };
				let r_type = (j.r_info & 0xffff_ffff) as u32;

				match r_type {
					#[cfg(target_arch = "aarch64")]
					R_AARCH64_RELATIVE => unsafe {
						*offset = user_start + j.r_addend as u64;
					},
					#[cfg(target_arch = "aarch64")]
					R_AARCH64_NONE => {} // no-op

					#[cfg(target_arch = "x86_64")]
					R_X86_64_RELATIVE => unsafe {
						*offset = user_start + j.r_addend as u64;
					},
					#[cfg(target_arch = "x86_64")]
					R_X86_64_NONE => {}

					other => {
						error!("Unsupported relocation type {other}");
						return Err(());
					}
				}
			}
		}

		Ok(tls_init_image)
	};

	load_application(exec_size, tls_size, elf_reader).map_err(|_| LoaderError::LoadingError)?;

	// After a jump to the user space, the application will
	// never comeback => release buffers
	drop(elf);
	drop(buffer);

	unsafe {
		jump_to_user_land(entry.try_into().unwrap(), args, envs);
	}
}

/// Arguments handed from `sys_spawn_process` to `loader_entry`.
///
/// All strings are owned copies on the kernel heap, so they stay valid
/// after the spawning process's user memory is gone.
struct SpawnArgs {
	path: CString,
	args: Vec<CString>,
	envs: Vec<CString>,
}

/// Copy a NULL-terminated C string array (`argv`/`envp` convention) from
/// user memory into owned kernel-heap strings.
///
/// # Safety
///
/// If non-null, `array` must point to a NULL-terminated array of valid
/// pointers to NUL-terminated C strings.
unsafe fn copy_c_str_array(mut array: *const *const c_char) -> Vec<CString> {
	let mut vec = Vec::new();

	if !array.is_null() {
		unsafe {
			while !(*array).is_null() {
				vec.push(CStr::from_ptr(*array).to_owned());
				array = array.add(1);
			}
		}
	}

	vec
}

fn mount_initd() {
	create_dir("/bin", AccessPermission::from_bits(0o777).unwrap())
		.expect("Unable to create directory /bin");

	let (header, mut offset) =
		InitRamdiskHeader::decode(INITD).expect("Failed to decode initrd header");
	if header.magic_number != MAGIC_NUMBER {
		panic!("File isn't a initrd");
	}

	while offset < INITD.len() {
		let (ramdisk_file, len) =
			InitRamdiskFile::decode(&INITD[offset..]).expect("Failed to decode initrd entry");
		offset += len;

		let decompressed = lz4_flex::decompress_size_prepended(&ramdisk_file.bin)
			.expect("Failed to decompress initrd entry");

		info!(
			"Mount file to {} ({} bytes)",
			ramdisk_file.path,
			decompressed.len()
		);

		// Mount in-memory file
		if create_file(
			&ramdisk_file.path,
			Box::leak(decompressed.into_boxed_slice()),
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
}

extern "C" fn loader_entry(arg: usize) {
	let spawn_args =
		unsafe { Box::from_raw(core::ptr::with_exposed_provenance_mut::<SpawnArgs>(arg)) };
	let SpawnArgs { path, args, envs } = *spawn_args;
	let _ = loader(&path, args, envs).map_err(|e| error!("Unable to load {path:?}: {e:?}"));
}

/// Spawn a new process by loading the binary at `path`.
///
/// `argv` and `envp` are optional NULL-terminated arrays of C strings
/// that become the new process's arguments and environment. If `argv`
/// is null (or empty), the new process gets `[path]` as its argument
/// vector.
///
/// # Safety
///
/// `path` must be a valid pointer to a NUL-terminated C string; `argv`
/// and `envp` must each be null or a valid NULL-terminated C string
/// array. All pointers only have to stay readable until this function
/// returns — the strings are copied to the kernel heap.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sys_spawn_process(
	path: *const c_char,
	argv: *const *const c_char,
	envp: *const *const c_char,
) -> i32 {
	// create owned copies in our kernel heap
	let path = unsafe { CStr::from_ptr(path) }.to_owned();
	let mut args = unsafe { copy_c_str_array(argv) };
	if args.is_empty() {
		args.push(path.clone());
	}
	let envs = unsafe { copy_c_str_array(envp) };

	let spawn_args = Box::new(SpawnArgs { path, args, envs });
	let id: i32 = unsafe {
		spawn(
			loader_entry,
			Box::into_raw(spawn_args) as usize,
			NORMAL_PRIO,
			hermit::config::DEFAULT_STACK_SIZE,
			-1,
		)
	}
	.into();

	id
}

/// The function sys_exec function replace the current process image
/// with a new process image.
///
/// `argv` and `envp` are optional NULL-terminated arrays of C strings
/// that become the new process image's arguments and environment. If
/// `argv` is null (or empty), the new image gets `[path]` as its
/// argument vector.
///
/// # Safety
///
/// `path` must be a valid pointer to a NUL-terminated C string; `argv`
/// and `envp` must each be null or a valid NULL-terminated C string
/// array. All pointers point into the caller's user memory, so they
/// have to be copied to the kernel heap *before* the old address space
/// is torn down.
#[cfg(feature = "fork")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn sys_exec(
	path: *const c_char,
	argv: *const *const c_char,
	envp: *const *const c_char,
) -> i32 {
	// Create owned copies in our kernel heap. This must happen before
	// `clear_user_space` unmaps the memory the pointers refer to.
	let app = unsafe { CStr::from_ptr(path) }.to_owned();
	let mut args = unsafe { copy_c_str_array(argv) };
	if args.is_empty() {
		args.push(app.clone());
	}
	let envs = unsafe { copy_c_str_array(envp) };

	hermit::arch::clear_user_space();
	let _ = loader(&app, args, envs).map_err(|e| error!("Unable to load {app:?}: {e:?}"));

	0
}

#[no_mangle] // don't mangle the name of this function
pub extern "C" fn main(_argc: i32, _argv: *const *const u8, _env: *const *const u8) {
	mount_initd();

	info!("Start user-level process to initialize the HermitOS");

	let app = c"/bin/fork".to_owned();
	let spawn_args = Box::new(SpawnArgs {
		args: vec![app.clone()],
		envs: Vec::new(),
		path: app,
	});
	let id = unsafe {
		spawn(
			loader_entry,
			Box::into_raw(spawn_args) as usize,
			NORMAL_PRIO,
			hermit::config::DEFAULT_STACK_SIZE,
			-1,
		)
	};
	let _ = join(id);

	shutdown(0);
}
