use std::env;
use std::fs;
use std::path::PathBuf;

fn main() {
    // Get the path to the library file
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let lib_src = manifest_dir.join("lib/libpdfium.dylib");
    
    // Build the path to the target directory
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let target_dir = out_dir
        .ancestors()
        .nth(3) // Go up from OUT_DIR to target/{profile}/
        .unwrap()
        .to_path_buf();
    
    let lib_dst = target_dir.join("libpdfium.dylib");
    
    // Copy the library to target directory
    if lib_src.exists() {
        fs::copy(&lib_src, &lib_dst).expect("Failed to copy libpdfium.dylib to target directory");
        println!("cargo:rerun-if-changed=lib/libpdfium.dylib");
    }
}
