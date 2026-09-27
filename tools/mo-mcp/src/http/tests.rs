use super::*;
use http_body_util::BodyExt;
use std::{
    path::PathBuf,
    sync::atomic::{AtomicUsize, Ordering},
};

struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let path = std::env::temp_dir().join(format!(
            "mo-http-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir(&path).unwrap();
        for name in ["input", "output", "temporary"] {
            std::fs::create_dir(path.join(name)).unwrap();
        }
        Self(path)
    }
    fn endpoint(&self, origins: Vec<String>) -> Endpoint {
        let value = serde_json::json!({
            "inputDirectory":self.0.join("input"),
            "outputDirectory":self.0.join("output"),
            "temporaryDirectory":self.0.join("temporary")
        });
        let path = self.0.join("config.json");
        std::fs::write(&path, value.to_string()).unwrap();
        Endpoint::new(
            &Config::read(&path).unwrap(),
            Options {
                allowed_hosts: vec!["localhost:3000".into()],
                allowed_origins: origins,
            },
        )
        .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).unwrap();
    }
}
fn get() -> Request<Body> {
    Request::builder()
        .method("GET")
        .uri("/mcp")
        .header("host", "localhost:3000")
        .body(Body::empty())
        .unwrap()
}
fn pending_body() -> Body {
    Body::from_stream(futures::stream::pending::<
        Result<axum::body::Bytes, std::io::Error>,
    >())
}

#[test]
fn deployment_configuration_is_explicit() {
    for host in [
        "",
        "*",
        "*.example.com",
        "user@example.com",
        "example.com/path",
    ] {
        assert!(
            Options {
                allowed_hosts: vec![host.into()],
                allowed_origins: vec![]
            }
            .validate()
            .is_err()
        );
    }
    for origin in [
        "*",
        "null",
        "https://example.com/page",
        "https://user@example.com",
        "https://example.com?x=1",
    ] {
        assert!(
            Options {
                allowed_hosts: vec!["localhost".into()],
                allowed_origins: vec![origin.into()]
            }
            .validate()
            .is_err()
        );
    }
    assert!(
        Options {
            allowed_hosts: vec!["[::1]:3000".into()],
            allowed_origins: vec!["https://example.com".into()]
        }
        .validate()
        .is_ok()
    );
}

#[tokio::test]
async fn admission_lasts_until_response_consumption_or_drop() {
    let fixture = Fixture::new();
    let endpoint = fixture.endpoint(vec![]);
    let mut held = Vec::new();
    for _ in 0..HTTP_REQUESTS {
        let response = endpoint.handle(get()).await;
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
        held.push(response);
    }
    assert_eq!(
        endpoint.handle(get()).await.status(),
        StatusCode::TOO_MANY_REQUESTS
    );
    drop(held.pop());
    assert_eq!(endpoint.state.requests.available_permits(), 1);
    let response = endpoint.handle(get()).await;
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    assert_eq!(endpoint.state.requests.available_permits(), 1);
    drop(held);
    assert_eq!(endpoint.state.requests.available_permits(), HTTP_REQUESTS);
    endpoint.shutdown();
    assert_eq!(
        endpoint.handle(get()).await.status(),
        StatusCode::SERVICE_UNAVAILABLE
    );
}

#[tokio::test]
async fn output_budget_deadline_error_and_drop_release_admission() {
    let fixture = Fixture::new();
    let endpoint = fixture.endpoint(vec![]);
    for (inner, bytes, deadline) in [
        (
            Body::from("abc"),
            2,
            tokio::time::Instant::now() + Duration::from_secs(10),
        ),
        (pending_body(), 2, tokio::time::Instant::now()),
        (
            Body::from_stream(futures::stream::once(async {
                Err::<axum::body::Bytes, _>(std::io::Error::other("fixture failure"))
            })),
            2,
            tokio::time::Instant::now() + Duration::from_secs(10),
        ),
    ] {
        let permit = endpoint.state.requests.clone().try_acquire_owned().unwrap();
        let mut body = body::Output::new(inner, permit, endpoint.state.clone(), bytes, deadline);
        assert!(body.frame().await.unwrap().is_err());
        assert!(body.frame().await.is_none());
        assert_eq!(endpoint.state.requests.available_permits(), HTTP_REQUESTS);
    }
    let permit = endpoint.state.requests.clone().try_acquire_owned().unwrap();
    let body = body::Output::new(
        pending_body(),
        permit,
        endpoint.state.clone(),
        2,
        tokio::time::Instant::now() + Duration::from_secs(10),
    );
    drop(body);
    assert_eq!(endpoint.state.requests.available_permits(), HTTP_REQUESTS);
}

#[tokio::test]
async fn invalid_inputs_keep_their_client_error_status() {
    for (input, status) in [
        (
            br#"{"outer":{"a":1,"a":2}}"#.to_vec(),
            StatusCode::BAD_REQUEST,
        ),
        (vec![0xff], StatusCode::BAD_REQUEST),
        (vec![b' '; INPUT_BYTES + 1], StatusCode::PAYLOAD_TOO_LARGE),
    ] {
        let (body, failure) = body::validated_input(Body::from(input));
        assert!(axum::body::to_bytes(body, INPUT_BYTES).await.is_err());
        assert_eq!(failure.get().unwrap().response().status(), status);
    }
    let (body, failure) = body::validated_input(Body::from("{}"));
    assert_eq!(axum::body::to_bytes(body, INPUT_BYTES).await.unwrap(), "{}");
    assert!(failure.get().is_none());
}

#[tokio::test]
async fn origin_is_checked_before_reading_body() {
    let fixture = Fixture::new();
    let endpoint = fixture.endpoint(vec!["https://caller.example".into()]);
    for (origin, expected) in [
        ("https://caller.example", StatusCode::NOT_ACCEPTABLE),
        ("https://other.example", StatusCode::FORBIDDEN),
    ] {
        let request = Request::builder()
            .method("POST")
            .uri("/mcp")
            .header("host", "localhost:3000")
            .header("origin", origin)
            .body(pending_body())
            .unwrap();
        let result = tokio::time::timeout(Duration::from_millis(100), endpoint.handle(request))
            .await
            .unwrap();
        assert_eq!(result.status(), expected);
    }
}
