fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Tell Cargo to re-run this script if the proto files change
    println!("cargo:rerun-if-changed=packet.proto");
    println!("cargo:rerun-if-changed=packet.options");

    // Get the OUT_DIR environment variable
    let out_dir = std::env::var("OUT_DIR")?;
    println!("OUT_DIR is: {}", out_dir);

    // Use the prost_build crate to compile the proto file
    prost_build::compile_protos(&["src/packet.proto"], &["src/"])?;

    Ok(())
}
