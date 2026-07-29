// build.rs
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_root = PathBuf::from("proto");
    let proto_files = [
        "proto/tapcommon.proto",
        "proto/taprootassets.proto",
        "proto/assetwalletrpc/assetwallet.proto",
        "proto/universerpc/universe.proto",
    ];

    tonic_build::configure()
        .build_client(true)
        .build_server(false)
        .compile_protos(&proto_files, &[&proto_root])?;

    Ok(())
}
