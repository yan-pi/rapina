use serde::Serialize;
use std::fmt;

use crate::response::{BoxBody, IntoResponse};
use bytes::Bytes;
use http_body_util::Full;

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: ErrorDetail,
    pub trace_id: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorDetail {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

#[derive(Debug)]
pub struct Error {
    pub status: u16,
    pub code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
    pub trace_id: Option<String>,
}

impl Error {
    pub fn new(status: u16, code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            status,
            code: code.into(),
            message: message.into(),
            details: None,
            trace_id: None,
        }
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    pub fn with_trace_id(mut self, trace_id: impl Into<String>) -> Self {
        self.trace_id = Some(trace_id.into());
        self
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(400, "BAD_REQUEST", message)
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(401, "UNAUTHORIZED", message)
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::new(403, "FORBIDDEN", message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(404, "NOT_FOUND", message)
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(409, "CONFLICT", message)
    }

    pub fn validation(message: impl Into<String>) -> Self {
        Self::new(422, "VALIDATION_ERROR", message)
    }

    pub fn rate_limited(message: impl Into<String>) -> Self {
        Self::new(429, "RATE_LIMITED", message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(500, "INTERNAL_ERROR", message)
    }

    pub fn to_response(&self, trace_id: String) -> ErrorResponse {
        ErrorResponse {
            error: ErrorDetail {
                code: self.code.clone(),
                message: self.message.clone(),
                details: self.details.clone(),
            },
            trace_id,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for Error {}

impl IntoResponse for Error {
    fn into_response(self) -> http::Response<BoxBody> {
        // Use existing trace_id or generate new one as fallback
        let trace_id = self
            .trace_id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let response = self.to_response(trace_id);
        let body = serde_json::to_vec(&response).unwrap_or_default();

        http::Response::builder()
            .status(self.status)
            .header("content-type", "application/json")
            .body(Full::new(Bytes::from(body)))
            .unwrap()
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;
    use http_body_util::BodyExt;

    #[test]
    fn test_error_new() {
        let err = Error::new(500, "TEST_ERROR", "test message");
        assert_eq!(err.status, 500);
        assert_eq!(err.code, "TEST_ERROR");
        assert_eq!(err.message, "test message");
        assert!(err.details.is_none());
        assert!(err.trace_id.is_none());
    }

    #[test]
    fn test_error_bad_request() {
        let err = Error::bad_request("invalid input");
        assert_eq!(err.status, 400);
        assert_eq!(err.code, "BAD_REQUEST");
        assert_eq!(err.message, "invalid input");
    }

    #[test]
    fn test_error_unauthorized() {
        let err = Error::unauthorized("not authenticated");
        assert_eq!(err.status, 401);
        assert_eq!(err.code, "UNAUTHORIZED");
    }

    #[test]
    fn test_error_forbidden() {
        let err = Error::forbidden("access denied");
        assert_eq!(err.status, 403);
        assert_eq!(err.code, "FORBIDDEN");
    }

    #[test]
    fn test_error_not_found() {
        let err = Error::not_found("resource not found");
        assert_eq!(err.status, 404);
        assert_eq!(err.code, "NOT_FOUND");
    }

    #[test]
    fn test_error_conflict() {
        let err = Error::conflict("already exists");
        assert_eq!(err.status, 409);
        assert_eq!(err.code, "CONFLICT");
    }

    #[test]
    fn test_error_validation() {
        let err = Error::validation("invalid data");
        assert_eq!(err.status, 422);
        assert_eq!(err.code, "VALIDATION_ERROR");
    }

    #[test]
    fn test_error_rate_limited() {
        let err = Error::rate_limited("too many requests");
        assert_eq!(err.status, 429);
        assert_eq!(err.code, "RATE_LIMITED");
    }

    #[test]
    fn test_error_internal() {
        let err = Error::internal("server error");
        assert_eq!(err.status, 500);
        assert_eq!(err.code, "INTERNAL_ERROR");
    }

    #[test]
    fn test_error_with_details() {
        let details = serde_json::json!({"field": "email", "error": "invalid format"});
        let err = Error::bad_request("validation failed").with_details(details.clone());
        assert_eq!(err.details, Some(details));
    }

    #[test]
    fn test_error_with_trace_id() {
        let err = Error::bad_request("test").with_trace_id("trace-123");
        assert_eq!(err.trace_id, Some("trace-123".to_string()));
    }

    #[test]
    fn test_error_display() {
        let err = Error::bad_request("invalid input");
        let display = format!("{}", err);
        assert_eq!(display, "BAD_REQUEST: invalid input");
    }

    #[test]
    fn test_error_to_response() {
        let err = Error::not_found("user not found");
        let response = err.to_response("trace-abc".to_string());
        assert_eq!(response.trace_id, "trace-abc");
        assert_eq!(response.error.code, "NOT_FOUND");
        assert_eq!(response.error.message, "user not found");
    }

    #[tokio::test]
    async fn test_error_into_response() {
        let err = Error::bad_request("test error").with_trace_id("my-trace");
        let response = err.into_response();

        assert_eq!(response.status(), 400);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            "application/json"
        );

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(json["error"]["code"], "BAD_REQUEST");
        assert_eq!(json["error"]["message"], "test error");
        assert_eq!(json["trace_id"], "my-trace");
    }

    #[tokio::test]
    async fn test_error_into_response_generates_trace_id() {
        let err = Error::internal("error");
        let response = err.into_response();

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

        // Should have a generated UUID trace_id
        let trace_id = json["trace_id"].as_str().unwrap();
        assert_eq!(trace_id.len(), 36); // UUID format
    }

    #[test]
    fn test_error_response_skips_none_details() {
        let err = Error::bad_request("test");
        let response = err.to_response("trace".to_string());
        let json = serde_json::to_string(&response).unwrap();
        assert!(!json.contains("details"));
    }

    #[test]
    fn test_error_response_includes_details() {
        let details = serde_json::json!({"key": "value"});
        let err = Error::bad_request("test").with_details(details);
        let response = err.to_response("trace".to_string());
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("details"));
        assert!(json.contains("key"));
    }

    #[test]
    fn test_error_is_std_error() {
        let err = Error::internal("test");
        let _: &dyn std::error::Error = &err;
    }

    #[test]
    fn test_error_builder_chain() {
        let details = serde_json::json!({"field": "name"});
        let err = Error::validation("invalid")
            .with_details(details.clone())
            .with_trace_id("trace-123");

        assert_eq!(err.status, 422);
        assert_eq!(err.code, "VALIDATION_ERROR");
        assert_eq!(err.details, Some(details));
        assert_eq!(err.trace_id, Some("trace-123".to_string()));
    }
}
