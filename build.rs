use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // In CI, set TLD_PROTO_PATH to the cloned tld-proto repo root.
    // Locally, defaults to the sibling backend/proto directory.
    let include_dir = PathBuf::from(
        env::var("TLD_PROTO_PATH")
            .unwrap_or_else(|_| "/Users/mertcikla/apps/diag/backend/proto".to_string()),
    );
    let proto_dir = include_dir.join("diag/v1");

    let proto_files = [
        "auth_service.proto",
        "billing_service.proto",
        "dependency_service.proto",
        "device_service.proto",
        "explore_service.proto",
        "import_service.proto",
        "notification_service.proto",
        "org_service.proto",
        "user_service.proto",
        "workspace_service.proto",
        "workspace_version_service.proto",
    ];

    let protos: Vec<String> = proto_files
        .iter()
        .map(|f| proto_dir.join(f).to_string_lossy().into_owned())
        .collect();

    for proto in &protos {
        println!("cargo:rerun-if-changed={}", proto);
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR")?);

    tonic_prost_build::configure()
        .build_server(false)
        .build_client(true)
        .file_descriptor_set_path(out_dir.join("diag_v1_descriptor.bin"))
        .out_dir(&out_dir)
        .compile_protos(&protos, &[include_dir.to_string_lossy().into_owned()])?;

    Ok(())
}
