//! Gateway HTTP contract tests.

use grokctl_core::config::{GatewaySecret, ResolvedGateway};
use grokctl_core::gateway::{GatewayClient, GatewayClientOptions};
use grokctl_test_support::{MockGateway, MockResponse};
use serde_json::json;
use std::collections::HashMap;
use std::time::Duration;
use url::Url;

#[tokio::test]
async fn health_omits_auth_and_commands_send_required_headers()
-> Result<(), Box<dyn std::error::Error>> {
    let gateway = MockGateway::start(json!({ "rows": [] })).await?;
    let client = client(&gateway.base_url, Some("secret-token"))?;

    let health = client.health().await;
    assert!(health.is_ok(), "health should decode: {health:?}");
    let command = client.command("listAgents", Some(&json!({ "limit": 2 }))).await;
    assert!(command.is_ok(), "command should decode: {command:?}");

    let requests = gateway.requests().await;
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].authorization, None);
    assert_eq!(requests[1].authorization.as_deref(), Some("Bearer secret-token"));
    assert_eq!(requests[1].body, Some(json!({ "limit": 2 })));
    assert_eq!(requests[1].slim_avatars.as_deref(), Some("1"));
    assert!(requests[1].request_id.as_ref().is_some_and(|value| !value.is_empty()));
    Ok(())
}

#[tokio::test]
async fn protected_command_requires_a_token_before_network_io()
-> Result<(), Box<dyn std::error::Error>> {
    let gateway = MockGateway::start(json!({})).await?;
    let client = client(&gateway.base_url, None)?;

    let result = client.command("listAgents", None).await;

    assert!(result.is_err());
    assert!(gateway.requests().await.is_empty());
    Ok(())
}

#[tokio::test]
async fn read_command_retries_transient_server_failures() -> Result<(), Box<dyn std::error::Error>>
{
    let gateway = MockGateway::start_responses(vec![
        MockResponse { status: 503, body: json!({ "error": "try again" }) },
        MockResponse { status: 200, body: json!([]) },
    ])
    .await?;
    let client = client(&gateway.base_url, Some("secret-token"))?;

    let result = client.command("listAgents", None).await;

    assert_eq!(result?, json!([]));
    assert_eq!(gateway.requests().await.len(), 2);
    Ok(())
}

#[tokio::test]
async fn mutation_does_not_retry_a_transient_server_failure()
-> Result<(), Box<dyn std::error::Error>> {
    let gateway = MockGateway::start_responses(vec![
        MockResponse { status: 503, body: json!({ "error": "unknown outcome" }) },
        MockResponse { status: 200, body: json!({ "created": true }) },
    ])
    .await?;
    let client = client(&gateway.base_url, Some("secret-token"))?;

    let result = client.command("createAgent", Some(&json!({ "name": "Ada" }))).await;

    assert!(result.is_err());
    assert_eq!(gateway.requests().await.len(), 1);
    Ok(())
}

#[tokio::test]
async fn authenticated_event_and_avatar_routes_decode() -> Result<(), Box<dyn std::error::Error>> {
    let gateway = MockGateway::start(json!({})).await?;
    let client = client(&gateway.base_url, Some("secret-token"))?;

    let events = client.collect_events(&[], 1, Duration::from_secs(1)).await?;
    let avatar = client.avatar("bot-1").await?;

    assert_eq!(events[0].channel, "agents");
    assert_eq!(avatar.mime_type, "image/png");
    assert_eq!(avatar.bytes, [137, 80, 78, 71]);
    let requests = gateway.requests().await;
    assert_eq!(requests[0].authorization.as_deref(), Some("Bearer secret-token"));
    assert_eq!(requests[1].authorization.as_deref(), Some("Bearer secret-token"));
    Ok(())
}

#[tokio::test]
async fn desktop_network_header_is_sent_on_every_route() -> Result<(), Box<dyn std::error::Error>> {
    let gateway = MockGateway::start(json!({ "rows": [] })).await?;
    let client = client_with_headers(&gateway.base_url, "secret-token", "network-token")?;

    client.health().await?;
    client.command("listAgents", None).await?;

    let requests = gateway.requests().await;
    assert_eq!(requests[0].authorization, None);
    assert_eq!(requests[0].network_token.as_deref(), Some("network-token"));
    assert_eq!(requests[1].authorization.as_deref(), Some("Bearer secret-token"));
    assert_eq!(requests[1].network_token.as_deref(), Some("network-token"));
    Ok(())
}

fn client(
    base_url: &str,
    token: Option<&str>,
) -> Result<GatewayClient, Box<dyn std::error::Error>> {
    let resolved = ResolvedGateway {
        base_url: Url::parse(base_url)?,
        token: token.map(|value| GatewaySecret::new(value.to_owned())),
        headers: HashMap::new(),
        discovery_path: None,
        has_token: token.is_some(),
    };
    Ok(GatewayClient::new(resolved, GatewayClientOptions::default())?)
}

fn client_with_headers(
    base_url: &str,
    token: &str,
    network_token: &str,
) -> Result<GatewayClient, Box<dyn std::error::Error>> {
    let mut headers = HashMap::new();
    headers.insert("x-sand-network-token".to_owned(), GatewaySecret::new(network_token.to_owned()));
    let resolved = ResolvedGateway {
        base_url: Url::parse(base_url)?,
        token: Some(GatewaySecret::new(token.to_owned())),
        headers,
        discovery_path: None,
        has_token: true,
    };
    Ok(GatewayClient::new(resolved, GatewayClientOptions::default())?)
}
