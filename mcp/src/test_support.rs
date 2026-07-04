//! Test-only helpers shared by this crate's tests and `runtime`'s tests.
//! Compiled only under `feature = "test-support"` (dev-dependencies only).

use rmcp::{
    ServiceExt,
    model::{ClientCapabilities, ClientInfo, Implementation},
    service::{RoleClient, RunningService},
    transport::{
        StreamableHttpClientTransport, streamable_http_client::StreamableHttpClientTransportConfig,
    },
};

/// Connect an rmcp streamable-HTTP client to `http://{addr}/mcp` with a
/// bearer token (task-6 report: `auth_header` takes the bare token, no
/// "Bearer " prefix). Panics on handshake failure — these are tests.
pub async fn connect(
    addr: impl std::fmt::Display,
    token: &str,
) -> RunningService<RoleClient, ClientInfo> {
    let transport = StreamableHttpClientTransport::from_config(
        StreamableHttpClientTransportConfig::with_uri(format!("http://{addr}/mcp"))
            .auth_header(token),
    );
    let client_info = ClientInfo::new(
        ClientCapabilities::default(),
        Implementation::new("daily-test-client", "0.0.1"),
    );
    client_info
        .serve(transport)
        .await
        .expect("client handshake")
}
