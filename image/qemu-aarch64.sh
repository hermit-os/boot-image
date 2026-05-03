#!/bin/sh
# Cargo runner for aarch64-unknown-none-softfloat: wraps the boot_image binary
# (passed as $1) into the hermit-loader-aarch64 guest-loader. Lives in image/
# because cargo invokes the runner with the working directory set there.
exec qemu-system-aarch64 \
    -display none -serial stdio \
    -kernel hermit-loader-aarch64 \
    -machine virt,gic-version=3 \
    -cpu cortex-a72 \
    -semihosting \
    -smp 1 -m 2G \
    -global virtio-mmio.force-legacy=off \
    -netdev user,id=net0,hostfwd=tcp::9975-:9975,hostfwd=udp::9975-:9975,net=192.168.76.0/24,dhcpstart=192.168.76.9 \
    -device virtio-net-pci,netdev=net0,disable-legacy=on,packed=on,mq=on \
    -device guest-loader,addr=0x48000000,initrd="$1"
