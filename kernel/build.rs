// SPDX-License-Identifier: MIT OR Apache-2.0

use std::env;
use std::path::Path;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let ld = Path::new(&manifest_dir).join("link-qemu_virt.ld");
    println!("cargo:rerun-if-changed={}", ld.display());

    let target = env::var("TARGET").unwrap_or_default();
    if target == "aarch64-unknown-none" {
        println!("cargo:rustc-link-arg=-T{}", ld.display());
    }
}
