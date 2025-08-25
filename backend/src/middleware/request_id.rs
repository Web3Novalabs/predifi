use axum::{extract::Request, http::HeaderValue, response::Response};
use std::task::{Context, Poll};
use tower::{Layer, Service};
use tracing::{Instrument, info_span};
use uuid::Uuid;

const REQUEST_ID_HEADER: &str = "x-request-id";

#[derive(Clone)]
pub struct RequestIdLayer;

impl<S> Layer<S> for RequestIdLayer {
    type Service = RequestIdService<S>;

    fn layer(&self, service: S) -> Self::Service {
        RequestIdService { inner: service }
    }
}

#[derive(Clone)]
pub struct RequestIdService<S> {
    inner: S,
}

impl<S> Service<Request> for RequestIdService<S>
where
    S: Service<Request, Response = Response> + Send + 'static + Clone,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>,
    >;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut request: Request) -> Self::Future {
        // Extract or generate request ID
        let request_id = extract_or_generate_request_id(&request);

        // Add request ID to request extensions for use in handlers
        request
            .extensions_mut()
            .insert(RequestId(request_id.clone()));

        // Create a span with the request ID for tracing
        let span = info_span!("request", request_id = %request_id);

        let mut inner = self.inner.clone();

        Box::pin(
            async move {
                let response = inner.call(request).await?;

                // Add request ID to response headers
                let mut response = response;
                if let Ok(header_value) = HeaderValue::from_str(&request_id) {
                    response
                        .headers_mut()
                        .insert(REQUEST_ID_HEADER, header_value);
                }

                Ok(response)
            }
            .instrument(span),
        )
    }
}

fn extract_or_generate_request_id(request: &Request) -> String {
    // Check for existing request ID in headers
    if let Some(request_id) = request.headers().get(REQUEST_ID_HEADER)
        && let Ok(id) = request_id.to_str()
        && !id.is_empty()
    {
        return id.to_string();
    }

    // Generate new request ID if none provided or invalid
    Uuid::new_v4().to_string()
}

#[derive(Clone)]
pub struct RequestId(pub String);

impl RequestId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

// Convenience function to create the middleware
pub fn request_id_middleware() -> RequestIdLayer {
    RequestIdLayer
}
