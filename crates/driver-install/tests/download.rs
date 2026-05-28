use driver_install::{download_to_file, DownloadOpts, DriverInstallError};
use tokio_util::sync::CancellationToken;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn downloads_body_to_file() {
    let server = MockServer::start().await;
    let body = vec![7u8; 4096];
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(body.clone()))
        .mount(&server)
        .await;
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("nested").join("driver.exe");
    let client = reqwest::Client::new();
    let outcome = download_to_file(&client, &server.uri(), &dest, DownloadOpts::default())
        .await
        .expect("download succeeds");
    assert_eq!(outcome.bytes, 4096);
    assert_eq!(outcome.path, dest);
    assert_eq!(std::fs::read(&dest).unwrap(), body);
}

#[tokio::test]
async fn retries_a_transient_5xx_then_succeeds() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(503))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![1u8; 16]))
        .mount(&server)
        .await;
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("driver.exe");
    let client = reqwest::Client::new();
    let opts = DownloadOpts {
        max_retries: 3,
        ..Default::default()
    };
    let outcome = download_to_file(&client, &server.uri(), &dest, opts)
        .await
        .expect("download succeeds after retry");
    assert_eq!(outcome.bytes, 16);
}

#[tokio::test]
async fn pre_cancelled_token_returns_cancelled() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![0u8; 8]))
        .mount(&server)
        .await;
    let token = CancellationToken::new();
    token.cancel();
    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("driver.exe");
    let client = reqwest::Client::new();
    let opts = DownloadOpts {
        cancel: Some(token),
        ..Default::default()
    };
    let err = download_to_file(&client, &server.uri(), &dest, opts)
        .await
        .expect_err("cancelled before any byte");
    assert!(matches!(err, DriverInstallError::Cancelled));
}
