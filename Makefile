ARCH ?= x86_64
DISK_IMG ?= disk.img
DISK_SIZE_MB ?= 128
DISK_SRC ?= data

ifeq ($(ARCH),x86_64)
    TARGET = x86_64-unknown-none
    # x86_64 uses isa-debug-exit, which returns 3 on a clean shutdown
    EXPECTED_EXIT = 3
else ifeq ($(ARCH),aarch64)
    TARGET = aarch64-unknown-none-softfloat
    # aarch64 uses semihosting shutdown, which returns 0
    EXPECTED_EXIT = 0
else ifeq ($(ARCH),riscv64)
    TARGET = riscv64gc-unknown-none-elf
    # riscv64 shuts down via SBI system reset, which returns 0 (no
    # semihosting feature — QEMU runs without semihosting support).
    # PCI and the network stack are not implemented on riscv64 yet,
    # so the image is built without them.
    CARGO_FLAGS = --no-default-features --features loader,fork,kernel-stack,common-os,virtio-blk
    EXPECTED_EXIT = 0
else
    $(error Unsupported ARCH "$(ARCH)" — use x86_64, aarch64, or riscv64)
endif

build:
	cargo build --target $(TARGET) $(CARGO_FLAGS) --release

run: $(DISK_IMG)
	cargo run --target $(TARGET) $(CARGO_FLAGS) --release; \
	rc=$$?; \
	if [ $$rc -ne $(EXPECTED_EXIT) ]; \
	then \
		echo "cargo failed $$rc (expected $(EXPECTED_EXIT))"; \
		exit 1; \
	else \
		exit 0; \
	fi

# FAT-formatted disk image with the content of $(DISK_SRC)/$(ARCH).
# Uses mtools if available (Linux and macOS). Without mtools, macOS falls
# back to hdiutil and newfs_msdos.

# .disk-arch records the architecture of the current disk image. On a change
# of ARCH, the image is removed while the Makefile is parsed. Timestamps are
# not reliable here: QEMU writes to the image, and make 3.81 compares them in
# seconds only.
ifneq ($(shell cat .disk-arch 2>/dev/null),$(ARCH))
$(shell rm -f $(DISK_IMG); echo $(ARCH) > .disk-arch)
endif

.PHONY: build run clean

$(DISK_IMG): $(shell find $(DISK_SRC)/$(ARCH) -type f 2>/dev/null)
	@set -e; \
	rm -f $@; \
	trap 'rc=$$?; [ $$rc -eq 0 ] || rm -f $@' EXIT; \
	if command -v mformat >/dev/null 2>&1 && command -v mcopy >/dev/null 2>&1; then \
		mformat -C -i $@ -T $$(( $(DISK_SIZE_MB) * 2048 )) -F -v HERMIT ::; \
		mcopy -s -i $@ $(DISK_SRC)/$(ARCH)/* ::/; \
	elif [ "$$(uname -s)" = Darwin ]; then \
		dd if=/dev/zero of=$@ bs=1048576 count=$(DISK_SIZE_MB) 2>/dev/null; \
		dev=$$(hdiutil attach -nomount -imagekey diskimage-class=CRawDiskImage $@ 2>/dev/null | awk 'NR==1 {print $$1}'); \
		mnt=$$(mktemp -d); \
		trap 'rc=$$?; diskutil unmount "$$dev" >/dev/null 2>&1 || true; hdiutil detach "$$dev" >/dev/null 2>&1 || true; rmdir "$$mnt" 2>/dev/null || true; [ $$rc -eq 0 ] || rm -f $@' EXIT; \
		newfs_msdos -F 32 -v HERMIT "$$dev" >/dev/null; \
		diskutil mount -mountPoint "$$mnt" "$$dev" >/dev/null; \
		COPYFILE_DISABLE=1 cp -R $(DISK_SRC)/$(ARCH)/. "$$mnt"/; \
		find "$$mnt" -name '._*' -delete; \
		rm -rf "$$mnt"/.fseventsd "$$mnt"/.Trashes; \
	else \
		echo "mtools (mformat, mcopy) is required to create $@" >&2; \
		exit 1; \
	fi; \
	echo "Created $@ ($(DISK_SIZE_MB) MiB, FAT32) from $(DISK_SRC)/$(ARCH)"

clean:
	cargo clean
	rm -rf $(DISK_IMG) .disk-arch
