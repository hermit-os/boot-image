ARCH ?= x86_64

ifeq ($(ARCH),x86_64)
    TARGET = x86_64-unknown-none
    # x86_64 uses isa-debug-exit, which returns 3 on a clean shutdown
    EXPECTED_EXIT = 3
else ifeq ($(ARCH),aarch64)
    TARGET = aarch64-unknown-none-softfloat
    # aarch64 uses semihosting shutdown, which returns 0
    EXPECTED_EXIT = 0
else
    $(error Unsupported ARCH "$(ARCH)" — use x86_64 or aarch64)
endif

build:
	cd image; cargo build --target $(TARGET) --release

run: initrd.img
	cd image; cargo run --target $(TARGET) --release; \
	rc=$$?; \
	if [ $$rc -ne $(EXPECTED_EXIT) ]; \
	then \
		echo "cargo failed $$rc (expected $(EXPECTED_EXIT))"; \
		exit 1; \
	else \
		exit 0; \
	fi

initrd.img:
	cd mkinitrd; cargo build --release
	mkinitrd/target/release/mkinitrd create data/$(ARCH)

clean:
	cd image; cargo clean
	cd mkinitrd; cargo clean
	rm -rf initrd.img
