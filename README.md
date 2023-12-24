= boot-image

`boot-image` allows to boot HermitOS` kernel as common monolithic kernel.

## Requirements

* [`rustup`](https://www.rust-lang.org/tools/install)

## Building the kernel

```sh
cargo build
```

## Booting the kernel

```sh
qemu-system-x86_64 -display none -serial stdio -kernel rusty-loader-x86_64 -cpu Skylake-Client -device isa-debug-exit,iobase=0xf4,iosize=0x04 -initrd target/x86_64-unknown-none/debug/boot_image -smp 1 -m 512M
```

## License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

The kernel is being developed on [hermit-os/kernel](https://github.com/hermit-os/kernel).
Create your own fork, send us a pull request, and chat with us on [Zulip](https://hermit.zulipchat.com/).
