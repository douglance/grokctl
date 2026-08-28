//! Command-level policy tests for Grok Bot template mutations.

mod support;

use std::error::Error;
use std::process::Output;

use serde_json::json;
use support::{MockGateway, MockResponse, TestCli};

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn template_mutations_enforce_idempotency_and_unsafe_mode() -> Result<(), Box<dyn Error>> {
    let gateway = gateway(json!({ "ok": true })).await?;
    let cli = TestCli::new()?;

    assert_failure(&cli.run(
        &gateway.base_url,
        &["bot", "template", "add", "share-1", "bot-2", "Name", "circle", "blue", "1"],
    )?);
    for args in destructive_arguments() {
        assert_failure(&cli.run(&gateway.base_url, &args)?);
    }

    assert!(gateway.requests().await.is_empty());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn invalid_visibility_is_rejected_before_network_io() -> Result<(), Box<dyn Error>> {
    let gateway = gateway(json!({ "ok": true })).await?;
    let cli = TestCli::new()?;
    let output = cli.run(
        &gateway.base_url,
        &[
            "bot",
            "template",
            "visibility",
            "share-1",
            "private",
            "--idempotency-key",
            "invalid-visibility-v1",
            "--unsafe-mode",
            "true",
        ],
    )?;

    assert_failure(&output);
    assert!(gateway.requests().await.is_empty());
    Ok(())
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn idempotent_replay_does_not_send_a_second_request() -> Result<(), Box<dyn Error>> {
    let gateway = gateway(json!({ "created": true })).await?;
    let cli = TestCli::new()?;
    let args = [
        "bot",
        "template",
        "add",
        "share-1",
        "bot-2",
        "Name",
        "circle",
        "blue",
        "1",
        "--idempotency-key",
        "replay-add-v1",
    ];

    assert_success(&cli.run(&gateway.base_url, &args)?);
    assert_success(&cli.run(&gateway.base_url, &args)?);

    assert_eq!(gateway.requests().await.len(), 1);
    Ok(())
}

fn destructive_arguments() -> Vec<Vec<&'static str>> {
    vec![
        vec!["bot", "template", "publish", "share-1", "1", "--idempotency-key", "guard-publish-v1"],
        vec![
            "bot",
            "template",
            "visibility",
            "share-1",
            "public",
            "--idempotency-key",
            "guard-visibility-v1",
        ],
        vec!["bot", "template", "delete", "share-1", "--idempotency-key", "guard-delete-v1"],
    ]
}

async fn gateway(body: serde_json::Value) -> Result<MockGateway, Box<dyn Error>> {
    MockGateway::start_responses(vec![MockResponse { status: 200, body }]).await
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "command failed: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_failure(output: &Output) {
    assert!(
        !output.status.success(),
        "command unexpectedly succeeded: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}
