//! This is the build script for the crankshaft-monitor crate.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .out_dir("src/proto")
        .compile_protos(&["src/proto/monitor.proto"], &["src/generated"])?;
    Ok(())
}
