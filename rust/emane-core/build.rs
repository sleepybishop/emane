use std::fs;
use std::path::PathBuf;

fn main() {
    println!("cargo:rustc-link-lib=pcap");
    
    let mut protos = Vec::new();
    for entry in fs::read_dir("../protos").unwrap() {
        let path = entry.unwrap().path();
        if path.extension().unwrap_or_default() == "proto" {
            protos.push(path.to_string_lossy().into_owned());
        }
    }
    
    prost_build::compile_protos(&protos, &["../protos"]).unwrap();
}
