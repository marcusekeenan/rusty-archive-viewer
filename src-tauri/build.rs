use std::io::Result;

fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=tauri.conf.json");
    println!("cargo:rerun-if-changed=src/archiver.proto");

    tauri_build::build();

    let mut config = prost_build::Config::new();
    config
        .type_attribute(".", "#[derive(serde::Serialize)]")
        .field_attribute(".*.fields", "#[serde(default)]")
        .field_attribute(".*.headers", "#[serde(default)]")
        .out_dir("src");

    config.compile_protos(&["src/archiver.proto"], &["src"])?;

    Ok(())
}
