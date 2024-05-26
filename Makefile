build:
	cd image; cargo build --release

run: initrd.img
	cd image; cargo run --release

initrd.img:
	cd mkinitrd; cargo build --release
	mkinitrd/target/release/mkinitrd create data

clean:
	cd image; cargo clean
	cd mkinitrd; cargo clean
	rm -rf initrd.img
