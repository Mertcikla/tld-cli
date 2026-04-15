/// Module containing all gRPC protobuf stubs generated from
/// `/Users/mertcikla/apps/diag/backend/proto/diag/v1`.
pub mod proto {
    pub mod diag {
        pub mod v1 {
            #![allow(clippy::all)]
            tonic::include_proto!("diag.v1");
        }
    }
}

pub use proto::diag::v1 as diagv1;

use crate::error::TldError;
use std::str::FromStr;
use tonic::{
    Request, Status,
    metadata::{Ascii, MetadataValue},
    service::interceptor::{InterceptedService, Interceptor},
    transport::Channel,
};

use diagv1::{
    device_service_client::DeviceServiceClient, workspace_service_client::WorkspaceServiceClient,
};

/// Normalises a server URL to include the `/api` suffix.
pub fn normalize_url(server_url: &str) -> String {
    let base = server_url.trim_end_matches('/');
    if base.ends_with("/api") {
        base.to_string()
    } else {
        format!("{}/api", base)
    }
}

/// Build a gRPC channel to the given base URL.
pub async fn connect_channel(server_url: &str) -> Result<Channel, TldError> {
    let endpoint = format!("https://{}", server_url.trim_start_matches("https://"));
    let channel = Channel::from_shared(endpoint)
        .map_err(|e| TldError::Generic(e.to_string()))?
        .connect()
        .await?;
    Ok(channel)
}

#[derive(Clone)]
pub struct AuthInterceptor {
    token: MetadataValue<Ascii>,
}

impl Interceptor for AuthInterceptor {
    fn call(&mut self, mut request: Request<()>) -> Result<Request<()>, Status> {
        request
            .metadata_mut()
            .insert("authorization", self.token.clone());
        Ok(request)
    }
}

/// Creates a WorkspaceServiceClient with bearer-token authentication.
pub async fn new_workspace_client(
    server_url: &str,
    api_key: &str,
) -> Result<WorkspaceServiceClient<InterceptedService<Channel, AuthInterceptor>>, TldError> {
    let channel = connect_channel(&normalize_url(server_url)).await?;
    let token = MetadataValue::from_str(&format!("Bearer {}", api_key))
        .map_err(|e| TldError::Generic(e.to_string()))?;

    let interceptor = AuthInterceptor { token };
    let client = WorkspaceServiceClient::with_interceptor(channel, interceptor);
    Ok(client)
}

/// Creates a DeviceServiceClient (no auth required for device flow).
pub async fn new_device_client(server_url: &str) -> Result<DeviceServiceClient<Channel>, TldError> {
    let channel = connect_channel(&normalize_url(server_url)).await?;
    let client = DeviceServiceClient::new(channel);
    Ok(client)
}
