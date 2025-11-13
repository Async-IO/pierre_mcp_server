// ABOUTME: Axum HTTP testing utilities for integration tests
// ABOUTME: Provides helpers to test Axum routes without running a full server

use axum::{
    body::Body,
    http::{header, Method, Request, StatusCode},
    Router,
};
use serde::Serialize;
use tower::ServiceExt;

/// Helper to build and execute HTTP requests against Axum routers
pub struct AxumTestRequest {
    method: Method,
    uri: String,
    headers: Vec<(String, String)>,
    body: Option<String>,
}

impl AxumTestRequest {
    /// Create a new GET request
    pub fn get(uri: &str) -> Self {
        Self {
            method: Method::GET,
            uri: uri.to_owned(),
            headers: Vec::new(),
            body: None,
        }
    }

    /// Create a new POST request
    pub fn post(uri: &str) -> Self {
        Self {
            method: Method::POST,
            uri: uri.to_owned(),
            headers: Vec::new(),
            body: None,
        }
    }

    /// Create a new DELETE request
    /// Note: Used by `routes_api_keys_http_test.rs`, but not all tests use it
    #[allow(dead_code)]
    pub fn delete(uri: &str) -> Self {
        Self {
            method: Method::DELETE,
            uri: uri.to_owned(),
            headers: Vec::new(),
            body: None,
        }
    }

    /// Create a new PUT request
    /// Note: Used by configuration and fitness tests, but not all tests use it
    #[allow(dead_code)]
    pub fn put(uri: &str) -> Self {
        Self {
            method: Method::PUT,
            uri: uri.to_owned(),
            headers: Vec::new(),
            body: None,
        }
    }

    /// Add a header to the request
    pub fn header(mut self, key: &str, value: &str) -> Self {
        self.headers.push((key.to_owned(), value.to_owned()));
        self
    }

    /// Add JSON body to the request
    pub fn json<T: Serialize>(mut self, data: &T) -> Self {
        self.body = Some(serde_json::to_string(data).expect("Failed to serialize JSON"));
        self.headers.push((
            header::CONTENT_TYPE.as_str().to_owned(),
            "application/json".to_owned(),
        ));
        self
    }

    /// Execute the request against an Axum router
    pub async fn send(self, app: Router) -> AxumTestResponse {
        let mut builder = Request::builder().method(self.method).uri(self.uri);

        for (key, value) in self.headers {
            builder = builder.header(key, value);
        }

        let body = self.body.unwrap_or_default();
        let request = builder
            .body(Body::from(body))
            .expect("Failed to build request");

        let response = app
            .oneshot(request)
            .await
            .expect("Failed to execute request");

        AxumTestResponse::from_response(response).await
    }

    /// Execute request for SSE endpoints - only reads headers, not streaming body
    ///
    /// SSE endpoints return infinite streams, so we can't read the full body.
    /// This method validates connection establishment and initial response headers only.
    #[allow(dead_code)]
    pub async fn send_sse(self, app: Router) -> AxumTestResponse {
        let mut builder = Request::builder().method(self.method).uri(self.uri);

        for (key, value) in self.headers {
            builder = builder.header(key, value);
        }

        let body = self.body.unwrap_or_default();
        let request = builder
            .body(Body::from(body))
            .expect("Failed to build request");

        let response = app
            .oneshot(request)
            .await
            .expect("Failed to execute request");

        // For SSE, only extract status code and headers - don't read the body stream
        AxumTestResponse::from_sse_response(response).await
    }
}

/// Wrapper around Axum HTTP response for testing
pub struct AxumTestResponse {
    status: StatusCode,
    body: Vec<u8>,
}

impl AxumTestResponse {
    /// Create from response by eagerly reading the body
    async fn from_response(response: axum::http::Response<Body>) -> Self {
        use axum::body::to_bytes;
        let status = response.status();
        let body = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("Failed to read response body")
            .to_vec();
        Self { status, body }
    }

    /// Create from SSE response - only extracts headers, doesn't read streaming body
    ///
    /// SSE streams are infinite, so we can't read the full body. This method
    /// validates the connection was established by checking the status code.
    #[allow(dead_code)]
    async fn from_sse_response(response: axum::http::Response<Body>) -> Self {
        use axum::body::to_bytes;

        let status = response.status();

        // For SSE endpoints, try to read first chunk with 1 second timeout
        // If timeout occurs, that's OK - it means the SSE stream is waiting for events
        let body_result = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            to_bytes(response.into_body(), 1024), // Read up to 1KB
        )
        .await;

        let body = match body_result {
            Ok(Ok(bytes)) => bytes.to_vec(),
            Ok(Err(_)) | Err(_) => Vec::new(), // Body read error or timeout (OK for SSE)
        };

        Self { status, body }
    }

    /// Get the response status code as u16 for easy assertion
    pub const fn status(&self) -> u16 {
        self.status.as_u16()
    }

    /// Get the response status code as `StatusCode`
    pub const fn status_code(&self) -> StatusCode {
        self.status
    }

    /// Get the response body as bytes
    pub fn bytes(self) -> Vec<u8> {
        self.body
    }

    /// Get the response body as a JSON value
    pub fn json<T: serde::de::DeserializeOwned>(self) -> T {
        serde_json::from_slice(&self.body).expect("Failed to deserialize JSON response")
    }

    /// Get the response body as a string
    pub fn text(self) -> String {
        String::from_utf8(self.body).expect("Failed to decode response as UTF-8")
    }

    /// Assert that the status code matches
    pub fn assert_status(self, expected: StatusCode) -> Self {
        assert_eq!(
            self.status, expected,
            "Expected status {}, got {}",
            expected, self.status
        );
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{routing::get, Json};

    #[tokio::test]
    async fn test_axum_test_request_get() {
        let app = Router::new().route("/test", get(|| async { "Hello" }));
        let response = AxumTestRequest::get("/test").send(app).await;
        assert_eq!(response.status(), 200);
        assert_eq!(response.text(), "Hello");
    }

    #[tokio::test]
    async fn test_axum_test_request_post_with_json() {
        let app = Router::new().route(
            "/test",
            axum::routing::post(|Json(body): Json<serde_json::Value>| async move {
                Json(serde_json::json!({"received": body}))
            }),
        );
        let response = AxumTestRequest::post("/test")
            .json(&serde_json::json!({"key": "value"}))
            .send(app)
            .await;
        assert_eq!(response.status(), 200);
        let json: serde_json::Value = response.json();
        assert_eq!(json["received"]["key"], "value");
    }

    #[tokio::test]
    async fn test_axum_test_request_with_header() {
        let app = Router::new().route(
            "/test",
            get(|headers: axum::http::HeaderMap| async move {
                headers
                    .get("x-custom")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("missing")
                    .to_owned()
            }),
        );
        let response = AxumTestRequest::get("/test")
            .header("x-custom", "test-value")
            .send(app)
            .await;
        assert_eq!(response.text(), "test-value");
    }

    #[tokio::test]
    async fn test_axum_test_response_methods() {
        let app = Router::new().route("/test", get(|| async { "test response" }));
        let response = AxumTestRequest::get("/test").send(app).await;

        assert_eq!(response.status(), 200);
        assert_eq!(response.status_code(), StatusCode::OK);
        let bytes = response.bytes();
        assert_eq!(bytes, b"test response");
    }

    #[tokio::test]
    async fn test_axum_test_response_assert_status() {
        let app = Router::new().route("/test", get(|| async { "ok" }));
        let response = AxumTestRequest::get("/test").send(app).await;
        let response = response.assert_status(StatusCode::OK);
        assert_eq!(response.text(), "ok");
    }
}
