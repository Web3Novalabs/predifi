//! # API Response Wrapper
//!
//! Provides a generic JSON envelope for all API responses.
//!
//! Every response has the same top-level shape:
//!
//! ```json
//! // success
//! { "status": "success", "data": <T> }
//!
//! // error
//! { "status": "error", "error": { "code": "ERROR_CODE", "message": "<message>", "request_id": "<uuid>" } }
//! ```
//!
//! ## Usage
//!
//! ```rust,ignore
//! use crate::response::ApiResponse;
//!
//! // success
//! ApiResponse::success(my_data)
//!
//! // error
//! ApiResponse::<()>::error(StatusCode::NOT_FOUND, "NOT_FOUND", "Resource not found")
//! ```

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use uuid::Uuid;

/// Standardized error detail structure.
#[derive(Debug, Serialize)]
pub struct ErrorDetail {
    /// Machine-readable error code for programmatic handling.
    pub code: &'static str,
    /// Human-readable error message.
    pub message: String,
    /// Unique request identifier for tracing and debugging.
    pub request_id: String,
}

/// Generic JSON envelope returned by every API handler.
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    /// `"success"` or `"error"`.
    pub status: &'static str,
    /// Present on success.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    /// Present on error.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorDetail>,
}

/// Common error codes for standardized error responses.
pub mod error_codes {
    pub const NOT_FOUND: &str = "NOT_FOUND";
    pub const CONFLICT: &str = "CONFLICT";
    pub const INVALID_INPUT: &str = "INVALID_INPUT";
    pub const SERVICE_UNAVAILABLE: &str = "SERVICE_UNAVAILABLE";
    pub const INTERNAL_ERROR: &str = "INTERNAL_ERROR";
    pub const DATABASE_UNAVAILABLE: &str = "DATABASE_UNAVAILABLE";
    pub const RATE_LIMIT_EXCEEDED: &str = "RATE_LIMIT_EXCEEDED";
    pub const UNAUTHORIZED: &str = "UNAUTHORIZED";
    pub const FORBIDDEN: &str = "FORBIDDEN";
}

impl<T: Serialize> ApiResponse<T> {
    /// Wrap a successful payload.
    pub fn success(data: T) -> (StatusCode, Json<Self>) {
        (
            StatusCode::OK,
            Json(Self {
                status: "success",
                data: Some(data),
                error: None,
            }),
        )
    }

    /// Wrap an error with an explicit HTTP status code, error code, and message.
    pub fn error(
        status_code: StatusCode,
        code: &'static str,
        message: impl Into<String>,
    ) -> (StatusCode, Json<Self>) {
        let request_id = Uuid::new_v4().to_string();
        (
            status_code,
            Json(Self {
                status: "error",
                data: None,
                error: Some(ErrorDetail {
                    code,
                    message: message.into(),
                    request_id,
                }),
            }),
        )
    }

    /// Wrap an error with an explicit HTTP status code, error code, message, and custom request_id.
    pub fn error_with_request_id(
        status_code: StatusCode,
        code: &'static str,
        message: impl Into<String>,
        request_id: String,
    ) -> (StatusCode, Json<Self>) {
        (
            status_code,
            Json(Self {
                status: "error",
                data: None,
                error: Some(ErrorDetail {
                    code,
                    message: message.into(),
                    request_id,
                }),
            }),
        )
    }
}

impl<T: Serialize + Send> IntoResponse for ApiResponse<T> {
    fn into_response(self) -> Response {
        let code = if self.error.is_some() {
            StatusCode::INTERNAL_SERVER_ERROR
        } else {
            StatusCode::OK
        };
        (code, Json(self)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn to_value<T: Serialize>(r: &ApiResponse<T>) -> Value {
        serde_json::to_value(r).unwrap()
    }

    #[test]
    fn success_sets_status_and_data() {
        let (code, Json(resp)) = ApiResponse::success(42u32);
        assert_eq!(code, StatusCode::OK);
        assert_eq!(resp.status, "success");
        assert_eq!(resp.data, Some(42u32));
        assert!(resp.error.is_none());
    }

    #[test]
    fn error_sets_status_and_message() {
        let (code, Json(resp)) = ApiResponse::<()>::error(StatusCode::NOT_FOUND, "NOT_FOUND", "not found");
        assert_eq!(code, StatusCode::NOT_FOUND);
        assert_eq!(resp.status, "error");
        assert!(resp.data.is_none());
        assert_eq!(resp.error.as_ref().unwrap().code, "NOT_FOUND");
        assert_eq!(resp.error.as_ref().unwrap().message, "not found");
        assert!(!resp.error.as_ref().unwrap().request_id.is_empty());
    }

    #[test]
    fn success_serializes_without_error_field() {
        let (_, Json(resp)) = ApiResponse::success("hello");
        let v = to_value(&resp);
        assert!(
            v.get("error").is_none(),
            "error field must be absent on success"
        );
        assert_eq!(v["status"], "success");
        assert_eq!(v["data"], "hello");
    }

    #[test]
    fn error_serializes_without_data_field() {
        let (_, Json(resp)) = ApiResponse::<()>::error(StatusCode::BAD_REQUEST, "INVALID_INPUT", "bad input");
        let v = to_value(&resp);
        assert!(
            v.get("data").is_none(),
            "data field must be absent on error"
        );
        assert_eq!(v["status"], "error");
        assert_eq!(v["error"]["code"], "INVALID_INPUT");
        assert_eq!(v["error"]["message"], "bad input");
        assert!(v["error"]["request_id"].is_string());
    }

    #[test]
    fn success_works_with_struct_payload() {
        #[derive(Serialize, PartialEq, Debug)]
        struct Payload {
            id: u32,
            name: String,
        }
        let (code, Json(resp)) = ApiResponse::success(Payload {
            id: 1,
            name: "pool".into(),
        });
        assert_eq!(code, StatusCode::OK);
        assert_eq!(resp.data.unwrap().id, 1);
    }

    #[test]
    fn error_with_custom_request_id() {
        let custom_id = "custom-request-123".to_string();
        let (_, Json(resp)) = ApiResponse::<()>::error_with_request_id(
            StatusCode::SERVICE_UNAVAILABLE,
            "SERVICE_UNAVAILABLE",
            "service temporarily unavailable",
            custom_id.clone(),
        );
        assert_eq!(resp.status, "error");
        assert_eq!(resp.error.as_ref().unwrap().request_id, custom_id);
    }
}
