#[macro_use]
extern crate log;
#[macro_use]
extern crate clap;

pub mod create;
pub mod list;
pub mod ramdisk;

use crate::create::*;
use crate::list::*;
use clap::{arg, Command};
use std::path::PathBuf;

fn cli() -> Command {
	env_logger::init();

	Command::new(crate_name!())
		.author(crate_authors!(", "))
		.version(crate_version!())
		.about("A tool to create a initial ramdisk for HermitOS")
		.subcommand_required(true)
		.arg_required_else_help(true)
		.allow_external_subcommands(false)
		.subcommand(
			Command::new("create")
				.about("Create new initrd")
				.arg(
					arg!(<PATH> "Path to a directory, where the unpacked initrd is located.")
						.value_parser(clap::value_parser!(PathBuf)),
				)
				.arg_required_else_help(true),
		)
		.subcommand(
			Command::new("list")
				.about("Show content of the initrd")
				.arg(arg!(<PATH> "Path to initrd").value_parser(clap::value_parser!(PathBuf)))
				.arg_required_else_help(true),
		)
}

fn main() {
	let matches = cli().get_matches();

	match matches.subcommand() {
		Some(("create", sub_matches)) => {
			let path = sub_matches
				.get_one::<PathBuf>("PATH")
				.expect("PATH is required");
			create(path).expect("Unable to create initrd");
		}
		Some(("list", sub_matches)) => {
			let path = sub_matches
				.get_one::<PathBuf>("PATH")
				.expect("PATH is required");
			list(path).expect("Unable to create initrd");
		}
		Some((name, _)) => error!("Unsupported subcommand `{name}`"),
		_ => unreachable!(),
	}
}
