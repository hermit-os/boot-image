use std::env;
use std::path::PathBuf;

fn main() {
    // Re-link only when the fragment or this script changes.
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=kpti.ld");

    if env::var_os("CARGO_FEATURE_KPTI").is_none() {
        return;
    }

    // Use the absolute path so the linker finds the script regardless of
    // the CWD cargo invokes rustc in.
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let script = PathBuf::from(manifest_dir).join("kpti.ld");
    println!("cargo:rustc-link-arg=-T{}", script.display());
}
