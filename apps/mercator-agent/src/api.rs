use reqwest::StatusCode;
use thiserror::Error;

use crate::types::{EnrollmentRequest, EnrollmentResponse, HeartbeatRequest};

#[derive(Debug, Error)]
pub enum ApiError {
    #[error("request failed: {0}")]
    Request(#[from] reqwest::Error),
    #[error("Mercator API returned {status}: {body}")]
    HttpStatus { status: StatusCode, body: String },
}

pub async fn enroll(
    server_url: &str,
    enrollment_token: &str,
    payload: &EnrollmentRequest,
) -> Result<EnrollmentResponse, ApiError> {
    let client = reqwest::Client::new();
    let response = client
        .post(endpoint(server_url, "/api/agent/enroll"))
        .bearer_auth(enrollment_token)
        .json(payload)
        .send()
        .await?;

    parse_json_response(response).await
}

pub async fn send_heartbeat(
    server_url: &str,
    device_token: &str,
    payload: &HeartbeatRequest,
) -> Result<(), ApiError> {
    let client = reqwest::Client::new();
    let response = client
        .post(endpoint(server_url, "/api/agent/heartbeat"))
        .bearer_auth(device_token)
        .json(payload)
        .send()
        .await?;

    parse_empty_response(response).await
}

fn endpoint(server_url: &str, path: &str) -> String {
    format!("{}{}", server_url.trim_end_matches('/'), path)
}

async fn parse_json_response<T>(response: reqwest::Response) -> Result<T, ApiError>
where
    T: serde::de::DeserializeOwned,
{
    let response = ensure_success(response).await?;
    Ok(response.json::<T>().await?)
}

async fn parse_empty_response(response: reqwest::Response) -> Result<(), ApiError> {
    ensure_success(response).await?;
    Ok(())
}

async fn ensure_success(response: reqwest::Response) -> Result<reqwest::Response, ApiError> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }

    let body = response
        .text()
        .await
        .unwrap_or_else(|_| "<failed to read response body>".to_string());
    Err(ApiError::HttpStatus {
        status,
        body: truncate_body(&body),
    })
}

fn truncate_body(body: &str) -> String {
    const MAX_BODY_CHARS: usize = 512;
    body.chars().take(MAX_BODY_CHARS).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn endpoint_should_join_without_double_slash() {
        assert_eq!(
            endpoint("https://mercator.example.com/", "/api/agent/enroll"),
            "https://mercator.example.com/api/agent/enroll"
        );
    }
}
