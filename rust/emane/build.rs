fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let libemane_path = format!("{}/../../src/libemane/.libs", manifest_dir);
    println!("cargo:rustc-link-search=native={}", libemane_path);
    println!("cargo:rustc-link-lib=dylib=emane");
    
    // Add other required C++ libraries
    println!("cargo:rustc-link-lib=dylib=xml2");
    println!("cargo:rustc-link-lib=dylib=uuid");
    println!("cargo:rustc-link-lib=dylib=stdc++");
}
