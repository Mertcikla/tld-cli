use std::env;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let proto_dir = "/Users/mertcikla/apps/diag/backend/proto/diag/v1";
    let include_dir = "/Users/mertcikla/apps/diag/backend/proto";

    let protos: Vec<String> = [
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
    ]
    .iter()
    .map(|f| format!("{}/{}", proto_dir, f))
    .collect();

    // Inform Cargo to rebuild when any proto file changes.
    for proto in &protos {
        println!("cargo:rerun-if-changed={}", proto);
    }

    let out_dir = PathBuf::from(env::var("OUT_DIR")?);

    tonic_prost_build::configure()
        .build_server(false)
        .build_client(true)
        .file_descriptor_set_path(out_dir.join("diag_v1_descriptor.bin"))
        .out_dir(&out_dir)
        .compile_protos(&protos, &[include_dir.to_string()])?;

    Ok(())
}
