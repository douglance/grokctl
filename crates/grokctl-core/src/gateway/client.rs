//! HTTP gateway client.

use std::time::Duration;

use reqwest::redirect::Policy;
use serde::de::DeserializeOwned;
use serde_json::Value;
use tokio::time::sleep;
use uuid::Uuid;

use grokctl_manifest::{CommandEffect, classify_command};

use crate::config::ResolvedGateway;
use crate::domain::HealthResponse;

use super::GatewayError;

/// HTTP client configuration.
#[derive(Clone, Debug)]
pub struct GatewayClientOptions {
    /// Unary request timeout.
    pub timeout: Duration,
    /// Ask the host to remove inline avatar data from summary rows.
    pub slim_avatars: bool,
    /// Maximum attempts for read-only transient failures.
    pub max_read_attempts: u8,
    /// Delay between read-only retry attempts.
    pub retry_delay: Duration,
}

impl Default for GatewayClientOptions {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            slim_avatars: true,
            max_read_attempts: 3,
            retry_delay: Duration::from_millis(100),
        }
    }
}

/// Typed client for a running Grok Bot Sand gateway.
#[derive(Clone, Debug)]
pub struct GatewayClient {
    gateway: ResolvedGateway,
    http: reqwest::Client,
    options: GatewayClientOptions,
}

struct GatewayRequest<'a> {
    method: reqwest::Method,
    path: String,
    command: &'a str,
    body: Option<&'a Value>,
    auth: bool,
    retry: bool,
}

impl GatewayClient {
    /// Construct a client with redirects disabled.
    /// # Errors
    /// Returns [`GatewayError`] if the HTTP client cannot be constructed.
    pub fn new(
        gateway: ResolvedGateway,
        options: GatewayClientOptions,
    ) -> Result<Self, GatewayError> {
        let http = reqwest::Client::builder()
            .redirect(Policy::none())
            .timeout(options.timeout)
            .build()
            .map_err(|error| transport_error("client", "", &error.to_string(), &gateway))?;
        Ok(Self { gateway, http, options })
    }

    /// Read unauthenticated host health.
    /// # Errors
    /// Returns [`GatewayError`] for transport, status, or JSON failures.
    pub async fn health(&self) -> Result<HealthResponse, GatewayError> {
        let value = self
            .request(GatewayRequest {
                method: reqwest::Method::GET,
                path: "/health".to_owned(),
                command: "health",
                body: None,
                auth: false,
                retry: true,
            })
            .await?;
        serde_json::from_value(value).map_err(|error| GatewayError::Json {
            command: "health".to_owned(),
            request_id: String::new(),
            message: error.to_string(),
        })
    }

    /// Call `POST /api/<command>` with an optional JSON body.
    /// # Errors
    /// Returns [`GatewayError`] for missing auth, transport, status, or JSON failures.
    pub async fn command(
        &self,
        command: &str,
        body: Option<&Value>,
    ) -> Result<Value, GatewayError> {
        self.request(GatewayRequest {
            method: reqwest::Method::POST,
            path: format!("/api/{command}"),
            command,
            body,
            auth: true,
            retry: classify_command(command).effect == CommandEffect::Read,
        })
        .await
    }

    /// Call a host command and deserialize its typed response.
    /// # Errors
    /// Returns [`GatewayError`] for gateway or response decoding failures.
    pub async fn command_typed<T: DeserializeOwned>(
        &self,
        command: &str,
        body: Option<&Value>,
    ) -> Result<T, GatewayError> {
        let value = self.command(command, body).await?;
        serde_json::from_value(value).map_err(|error| GatewayError::Json {
            command: command.to_owned(),
            request_id: String::new(),
            message: redact(&error.to_string(), &self.gateway),
        })
    }

    async fn request(&self, spec: GatewayRequest<'_>) -> Result<Value, GatewayError> {
        let mut attempt = 1;
        loop {
            let result = self.request_once(&spec).await;
            match retry_delay(&result, &spec, attempt, &self.options) {
                Some(delay) => sleep(delay).await,
                None => return result,
            }
            attempt += 1;
        }
    }

    async fn request_once(&self, spec: &GatewayRequest<'_>) -> Result<Value, GatewayError> {
        let request_id = Uuid::new_v4().to_string();
        let url = self.gateway.base_url.join(&spec.path).map_err(|error| {
            transport_error(spec.command, &request_id, &error.to_string(), &self.gateway)
        })?;
        let mut request =
            self.http.request(spec.method.clone(), url).header("x-sand-request-id", &request_id);
        if self.options.slim_avatars {
            request = request.header("x-sand-slim-avatars", "1");
        }
        request = apply_gateway_headers(request, &self.gateway);
        if spec.auth {
            let token = self
                .gateway
                .token
                .as_ref()
                .ok_or_else(|| GatewayError::MissingToken(spec.command.to_owned()))?;
            request = request.bearer_auth(token.expose());
        }
        if let Some(value) = spec.body {
            request = request.json(value);
        }
        let response = request.send().await.map_err(|error| {
            transport_error(spec.command, &request_id, &error.to_string(), &self.gateway)
        })?;
        read_response(response, spec.command, &request_id, &self.gateway).await
    }

    pub(super) async fn protected_get(
        &self,
        path: &str,
        command: &str,
        accept: &str,
    ) -> Result<(reqwest::Response, String), GatewayError> {
        let request_id = Uuid::new_v4().to_string();
        let url = self.gateway.base_url.join(path).map_err(|error| {
            transport_error(command, &request_id, &error.to_string(), &self.gateway)
        })?;
        let token = self
            .gateway
            .token
            .as_ref()
            .ok_or_else(|| GatewayError::MissingToken(command.to_owned()))?;
        let request = self
            .http
            .get(url)
            .header("x-sand-request-id", &request_id)
            .header("x-sand-slim-avatars", "1")
            .header("accept", accept)
            .bearer_auth(token.expose());
        let response =
            apply_gateway_headers(request, &self.gateway).send().await.map_err(|error| {
                transport_error(command, &request_id, &error.to_string(), &self.gateway)
            })?;
        if response.status().is_success() {
            return Ok((response, request_id));
        }
        match read_response(response, command, &request_id, &self.gateway).await {
            Err(error) => Err(error),
            Ok(_) => Err(GatewayError::Json {
                command: command.to_owned(),
                request_id,
                message: "non-success response had no error".to_owned(),
            }),
        }
    }
}

fn retry_delay(
    result: &Result<Value, GatewayError>,
    spec: &GatewayRequest<'_>,
    attempt: u8,
    options: &GatewayClientOptions,
) -> Option<Duration> {
    let transient = result.as_ref().err().is_some_and(is_transient);
    (spec.retry && transient && attempt < options.max_read_attempts).then_some(options.retry_delay)
}

const fn is_transient(error: &GatewayError) -> bool {
    matches!(error, GatewayError::Transport { .. } | GatewayError::Status { status: 500..=599, .. })
}

async fn read_response(
    response: reqwest::Response,
    command: &str,
    request_id: &str,
    gateway: &ResolvedGateway,
) -> Result<Value, GatewayError> {
    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|error| transport_error(command, request_id, &error.to_string(), gateway))?;
    if !status.is_success() {
        let message = gateway_message(&text);
        return Err(GatewayError::Status {
            status: status.as_u16(),
            command: command.to_owned(),
            request_id: request_id.to_owned(),
            message: redact(&message, gateway),
        });
    }
    if text.is_empty() {
        return Ok(Value::Null);
    }
    serde_json::from_str(&text).map_err(|error| GatewayError::Json {
        command: command.to_owned(),
        request_id: request_id.to_owned(),
        message: redact(&error.to_string(), gateway),
    })
}

fn gateway_message(text: &str) -> String {
    serde_json::from_str::<Value>(text)
        .ok()
        .and_then(|value| value.get("error")?.as_str().map(str::to_owned))
        .unwrap_or_else(|| text.to_owned())
}

fn transport_error(
    command: &str,
    request_id: &str,
    message: &str,
    gateway: &ResolvedGateway,
) -> GatewayError {
    GatewayError::Transport {
        command: command.to_owned(),
        request_id: request_id.to_owned(),
        message: redact(message, gateway),
    }
}

fn redact(message: &str, gateway: &ResolvedGateway) -> String {
    let redacted = gateway
        .token
        .as_ref()
        .map_or_else(|| message.to_owned(), |token| message.replace(token.expose(), "[redacted]"));
    gateway
        .headers
        .values()
        .fold(redacted, |text, secret| text.replace(secret.expose(), "[redacted]"))
}

fn apply_gateway_headers(
    mut request: reqwest::RequestBuilder,
    gateway: &ResolvedGateway,
) -> reqwest::RequestBuilder {
    for (name, value) in &gateway.headers {
        request = request.header(name, value.expose());
    }
    request
}
