fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Build script no longer needed - protobuf support removed
    println!("cargo:rerun-if-changed=build.rs");
    Ok(())
}
