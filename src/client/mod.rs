pub use diag_proto::diagv1;

use crate::error::TldError;
use prost::Message;
use reqwest::header::{ACCEPT, CONTENT_TYPE, HeaderMap, HeaderValue, USER_AGENT};
use serde::Deserialize;

const CONNECT_PROTOCOL_VERSION_HEADER: &str = "Connect-Protocol-Version";
const CONNECT_PROTOCOL_VERSION: &str = "1";
const CONNECT_CONTENT_TYPE: &str = "application/proto";
const CONNECT_ACCEPT: &str = "application/proto, application/json";

pub type AuthedWorkspaceClient = WorkspaceClient;

/// Normalises a server URL to include the `/api` suffix.
pub fn normalize_url(server_url: &str) -> String {
    let base = server_url.trim_end_matches('/');
    if base.ends_with("/api") {
        base.to_string()
    } else {
        format!("{base}/api")
    }
}

/// Resolves the full API base URL, preserving the caller's scheme when present.
fn resolved_server_url(server_url: &str) -> String {
    let base = normalize_url(server_url);
    if base.starts_with("http://") || base.starts_with("https://") {
        base
    } else {
        format!("https://{base}")
    }
}

fn default_headers() -> HeaderMap {
    let mut headers = HeaderMap::new();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static(CONNECT_CONTENT_TYPE));
    headers.insert(ACCEPT, HeaderValue::from_static(CONNECT_ACCEPT));
    headers.insert(
        CONNECT_PROTOCOL_VERSION_HEADER,
        HeaderValue::from_static(CONNECT_PROTOCOL_VERSION),
    );
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static(concat!("tld/", env!("CARGO_PKG_VERSION"))),
    );
    headers
}

#[derive(Clone)]
struct ConnectTransport {
    http: reqwest::Client,
    base_url: String,
    bearer_token: Option<String>,
}

impl ConnectTransport {
    fn new(server_url: &str, bearer_token: Option<String>) -> Result<Self, TldError> {
        let http = reqwest::Client::builder()
            .default_headers(default_headers())
            .build()
            .map_err(|e| TldError::Generic(format!("Failed to build HTTP client: {e}")))?;

        Ok(Self {
            http,
            base_url: resolved_server_url(server_url),
            bearer_token,
        })
    }

    fn rpc_url(&self, service: &str, method: &str) -> String {
        format!("{}/{service}/{method}", self.base_url.trim_end_matches('/'))
    }

    async fn unary<Req, Resp>(
        &self,
        service: &str,
        method: &str,
        request: Req,
    ) -> Result<Resp, TldError>
    where
        Req: Message,
        Resp: Message + Default,
    {
        let mut http_request = self
            .http
            .post(self.rpc_url(service, method))
            .body(request.encode_to_vec());

        if let Some(token) = &self.bearer_token {
            http_request = http_request.bearer_auth(token);
        }

        let response = http_request
            .send()
            .await
            .map_err(|e| TldError::Generic(format!("Request failed: {e}")))?;

        let status = response.status();
        let body = response
            .bytes()
            .await
            .map_err(|e| TldError::Generic(format!("Failed to read response body: {e}")))?;

        if !status.is_success() {
            return Err(connect_error(status, &body));
        }

        Resp::decode(body.as_ref()).map_err(|e| {
            TldError::Generic(format!("Failed to decode {service}/{method} response: {e}"))
        })
    }
}

#[derive(Debug, Deserialize)]
struct ConnectErrorBody {
    code: Option<String>,
    message: Option<String>,
}

fn connect_error(status: reqwest::StatusCode, body: &[u8]) -> TldError {
    let parsed = serde_json::from_slice::<ConnectErrorBody>(body).ok();
    let code = parsed
        .as_ref()
        .and_then(|payload| payload.code.as_deref())
        .unwrap_or("unknown");
    let message = parsed
        .as_ref()
        .and_then(|payload| payload.message.as_deref())
        .unwrap_or_else(|| std::str::from_utf8(body).unwrap_or("request failed"));

    if status == reqwest::StatusCode::UNAUTHORIZED
        || status == reqwest::StatusCode::FORBIDDEN
        || code.eq_ignore_ascii_case("unauthenticated")
    {
        TldError::Auth(message.to_string())
    } else {
        TldError::Generic(format!("RPC failed: {message} ({status}, {code})"))
    }
}

#[derive(Clone)]
pub struct WorkspaceClient {
    transport: ConnectTransport,
}

impl WorkspaceClient {
    pub async fn apply_workspace_plan(
        &mut self,
        request: diagv1::ApplyPlanRequest,
    ) -> Result<diagv1::ApplyPlanResponse, TldError> {
        self.transport
            .unary("diag.v1.WorkspaceService", "ApplyWorkspacePlan", request)
            .await
    }

    pub async fn export_workspace(
        &mut self,
        request: diagv1::ExportOrganizationRequest,
    ) -> Result<diagv1::ExportOrganizationResponse, TldError> {
        self.transport
            .unary("diag.v1.WorkspaceService", "ExportWorkspace", request)
            .await
    }

    pub async fn get_workspace(
        &mut self,
        request: diagv1::GetWorkspaceRequest,
    ) -> Result<diagv1::GetWorkspaceResponse, TldError> {
        self.transport
            .unary("diag.v1.WorkspaceService", "GetWorkspace", request)
            .await
    }

    pub async fn list_elements(
        &mut self,
        request: diagv1::ListElementsRequest,
    ) -> Result<diagv1::ListElementsResponse, TldError> {
        self.transport
            .unary("diag.v1.WorkspaceService", "ListElements", request)
            .await
    }

    pub async fn list_element_placements(
        &mut self,
        request: diagv1::ListElementPlacementsRequest,
    ) -> Result<diagv1::ListElementPlacementsResponse, TldError> {
        self.transport
            .unary("diag.v1.WorkspaceService", "ListElementPlacements", request)
            .await
    }
}

#[derive(Clone)]
pub struct DeviceClient {
    transport: ConnectTransport,
}

impl DeviceClient {
    pub async fn authorize(
        &mut self,
        request: diagv1::DeviceAuthorizeRequest,
    ) -> Result<diagv1::DeviceAuthorizeResponse, TldError> {
        self.transport
            .unary("diag.v1.DeviceService", "Authorize", request)
            .await
    }

    pub async fn poll_token(
        &mut self,
        request: diagv1::DevicePollTokenRequest,
    ) -> Result<diagv1::DevicePollTokenResponse, TldError> {
        self.transport
            .unary("diag.v1.DeviceService", "PollToken", request)
            .await
    }
}

#[derive(Clone)]
pub struct OrgClient {
    transport: ConnectTransport,
}

impl OrgClient {
    pub async fn update_tag(
        &mut self,
        request: diagv1::UpdateTagRequest,
    ) -> Result<diagv1::UpdateTagResponse, TldError> {
        self.transport
            .unary("diag.v1.OrgService", "UpdateTag", request)
            .await
    }
}

/// Creates a WorkspaceService client with bearer-token authentication.
pub fn new_workspace_client(
    server_url: &str,
    api_key: &str,
) -> Result<AuthedWorkspaceClient, TldError> {
    Ok(WorkspaceClient {
        transport: ConnectTransport::new(server_url, Some(api_key.to_string()))?,
    })
}

/// Creates a DeviceService client for device-flow authentication.
pub fn new_device_client(server_url: &str) -> Result<DeviceClient, TldError> {
    Ok(DeviceClient {
        transport: ConnectTransport::new(server_url, None)?,
    })
}

/// Creates an OrgService client with bearer-token authentication.
pub fn new_org_client(server_url: &str, api_key: &str) -> Result<OrgClient, TldError> {
    Ok(OrgClient {
        transport: ConnectTransport::new(server_url, Some(api_key.to_string()))?,
    })
}

#[cfg(test)]
mod tests {
    use super::{ConnectTransport, normalize_url, resolved_server_url};

    #[test]
    fn normalize_url_preserves_explicit_http_scheme() {
        assert_eq!(
            normalize_url("http://localhost:808"),
            "http://localhost:808/api"
        );
    }

    #[test]
    fn normalize_url_preserves_explicit_https_scheme() {
        assert_eq!(
            normalize_url("https://example.com"),
            "https://example.com/api"
        );
    }

    #[test]
    fn normalize_url_adds_api_to_scheme_less_urls() {
        assert_eq!(normalize_url("example.com"), "example.com/api");
    }

    #[test]
    fn resolved_server_url_adds_https_for_scheme_less_urls() {
        assert_eq!(
            resolved_server_url("example.com"),
            "https://example.com/api"
        );
    }

    #[test]
    fn rpc_url_joins_service_and_method() {
        let transport = ConnectTransport::new("https://tldiagram.com", Some("secret".to_string()))
            .expect("transport should build");
        assert_eq!(
            transport.rpc_url("diag.v1.WorkspaceService", "ApplyWorkspacePlan"),
            "https://tldiagram.com/api/diag.v1.WorkspaceService/ApplyWorkspacePlan"
        );
    }
}
