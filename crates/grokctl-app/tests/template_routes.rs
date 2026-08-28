//! Command-level contract tests for Grok Bot template gateway routes.

mod support;

use std::error::Error;
use std::process::Output;

use serde_json::{Value, json};
use support::{MockGateway, MockResponse, TestCli};

type ExpectedRequest = (String, Option<Value>);

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn template_commands_send_the_observed_gateway_contract() -> Result<(), Box<dyn Error>> {
    let responses =
        (0..7).map(|_| MockResponse { status: 200, body: json!({ "ok": true }) }).collect();
    let gateway = MockGateway::start_responses(responses).await?;
    let cli = TestCli::new()?;

    for args in route_arguments() {
        assert_success(&cli.run(&gateway.base_url, &args)?);
    }

    let actual = gateway
        .requests()
        .await
        .into_iter()
        .map(|request| (request.path, request.body))
        .collect::<Vec<_>>();
    assert_eq!(actual, expected_requests());
    Ok(())
}

fn route_arguments() -> Vec<Vec<&'static str>> {
    vec![
        vec!["bot", "template", "list"],
        vec!["bot", "template", "version", "share-1", "3"],
        vec!["bot", "template", "for-bot", "bot-1"],
        add_arguments(),
        publish_arguments(),
        visibility_arguments(),
        delete_arguments(),
    ]
}

fn add_arguments() -> Vec<&'static str> {
    vec![
        "bot",
        "template",
        "add",
        "share-1",
        "bot-2",
        "Release Captain",
        "circle",
        "blue",
        "3",
        "--idempotency-key",
        "routes-add-v1",
    ]
}

fn publish_arguments() -> Vec<&'static str> {
    vec![
        "bot",
        "template",
        "publish",
        "share-1",
        "3",
        "--idempotency-key",
        "routes-publish-v1",
        "--unsafe-mode",
        "true",
    ]
}

fn visibility_arguments() -> Vec<&'static str> {
    vec![
        "bot",
        "template",
        "visibility",
        "share-1",
        "public",
        "--idempotency-key",
        "routes-visibility-v1",
        "--unsafe-mode",
        "true",
    ]
}

fn delete_arguments() -> Vec<&'static str> {
    vec![
        "bot",
        "template",
        "delete",
        "share-1",
        "--idempotency-key",
        "routes-delete-v1",
        "--unsafe-mode",
        "true",
    ]
}

fn expected_requests() -> Vec<ExpectedRequest> {
    vec![
        request("listBotTemplates", None),
        request("getBotTemplateVersion", Some(json!({ "shareId": "share-1", "version": 3 }))),
        request("getBotTemplateForSourceAgent", Some(json!({ "sourceAgentId": "bot-1" }))),
        request(
            "createAgentFromTemplate",
            Some(json!({
                "shareId": "share-1",
                "agentId": "bot-2",
                "name": "Release Captain",
                "avatarShape": "circle",
                "avatarColor": "blue",
                "expectedActiveVersion": 3,
            })),
        ),
        request("publishBotTemplate", Some(json!({ "shareId": "share-1", "version": 3 }))),
        request(
            "setBotTemplateVisibility",
            Some(json!({ "shareId": "share-1", "visibility": "public" })),
        ),
        request("deleteBotTemplate", Some(json!({ "shareId": "share-1" }))),
    ]
}

fn request(command: &str, body: Option<Value>) -> ExpectedRequest {
    (format!("/api/{command}"), body)
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "command failed: {}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
