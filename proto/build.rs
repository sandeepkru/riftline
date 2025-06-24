fn main() -> Result<(), Box<dyn std::error::Error>> {
    let protoc = protoc_bin_vendored::protoc_bin_path()?;
    unsafe {
        std::env::set_var("PROTOC", protoc);
    }

    tonic_build::configure()
        .build_client(true)
        .build_server(true)
        .out_dir("src/generated")
        .compile(&["protos/riftline.proto"], &["protos"])?;
    println!("cargo:rerun-if-changed=protos/riftline.proto");
    Ok(())
}
