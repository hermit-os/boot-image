# boot-image

`boot-image` allows booting HermitOS as a common monolithic kernel and
is currently just a _poof of concept_.

## Requirements

On every platform, you essentially need four things:

1. the Rust toolchain via [`rustup`](https://www.rust-lang.org/tools/install)
2. QEMU
3. [mtools](https://www.gnu.org/software/mtools/) to create the disk image
   (`apt install mtools` on Debian/Ubuntu, `brew install mtools` on macOS)
4. a POSIX-compatible shell as well as `make` and `git`

The kernel is included as the Git submodule [`kernel`](kernel), so clone the
repository with `git clone --recurse-submodules` or run
`git submodule update --init` afterwards.

The exact nightly version is pinned in
[`rust-toolchain.toml`](rust-toolchain.toml) and is installed
automatically the first time `cargo` is invoked.

## Building the boot image

By default, `make` builds the image for `x86_64`. You can select
the architecture with the environment variable `ARCH`. Besides `x86_64`,
the values `aarch64` and `riscv64` are supported.

```sh
make ARCH=x86_64
```

## Disk image

The user-space binaries are not embedded into the boot image. Instead,
`make disk.img` creates a 128 MiB FAT32 image with the content of
[`data/<ARCH>`](data). The kernel mounts this image at `/root`, and the boot
image additionally mounts its directory `bin` at `/bin`. The first process is
started from `/bin`, see `main` in [`src/main.rs`](src/main.rs).

`make` rebuilds `disk.img` when the binaries in `data/<ARCH>` or the
architecture change. The size of the image can be changed with the variable
`DISK_SIZE_MB`.

## Booting the kernel

Boot the kernel with `ARCH=x86_64 make run` or use the following command:

```sh
ARCH=x86_64 make run
```

`make run` creates the disk image if necessary and starts QEMU through
`cargo run`. The QEMU command lines are defined as runners in
[`.cargo/config.toml`](.cargo/config.toml) and, for `aarch64`, in
[`qemu-aarch64.sh`](qemu-aarch64.sh). The disk image is attached as a
virtio block device.

For `riscv64`, the runner expects the OpenSBI firmware at
`/usr/lib/riscv64-linux-gnu/opensbi/generic/fw_jump.bin`, as provided by the
Debian/Ubuntu package `opensbi`. On other systems, adjust the path to `-bios`.

## User-space binaries

To build the binaries in [`data`](data), the repository `hermit-rs` is used with the pull request [`970`](https://github.com/hermit-os/hermit-rs/pull/970).
For instance, the `fork` example is built with the following command:

```sh
cargo build -Zbuild-std=std,panic_abort -Zbuild-std-features=compiler-builtins-mem --target x86_64-unknown-hermit --no-default-features --features hermit/common-os,hermit/loader -p fork --release
```

For the other architectures, replace the target with `aarch64-unknown-hermit` or `riscv64gc-unknown-hermit`.
Afterwards, copy the binary from `target/<target>/release/` to `data/<ARCH>/bin/`.

## License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

The kernel is being developed on [hermit-os/kernel](https://github.com/hermit-os/kernel).
Create your own fork, send us a pull request, and chat with us on [Zulip](https://hermit.zulipchat.com/).
