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
        ],
        &["../../src/libemane/"],
    ).expect("Failed to compile protobuf files!");
}
