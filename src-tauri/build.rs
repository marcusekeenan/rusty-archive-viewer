fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=tauri.conf.json");
    println!("cargo:rerun-if-changed=proto/archiver.proto");

    // Build Tauri
    tauri_build::build();

    // Compile protobuf definitions
    prost_build::compile_protos(&["proto/archiver.proto"], &["proto/"])
        .unwrap_or_else(|e| panic!("Failed to compile protos: {}", e));
}