use serde_json::Value;
use utoipa::OpenApi;

use predifi_backend::openapi::ApiDoc;

#[test]
fn openapi_document_has_required_shape() {
    let spec: Value = serde_json::to_value(ApiDoc::openapi()).expect("OpenAPI spec must serialize to JSON");

    assert!(
        spec["info"]["title"].as_str().is_some_and(|title| !title.trim().is_empty()),
        "OpenAPI info.title must be non-empty, got: {spec:?}"
    );

    assert!(
        spec["info"]["version"].as_str().is_some_and(|version| !version.trim().is_empty()),
        "OpenAPI info.version must be non-empty, got: {spec:?}"
    );

    assert!(
        spec["paths"].as_object().is_some_and(|paths| !paths.is_empty()),
        "OpenAPI paths must contain at least one endpoint, got: {spec:?}"
    );
}
