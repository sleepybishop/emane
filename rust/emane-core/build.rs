use std::fs;

fn main() {
    let mut protos = Vec::new();
    for entry in fs::read_dir("../protos").unwrap() {
        let path = entry.unwrap().path();
        if path.extension().unwrap_or_default() == "proto" {
            println!("cargo:rerun-if-changed={}", path.display());
            protos.push(path.to_string_lossy().into_owned());
        }
    }
    protos.sort();

    let descriptors = protox::compile(&protos, ["../protos"]).unwrap();
    prost_build::compile_fds(descriptors).unwrap();
}
