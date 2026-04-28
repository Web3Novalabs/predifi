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
//! { "status": "error", "error": "<message>" }
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
//! ApiResponse::<()>::error("not found")
//! ```

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

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
    pub error: Option<String>,
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

    /// Wrap an error message with an explicit HTTP status code.
    pub fn error(status_code: StatusCode, message: impl Into<String>) -> (StatusCode, Json<Self>) {
        (
            status_code,
            Json(Self {
                status: "error",
                data: None,
                error: Some(message.into()),
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
        let (code, Json(resp)) = ApiResponse::<()>::error(StatusCode::NOT_FOUND, "not found");
        assert_eq!(code, StatusCode::NOT_FOUND);
        assert_eq!(resp.status, "error");
        assert!(resp.data.is_none());
        assert_eq!(resp.error.as_deref(), Some("not found"));
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
        let (_, Json(resp)) = ApiResponse::<()>::error(StatusCode::BAD_REQUEST, "bad input");
        let v = to_value(&resp);
        assert!(
            v.get("data").is_none(),
            "data field must be absent on error"
        );
        assert_eq!(v["status"], "error");
        assert_eq!(v["error"], "bad input");
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
}
