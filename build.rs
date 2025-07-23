fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = std::env::var("OUT_DIR").unwrap();
    tonic_build::configure()
        .file_descriptor_set_path(format!("{}/user_stats.bin", out_dir))
        .compile(&["proto/user_stats.proto"], &["proto"])?;
    Ok(())
}