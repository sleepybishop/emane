use std::env;
use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-changed=wrapper.h");

    let bindings = bindgen::Builder::default()
        .header("wrapper.h")
        // Clang arguments to find EMANE headers
        .clang_arg("-I../../include")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        // Since EMANE is C++, we must enable C++ support
        .clang_arg("-xc++")
        .clang_arg("-std=c++17")
        .clang_arg("-I/usr/include/uuid")
        .clang_arg("-I/usr/include/libxml2")
        .allowlist_type("EMANE::.*")
        .allowlist_function("EMANE::.*")
        .allowlist_var("EMANE::.*")
        .opaque_type("std::.*")
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());
    bindings
        .write_to_file(out_path.join("bindings.rs"))
        .expect("Couldn't write bindings!");

    let mut prost_config = prost_build::Config::new();
    prost_config.compile_protos(
        &[
            "../../src/libemane/antennaprofileevent.proto",
            "../../src/libemane/commeffectevent.proto",
            "../../src/libemane/commonmacheader.proto",
            "../../src/libemane/commonphyheader.proto",
            "../../src/libemane/event.proto",
            "../../src/libemane/fadingselectionevent.proto",
            "../../src/libemane/flowcontrol.proto",
            "../../src/libemane/locationevent.proto",
            "../../src/libemane/loggermessage.proto",
            "../../src/libemane/otaheader.proto",
            "../../src/libemane/otatransmitter.proto",
            "../../src/libemane/pathlossevent.proto",
            "../../src/libemane/pathlossexevent.proto",
            "../../src/libemane/radiotorouter.proto",
            "../../src/libemane/remotecontrolportapi.proto",
            "../../src/libemane/tdmascheduleevent.proto",
            "../../src/models/mac/tdma/tdmabasemodelmessage.proto",
            "../../src/models/mac/rfpipe/rfpipemacheader.proto",
        ],
        &["../../src/libemane/", "../../src/models/mac/tdma/", "../../src/models/mac/rfpipe/"],
    ).expect("Failed to compile protobuf files!");
    
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let emane_root = std::fs::canonicalize(PathBuf::from(manifest_dir).join("../../")).unwrap();
    let emane_root_str = emane_root.to_str().unwrap();

        
    println!("cargo:rustc-link-search=native={}/src/libemane/.libs", emane_root_str);
    println!("cargo:rustc-link-search=native={}/src/models/mac/ieee80211abg/.libs", emane_root_str);
    println!("cargo:rustc-link-search=native={}/src/models/mac/tdma/.libs", emane_root_str);
    println!("cargo:rustc-link-search=native={}/src/models/mac/rfpipe/.libs", emane_root_str);
    println!("cargo:rustc-link-search=native={}/src/models/mac/bentpipe/.libs", emane_root_str);
    println!("cargo:rustc-link-lib=dylib=emane");

    cc::Build::new().file("src/tdma_stubs.c").file("src/ieee80211abg_stubs.c").file("src/bentpipe_stubs.c").compile("tdma_stubs");
}
