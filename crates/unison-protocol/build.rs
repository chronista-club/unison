mod build_certs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Generate development certificates for embedding
    build_certs::generate_dev_certs()?;

    // Compile .proto files with buffa
    buffa_build::Config::new()
        .files(&["proto/creo_sync.proto"])
        .includes(&["proto/"])
        .compile()?;

    Ok(())
}
