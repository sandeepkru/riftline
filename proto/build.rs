fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_client(true)
        .build_server(true)
        .out_dir("src/generated")
        .compile(&["protos/riftline.proto"], &["protos"])?;
    println!("cargo:rerun-if-changed=protos/riftline.proto");
    Ok(())
}
