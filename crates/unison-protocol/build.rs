mod build_certs;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Generate development certificates for embedding
    build_certs::generate_dev_certs()?;

    Ok(())
}
