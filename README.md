# boot-image

`boot-image` allows booting HermitOS as a common monolithic kernel.

## Requirements

On every platform, you essentially need three things:

1. the Rust toolchain via [`rustup`](https://www.rust-lang.org/tools/install)
2. QEMU
3. a POSIX-compatible shell as well as `make` and `git`

The exact nightly version is pinned in
[`image/rust-toolchain.toml`](image/rust-toolchain.toml) and is installed
automatically the first time `cargo` is invoked.

## Building the boot image

By default, `make` builds the image for `x86_64`. You can select
the architecture with the environment variable `ARCH`. Besides `x86_64`,
the values `aarch64` and `riscv64` are supported.

```sh
ARCH=x86_64 make
```

## Booting the kernel

Boot the kernel with `ARCH=x86_64 make run` or use the following command:

```sh
qemu-system-x86_64 -display none -serial stdio -kernel hermit-loader-x86_64 -cpu Skylake-Client -device isa-debug-exit,iobase=0xf4,iosize=0x04 -smp 1 -m 2G -netdev user,id=u1,hostfwd=tcp::9975-:9975,hostfwd=udp::9975-:9975,net=192.168.76.0/24,dhcpstart=192.168.76.9 -device virtio-net-pci,netdev=u1,disable-legacy=on -initrd image/target/x86_64-unknown-none/release/boot_image
```

## Initial ramdisk

If `initrd.img` is missing, `make` creates the ramdisk using the tool
[`mkinitrd`](mkinitrd). The ramdisk includes all binaries for the architecture `ARCH` from the directory [`data`](data).
The ramdisk is embedded into the boot image at compile time.
`make` does not rebuild an existing `initrd.img`, so delete it after changing the binaries in [`data`](data).

To build these binaries, the repository `hermit-rs` is used with the pull request [`970`](https://github.com/hermit-os/hermit-rs/pull/970).
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
