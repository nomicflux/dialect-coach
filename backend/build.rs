use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    // Determine library name based on target OS
    let lib_name = if cfg!(target_os = "macos") {
        "libpdfium.dylib"
    } else if cfg!(target_os = "linux") {
        "libpdfium.so"
    } else {
        panic!("Unsupported platform for pdfium");
    };

    let lib_src = manifest_dir.join("lib").join(lib_name);

    // Build the path to the target directory
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let target_dir = out_dir
        .ancestors()
        .nth(3) // Go up from OUT_DIR to target/{profile}/
        .unwrap()
        .to_path_buf();

    let lib_dst = target_dir.join(lib_name);

    // Copy the library to target directory
    if lib_src.exists() {
        fs::copy(&lib_src, &lib_dst)
            .unwrap_or_else(|_| panic!("Failed to copy {} to target directory", lib_name));
        println!("cargo:rerun-if-changed=lib/{}", lib_name);
    }
}
