//! This is the build script for the crankshaft-monitor crate.

fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(true)
        .out_dir("src/proto")
        .type_attribute(
            ".",
            "#[allow(clippy::all, missing_docs, clippy::missing_docs_in_private_items)]",
        )
        .client_mod_attribute(
            ".",
            "#[allow(clippy::all, missing_docs, clippy::missing_docs_in_private_items)]",
        )
        .server_mod_attribute(
            ".",
            "#[allow(clippy::all, missing_docs, clippy::missing_docs_in_private_items)]",
        )
        .compile_protos(&["src/proto/monitor.proto"], &["src/generated"])?;
    Ok(())
}
