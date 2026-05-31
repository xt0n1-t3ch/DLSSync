use dll_catalog::{
    download_and_extract_dll, download_and_extract_dll_cached, fetch_shared, DownloadCache,
    DownloadOptions, Release,
};
use std::io::Write as _;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Respond, ResponseTemplate};
use zip::write::SimpleFileOptions;
use zip::CompressionMethod;

const XESS_GROUP_TARGETS: &[&str] = &[
    "libxess.dll",
    "libxess_dx11.dll",
    "libxell.dll",
    "libxess_fg.dll",
];

fn build_xess_like_zip(filenames: &[&str], payload: &[u8]) -> Vec<u8> {
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut w = zip::ZipWriter::new(&mut buf);
        let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Stored);
        for f in filenames {
            w.start_file(format!("bin/x64/{}", f), opts).unwrap();
            w.write_all(payload).unwrap();
        }
        w.finish().unwrap();
    }
    buf.into_inner()
}

fn release_for(filename: &str, cdn_url: &str, sha: &str, size: u64) -> Release {
    Release {
        version: "3.0.1".into(),
        version_packed: 0,
        filename: filename.into(),
        sha256: sha.into(),
        size_bytes: size,
        signed: true,
        released_at: chrono::Utc::now(),
        source: "test".into(),
        cdn_url: cdn_url.into(),
        release_notes: None,
        signature_subject: Some("Intel Corporation".into()),
        channel: "stable".into(),
        is_dev: false,
        min_driver: None,
        hash_algorithm: "sha256".into(),
        zip_entry: None,
    }
}

fn sha256_of(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(bytes);
    let d = h.finalize();
    let mut s = String::with_capacity(64);
    for b in d {
        let _ = std::fmt::Write::write_fmt(&mut s, format_args!("{:02x}", b));
    }
    s
}

#[tokio::test]
async fn xess_regression_zip_downloads_exactly_once_for_four_dlls() {
    let server = MockServer::start().await;
    let payload = b"intel-xess-runtime-dll-content";
    let payload_sha = sha256_of(payload);
    let zip_bytes = build_xess_like_zip(XESS_GROUP_TARGETS, payload);

    Mock::given(method("GET"))
        .and(path("/XeSS_SDK_3.0.1.zip"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(zip_bytes.clone()))
        .expect(1)
        .mount(&server)
        .await;

    let url = format!("{}/XeSS_SDK_3.0.1.zip", server.uri());
    let client = reqwest::Client::new();
    let cache = DownloadCache::new();
    let releases: Vec<Release> = XESS_GROUP_TARGETS
        .iter()
        .map(|name| release_for(name, &url, &payload_sha, payload.len() as u64))
        .collect();

    let tmp = tempfile::tempdir().unwrap();
    let mut handles = Vec::new();
    for r in releases {
        let dest = tmp.path().join(format!("d_{}", r.filename));
        std::fs::create_dir_all(&dest).unwrap();
        let client_c = client.clone();
        let cache_c = cache.clone();
        handles.push(tokio::spawn(async move {
            download_and_extract_dll_cached(
                &cache_c,
                &client_c,
                &r,
                &dest,
                DownloadOptions::default(),
            )
            .await
            .map(|p| std::fs::metadata(&p).unwrap().len())
        }));
    }
    let mut sizes = Vec::new();
    for h in handles {
        sizes.push(h.await.unwrap().unwrap());
    }
    assert_eq!(sizes.len(), 4, "all four DLLs extracted");
    for s in &sizes {
        assert_eq!(*s as usize, payload.len());
    }
    // wiremock's .expect(1).mount() asserts the upstream was hit exactly once on drop.
    drop(server);
}

#[tokio::test]
async fn truncated_body_then_full_succeeds_via_retry() {
    let server = MockServer::start().await;
    let payload = b"recoverable-payload-bytes";

    let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    struct FlipFlop {
        counter: Arc<std::sync::atomic::AtomicUsize>,
        good: Vec<u8>,
    }
    impl Respond for FlipFlop {
        fn respond(&self, _request: &wiremock::Request) -> ResponseTemplate {
            let n = self
                .counter
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if n == 0 {
                ResponseTemplate::new(200)
                    .insert_header("content-length", self.good.len().to_string())
                    .set_body_bytes(&self.good[..self.good.len() / 2])
            } else {
                ResponseTemplate::new(200)
                    .insert_header("content-length", self.good.len().to_string())
                    .set_body_bytes(self.good.clone())
            }
        }
    }

    Mock::given(method("GET"))
        .and(path("/flaky.dll"))
        .respond_with(FlipFlop {
            counter: counter.clone(),
            good: payload.to_vec(),
        })
        .mount(&server)
        .await;

    let url = format!("{}/flaky.dll", server.uri());
    let cache = DownloadCache::new();
    let client = reqwest::Client::new();
    let opts = DownloadOptions {
        max_retries: 3,
        chunk_timeout: Duration::from_secs(5),
        progress_tx: None,
        cancel: None,
    };
    let bytes = fetch_shared(&cache, &client, &url, opts).await.unwrap();
    assert_eq!(bytes.as_ref().as_ref(), payload);
    assert!(counter.load(std::sync::atomic::Ordering::SeqCst) >= 2);
}

#[tokio::test]
async fn fatal_404_does_not_retry() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/missing.dll"))
        .respond_with(ResponseTemplate::new(404))
        .expect(1)
        .mount(&server)
        .await;
    let url = format!("{}/missing.dll", server.uri());
    let cache = DownloadCache::new();
    let client = reqwest::Client::new();
    let result = fetch_shared(&cache, &client, &url, DownloadOptions::default()).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn retries_503_with_eventual_success() {
    let server = MockServer::start().await;
    let payload = b"eventually-served-bytes";
    let payload_sha = sha256_of(payload);

    let counter = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    struct FailThenOk {
        counter: Arc<std::sync::atomic::AtomicUsize>,
        good: Vec<u8>,
    }
    impl Respond for FailThenOk {
        fn respond(&self, _r: &wiremock::Request) -> ResponseTemplate {
            let n = self
                .counter
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            if n == 0 {
                ResponseTemplate::new(503)
            } else {
                ResponseTemplate::new(200).set_body_bytes(self.good.clone())
            }
        }
    }

    Mock::given(method("GET"))
        .and(path("/maybe.dll"))
        .respond_with(FailThenOk {
            counter: counter.clone(),
            good: payload.to_vec(),
        })
        .mount(&server)
        .await;

    let url = format!("{}/maybe.dll", server.uri());
    let cache = DownloadCache::new();
    let client = reqwest::Client::new();
    let result = fetch_shared(&cache, &client, &url, DownloadOptions::default()).await;
    assert!(result.is_ok());
    let _ = payload_sha;
    assert_eq!(counter.load(std::sync::atomic::Ordering::SeqCst), 2);
}

#[tokio::test]
async fn cancellation_token_aborts_in_flight_download() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/slow.dll"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_delay(Duration::from_secs(30))
                .set_body_bytes(vec![0u8; 1024]),
        )
        .mount(&server)
        .await;
    let url = format!("{}/slow.dll", server.uri());
    let cache = DownloadCache::new();
    let client = reqwest::Client::new();
    let cancel = CancellationToken::new();
    let opts = DownloadOptions {
        max_retries: 1,
        chunk_timeout: Duration::from_secs(5),
        progress_tx: None,
        cancel: Some(cancel.clone()),
    };
    let cancel_handle = cancel.clone();
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(200)).await;
        cancel_handle.cancel();
    });
    let result = fetch_shared(&cache, &client, &url, opts).await;
    assert!(result.is_err(), "cancelled download should error out");
}

fn build_deflated_zip(filename: &str, payload: &[u8]) -> Vec<u8> {
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut w = zip::ZipWriter::new(&mut buf);
        let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        w.start_file(filename, opts).unwrap();
        w.write_all(payload).unwrap();
        w.finish().unwrap();
    }
    buf.into_inner()
}

// `expected 7499376 bytes, server reported 3736946`). SHA-256 of the final
#[tokio::test]
async fn small_compressed_zip_does_not_false_positive_size_mismatch() {
    let server = MockServer::start().await;
    let dll_bytes = vec![0u8; 7_499_376];
    let dll_sha = sha256_of(&dll_bytes);
    let zip_bytes = build_deflated_zip("nvngx_dlssg.dll", &dll_bytes);
    assert!(
        zip_bytes.len() < dll_bytes.len() / 4,
        "test fixture should compress at least 4x to exercise the bug shape"
    );

    Mock::given(method("GET"))
        .and(path("/nvngx_dlssg.zip"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(zip_bytes.clone()))
        .mount(&server)
        .await;

    let url = format!("{}/nvngx_dlssg.zip", server.uri());
    let release = release_for("nvngx_dlssg.dll", &url, &dll_sha, dll_bytes.len() as u64);
    let client = reqwest::Client::new();
    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("dest");
    std::fs::create_dir_all(&dest).unwrap();

    let out = download_and_extract_dll(&client, &release, &dest).await;
    assert!(
        out.is_ok(),
        "apply for a compressed-zip-wrapped DLL must NOT be rejected by size compare; got {:?}",
        out.err()
    );
    let extracted = out.unwrap();
    let on_disk = std::fs::metadata(&extracted).unwrap().len();
    assert_eq!(
        on_disk,
        dll_bytes.len() as u64,
        "extracted DLL must match manifest size_bytes byte-for-byte"
    );
}

#[tokio::test]
async fn bare_dll_with_smaller_content_length_does_not_false_positive() {
    let server = MockServer::start().await;
    let dll_bytes = b"realistic-dll-bytes-here".repeat(64);
    let dll_sha = sha256_of(&dll_bytes);

    Mock::given(method("GET"))
        .and(path("/raw.dll"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(dll_bytes.clone()))
        .mount(&server)
        .await;

    let url = format!("{}/raw.dll", server.uri());
    let release = release_for("raw.dll", &url, &dll_sha, dll_bytes.len() as u64 * 10);
    let client = reqwest::Client::new();
    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("dest2");
    std::fs::create_dir_all(&dest).unwrap();

    let out = download_and_extract_dll(&client, &release, &dest).await;
    assert!(
        out.is_ok(),
        "bare-DLL with Content-Length smaller than manifest size_bytes must succeed; got {:?}",
        out.err()
    );
}

#[tokio::test]
async fn empty_200_response_does_not_silently_succeed() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/empty.dll"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(Vec::<u8>::new()))
        .mount(&server)
        .await;
    let url = format!("{}/empty.dll", server.uri());
    let release = release_for(
        "empty.dll",
        &url,
        "0000000000000000000000000000000000000000000000000000000000000000",
        0,
    );
    let client = reqwest::Client::new();
    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("dest_empty");
    std::fs::create_dir_all(&dest).unwrap();
    let out = download_and_extract_dll(&client, &release, &dest).await;
    assert!(
        out.is_err(),
        "zero-byte 200 response must NOT be treated as a successful apply"
    );
}

#[tokio::test]
async fn sha256_mismatch_on_extracted_dll_is_rejected() {
    let server = MockServer::start().await;
    let dll_bytes = b"this-is-the-real-payload".to_vec();
    let zip_bytes = build_deflated_zip("nvngx_dlss.dll", &dll_bytes);
    Mock::given(method("GET"))
        .and(path("/bad-hash.zip"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(zip_bytes))
        .mount(&server)
        .await;
    let url = format!("{}/bad-hash.zip", server.uri());
    // Wrong SHA-256 in the manifest entry — extraction must fail before any
    let lying_sha = "dead".repeat(16);
    let release = release_for("nvngx_dlss.dll", &url, &lying_sha, dll_bytes.len() as u64);
    let client = reqwest::Client::new();
    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("dest_badsha");
    std::fs::create_dir_all(&dest).unwrap();
    let out = download_and_extract_dll(&client, &release, &dest).await;
    assert!(out.is_err(), "SHA-256 mismatch must reject the apply");
}

#[tokio::test]
async fn case_insensitive_filename_match_inside_zip() {
    let server = MockServer::start().await;
    let dll_bytes = b"case-sensitive-payload".to_vec();
    let dll_sha = sha256_of(&dll_bytes);
    let zip_bytes = build_deflated_zip("bin/x64/NVngx_Dlss.dll", &dll_bytes);
    Mock::given(method("GET"))
        .and(path("/cased.zip"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(zip_bytes))
        .mount(&server)
        .await;
    let url = format!("{}/cased.zip", server.uri());
    let release = release_for("nvngx_dlss.dll", &url, &dll_sha, dll_bytes.len() as u64);
    let client = reqwest::Client::new();
    let tmp = tempfile::tempdir().unwrap();
    let dest = tmp.path().join("dest_case");
    std::fs::create_dir_all(&dest).unwrap();
    let out = download_and_extract_dll(&client, &release, &dest).await;
    assert!(
        out.is_ok(),
        "filename match inside zip must be case-insensitive; got {:?}",
        out.err()
    );
}

#[tokio::test]
async fn parallel_distinct_urls_run_concurrently_without_cache_collision() {
    let server = MockServer::start().await;
    let payload = b"distinct-payload".to_vec();
    let sha = sha256_of(&payload);
    for n in 0..4 {
        let p = format!("/iso-{}.dll", n);
        Mock::given(method("GET"))
            .and(path(p))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(payload.clone()))
            .expect(1)
            .mount(&server)
            .await;
    }
    let cache = DownloadCache::new();
    let client = reqwest::Client::new();
    let tmp = tempfile::tempdir().unwrap();
    let mut handles = Vec::new();
    for n in 0..4 {
        let url = format!("{}/iso-{}.dll", server.uri(), n);
        let release = release_for(&format!("iso-{}.dll", n), &url, &sha, payload.len() as u64);
        let dest = tmp.path().join(format!("d{}", n));
        std::fs::create_dir_all(&dest).unwrap();
        let cache_c = cache.clone();
        let client_c = client.clone();
        handles.push(tokio::spawn(async move {
            download_and_extract_dll_cached(
                &cache_c,
                &client_c,
                &release,
                &dest,
                DownloadOptions::default(),
            )
            .await
        }));
    }
    for h in handles {
        h.await
            .unwrap()
            .expect("each distinct URL must download independently");
    }
    // Each upstream hit exactly once (per `.expect(1)` mounts); drop asserts.
    drop(server);
}

#[tokio::test]
async fn progress_events_emitted_during_download() {
    let server = MockServer::start().await;
    let big_payload = vec![0u8; 256 * 1024];
    Mock::given(method("GET"))
        .and(path("/big.bin"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(big_payload.clone()))
        .mount(&server)
        .await;
    let (tx, mut rx) = mpsc::unbounded_channel();
    let url = format!("{}/big.bin", server.uri());
    let cache = DownloadCache::new();
    let client = reqwest::Client::new();
    let opts = DownloadOptions {
        max_retries: 1,
        chunk_timeout: Duration::from_secs(5),
        progress_tx: Some(tx),
        cancel: None,
    };
    let result = fetch_shared(&cache, &client, &url, opts).await;
    assert!(result.is_ok());
    let mut got_event = false;
    while let Ok(p) = rx.try_recv() {
        if p.bytes_downloaded > 0 {
            got_event = true;
            assert!(p.bytes_per_sec >= 0.0);
        }
    }
    assert!(got_event, "expected at least one progress event");
}
