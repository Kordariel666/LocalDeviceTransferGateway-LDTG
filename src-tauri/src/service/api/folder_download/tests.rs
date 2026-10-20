use super::*;
use crate::service::api::tests::{request, session_headers, test_state};
use axum::body::to_bytes;
use tower::ServiceExt;

async fn response(state: Arc<TransferServiceState>, path: &str) -> Response {
    let existing = state
        .sessions
        .lock()
        .await
        .values()
        .next()
        .map(|s| format!("ldtg_session={}", s.token));
    let cookie = match existing {
        Some(cookie) => cookie,
        None => session_headers(&state).await.0,
    };
    let mut req = request(
        Method::GET,
        &format!(
            "/api/v1/download-folder?path={}",
            utf8_percent_encode(path, NON_ALPHANUMERIC)
        ),
        Body::empty(),
    );
    req.headers_mut()
        .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
    router(state).oneshot(req).await.unwrap()
}

fn manifest(root: &Path, limits: Limits) -> ApiResult<Manifest> {
    let root = fs::canonicalize(root).unwrap();
    scan(
        &root,
        &RootAnchor::open(&root).unwrap(),
        "",
        &AtomicBool::new(false),
        Instant::now() + SCAN_TIMEOUT,
        limits,
    )
}

async fn frame(body: &mut Body) -> Option<Result<hyper::body::Frame<Bytes>, axum::Error>> {
    std::future::poll_fn(|cx| std::pin::Pin::new(&mut *body).poll_frame(cx)).await
}

async fn wait_for_terminal(state: &TransferServiceState, expected: TransferState) {
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if state
                .transfers
                .lock()
                .await
                .last()
                .is_some_and(|t| t.state == expected)
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn folder_zip_preserves_nested_empty_unicode_and_duplicate_basenames() {
    let temp = tempfile::tempdir().unwrap();
    let folder = temp.path().join("Ordner");
    fs::create_dir_all(folder.join("verschachtelt/leer")).unwrap();
    fs::create_dir_all(folder.join("ganz leer")).unwrap();
    fs::write(folder.join("Grüße 🐈.txt"), "Hallo 世界").unwrap();
    fs::write(folder.join("gleich.txt"), "eins").unwrap();
    fs::write(folder.join("verschachtelt/gleich.txt"), "zwei").unwrap();
    fs::write(folder.join("null.txt"), "").unwrap();
    fs::write(folder.join(".privat"), "NICHT EXPORTIEREN").unwrap();
    let state = test_state(temp.path());
    let res = response(state.clone(), "Ordner").await;
    assert_eq!(res.status(), StatusCode::OK);
    assert_eq!(res.headers()[header::CONTENT_TYPE], "application/zip");
    assert_eq!(res.headers()[header::ACCEPT_RANGES], "none");
    assert!(res.headers()[header::CONTENT_DISPOSITION]
        .to_str()
        .unwrap()
        .contains("Ordner%2Ezip"));
    assert_eq!(res.headers()[header::CACHE_CONTROL], "no-store");
    let expected_size: usize = res.headers()[header::CONTENT_LENGTH]
        .to_str()
        .unwrap()
        .parse()
        .unwrap();
    let bytes = to_bytes(res.into_body(), 1024 * 1024).await.unwrap();
    assert_eq!(bytes.len(), expected_size);
    assert_eq!(&bytes[..4], b"PK\x03\x04");
    assert_eq!(&bytes[bytes.len() - 22..bytes.len() - 18], b"PK\x05\x06");
    wait_for_terminal(&state, TransferState::Complete).await;
    let transfer = state.transfers.lock().await.last().cloned().unwrap();
    assert_eq!(transfer.total_bytes, bytes.len() as u64);
    assert_eq!(transfer.transferred_bytes, transfer.total_bytes);

    // Independent native decoder, rather than verifying only our own record writer.
    // Temporary ZIP exists only in this test; production never writes an archive.
    #[cfg(windows)]
    {
        let output = tempfile::tempdir().unwrap();
        let zip = output.path().join("test.zip");
        fs::write(&zip, &bytes).unwrap();
        let expected = serde_json::json!([
            {"name":"Ordner/", "text":""},
            {"name":"Ordner/verschachtelt/", "text":""},
            {"name":"Ordner/verschachtelt/leer/", "text":""},
            {"name":"Ordner/ganz leer/", "text":""},
            {"name":"Ordner/Grüße 🐈.txt", "text":"Hallo 世界"},
            {"name":"Ordner/gleich.txt", "text":"eins"},
            {"name":"Ordner/verschachtelt/gleich.txt", "text":"zwei"},
            {"name":"Ordner/null.txt", "text":""}
        ]);
        let status = std::process::Command::new("powershell.exe")
            .args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                r#"
                $ErrorActionPreference = 'Stop'
                Add-Type -AssemblyName System.IO.Compression.FileSystem
                $archive = [IO.Compression.ZipFile]::OpenRead($env:LDTG_ZIP_TEST)
                try {
                    $expected = ConvertFrom-Json $env:LDTG_ZIP_EXPECTED
                    if ($archive.Entries.Count -ne $expected.Count) { throw 'entry count' }
                    foreach ($item in $expected) {
                        $entry = $archive.GetEntry($item.name)
                        if ($null -eq $entry) { throw 'missing entry' }
                        $reader = [IO.StreamReader]::new($entry.Open(), [Text.Encoding]::UTF8)
                        try { if ($reader.ReadToEnd() -cne $item.text) { throw 'wrong content' } }
                        finally { $reader.Dispose() }
                    }
                } finally { $archive.Dispose() }
            "#,
            ])
            .env("LDTG_ZIP_TEST", zip)
            .env("LDTG_ZIP_EXPECTED", expected.to_string())
            .status()
            .unwrap();
        assert!(
            status.success(),
            "native ZIP decoder must read exact paths and bytes"
        );
    }
}

#[tokio::test]
async fn folder_zip_can_download_an_empty_selected_directory() {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir(temp.path().join("leer")).unwrap();
    let state = test_state(temp.path());
    let res = response(state, "leer").await;
    let bytes = to_bytes(res.into_body(), 1024).await.unwrap();
    assert!(bytes.windows(5).any(|s| s == b"leer/"));
    assert_eq!(
        u16::from_le_bytes(
            bytes[bytes.len() - 12..bytes.len() - 10]
                .try_into()
                .unwrap()
        ),
        1
    );
}

#[tokio::test]
async fn folder_zip_streams_bounded_frames_and_stops_when_dropped() {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir(temp.path().join("folder")).unwrap();
    fs::write(
        temp.path().join("folder/big.bin"),
        vec![42; BUFFER_SIZE * 5],
    )
    .unwrap();
    let state = test_state(temp.path());
    let res = response(state.clone(), "folder").await;
    let mut body = res.into_body();
    let mut read = 0;
    while read < BUFFER_SIZE {
        let bytes = frame(&mut body)
            .await
            .unwrap()
            .unwrap()
            .into_data()
            .unwrap();
        assert!(bytes.len() <= BUFFER_SIZE);
        read += bytes.len();
    }
    assert!(read < BUFFER_SIZE * 5);
    assert_eq!(
        state.transfers.lock().await.last().unwrap().state,
        TransferState::Active
    );
    drop(body);
    wait_for_terminal(&state, TransferState::Cancelled).await;
    // Reclaim all per-session slots, including an entirely unpolled response.
    let res = response(state.clone(), "folder").await;
    drop(res);
    wait_for_terminal(&state, TransferState::Cancelled).await;
}

#[tokio::test]
async fn folder_zip_single_and_global_revocation_interrupt_body() {
    for revoke in [true, false] {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("data"), vec![5; BUFFER_SIZE * 2]).unwrap();
        let state = test_state(temp.path());
        let mut body = response(state.clone(), "").await.into_body();
        frame(&mut body).await.unwrap().unwrap();
        if revoke {
            let id = state
                .sessions
                .lock()
                .await
                .values()
                .next()
                .unwrap()
                .id
                .clone();
            state.revoke_session(&id).await;
        } else {
            state.revoke_all().await;
        }
        assert!(to_bytes(body, 2 * BUFFER_SIZE).await.is_err());
        wait_for_terminal(&state, TransferState::Cancelled).await;
    }
}

#[tokio::test]
async fn folder_zip_opens_files_lazily_and_rejects_changed_sources() {
    for size in [0, 3, 6] {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir(temp.path().join("folder")).unwrap();
        let source = temp.path().join("folder/data");
        fs::write(&source, b"four").unwrap();
        let state = test_state(temp.path());
        let res = response(state.clone(), "folder").await;
        // No file content has been consumed when response headers are available.
        fs::write(&source, vec![8; size]).unwrap();
        assert!(to_bytes(res.into_body(), 4096).await.is_err());
        wait_for_terminal(&state, TransferState::Failed).await;
    }
}

#[test]
fn folder_zip_limits_size_entries_depth_names_and_scan_work() {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir_all(temp.path().join("a/b")).unwrap();
    fs::write(temp.path().join("a/data"), "1234").unwrap();
    for limits in [
        Limits {
            bytes: 100,
            ..LIMITS
        },
        Limits {
            entries: 2,
            ..LIMITS
        },
        Limits { depth: 1, ..LIMITS },
        Limits { names: 2, ..LIMITS },
        Limits {
            scanned: 1,
            ..LIMITS
        },
    ] {
        assert_eq!(
            manifest(temp.path(), limits)
                .err()
                .unwrap()
                .into_response()
                .status(),
            StatusCode::PAYLOAD_TOO_LARGE
        );
    }
    assert!(manifest(temp.path(), LIMITS).is_ok());
    let root = fs::canonicalize(temp.path()).unwrap();
    let cancelled = ScanCancellation(Arc::new(AtomicBool::new(false)));
    let flag = cancelled.0.clone();
    drop(cancelled);
    for deadline in [Instant::now(), Instant::now() + SCAN_TIMEOUT] {
        assert_eq!(
            scan(
                &root,
                &RootAnchor::open(&root).unwrap(),
                "",
                &flag,
                deadline,
                LIMITS
            )
            .err()
            .unwrap()
            .into_response()
            .status(),
            StatusCode::REQUEST_TIMEOUT
        );
    }
    assert!(scan(
        &root,
        &RootAnchor::open(&root).unwrap(),
        "",
        &AtomicBool::new(false),
        Instant::now(),
        LIMITS
    )
    .is_err());
}

#[test]
fn folder_zip_rejects_unsafe_archive_components() {
    for name in [
        "",
        ".",
        "..",
        "../evil",
        "/root",
        "C:ads",
        "x\\y",
        "NUL.txt",
        "CON .txt",
        "com1",
        "LPT².txt",
        "bad.",
        "bad ",
        "a\n",
        "a\u{202e}b",
    ] {
        assert!(!safe_component(name), "{name:?}");
    }
    for name in ["Grüße 🐈.txt", "文件", "normal.txt", "com10", "a b"] {
        assert!(safe_component(name), "{name:?}");
    }
}

#[tokio::test]
async fn folder_zip_auth_paths_ranges_and_download_capacity_are_guarded() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("file.txt"), b"ok").unwrap();
    let state = test_state(temp.path());
    let app = router(state.clone());
    let req = request(Method::GET, "/api/v1/download-folder?path=", Body::empty());
    assert_eq!(
        app.clone().oneshot(req).await.unwrap().status(),
        StatusCode::UNAUTHORIZED
    );
    for path in [
        "..",
        "../outside",
        "/outside",
        "C:\\Windows",
        "file.txt:ads",
        "file.txt",
    ] {
        assert_eq!(
            response(state.clone(), path).await.status(),
            StatusCode::NOT_FOUND
        );
    }
    let (cookie, _) = session_headers(&state).await;
    for method in [Method::GET, Method::HEAD] {
        let mut req = request(
            method.clone(),
            "/api/v1/download-folder?path=",
            Body::empty(),
        );
        req.headers_mut()
            .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
        req.headers_mut()
            .insert(header::RANGE, HeaderValue::from_static("bytes=0-5"));
        assert_eq!(
            app.clone().oneshot(req).await.unwrap().status(),
            if method == Method::GET {
                StatusCode::RANGE_NOT_SATISFIABLE
            } else {
                StatusCode::METHOD_NOT_ALLOWED
            }
        );
    }
    // ZIPs and individual files share the same session concurrency budget.
    let mut held = Vec::new();
    for _ in 0..3 {
        let mut req = request(Method::GET, "/api/v1/download-folder?path=", Body::empty());
        req.headers_mut()
            .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
        let res = app.clone().oneshot(req).await.unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        held.push(res);
    }
    let mut req = request(Method::GET, "/api/v1/download?path=file.txt", Body::empty());
    req.headers_mut()
        .insert(header::COOKIE, HeaderValue::from_str(&cookie).unwrap());
    assert_eq!(
        app.oneshot(req).await.unwrap().status(),
        StatusCode::TOO_MANY_REQUESTS
    );
    drop(held);
}

#[cfg(windows)]
#[tokio::test]
async fn folder_zip_excludes_junctions_and_revalidates_them_after_scan() {
    let temp = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("secret"), b"OUTSIDE SECRET").unwrap();
    fs::create_dir(temp.path().join("folder")).unwrap();
    let junction = temp.path().join("folder/link");
    let create_junction = || {
        let status = std::process::Command::new("powershell.exe").args(["-NoProfile", "-NonInteractive", "-Command", "$ErrorActionPreference='Stop'; New-Item -ItemType Junction -Path $env:LDTG_LINK_TEST -Value $env:LDTG_LINK_TARGET | Out-Null"])
            .env("LDTG_LINK_TEST", &junction).env("LDTG_LINK_TARGET", outside.path()).status().unwrap();
        assert!(status.success());
    };
    create_junction();
    let state = test_state(temp.path());
    let bytes = to_bytes(response(state.clone(), "folder").await.into_body(), 4096)
        .await
        .unwrap();
    assert!(!bytes.windows(6).any(|s| s == b"secret"));
    assert_eq!(
        response(state.clone(), "folder/link").await.status(),
        StatusCode::NOT_FOUND
    );
    fs::remove_dir(&junction).unwrap();
    fs::create_dir(&junction).unwrap();
    fs::write(junction.join("secret"), b"inside").unwrap();
    let res = response(state.clone(), "folder").await;
    fs::remove_file(junction.join("secret")).unwrap();
    fs::remove_dir(&junction).unwrap();
    create_junction();
    assert!(to_bytes(res.into_body(), 4096).await.is_err());
    wait_for_terminal(&state, TransferState::Failed).await;
    fs::remove_dir(junction).unwrap();
}

#[cfg(windows)]
#[tokio::test]
#[ignore = "requires Windows Developer Mode or SeCreateSymbolicLinkPrivilege; junction coverage runs unconditionally"]
async fn folder_zip_excludes_file_and_directory_symlinks() {
    let temp = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("secret"), b"OUTSIDE SECRET").unwrap();
    std::os::windows::fs::symlink_file(
        outside.path().join("secret"),
        temp.path().join("file-link"),
    )
    .unwrap();
    std::os::windows::fs::symlink_dir(outside.path(), temp.path().join("dir-link")).unwrap();
    let state = test_state(temp.path());
    for path in ["file-link", "dir-link", "dir-link/secret"] {
        assert_eq!(
            response(state.clone(), path).await.status(),
            StatusCode::NOT_FOUND
        );
    }
    let bytes = to_bytes(response(state, "").await.into_body(), 4096)
        .await
        .unwrap();
    assert!(!bytes.windows(6).any(|s| s == b"secret"));
    assert!(!bytes.windows(4).any(|s| s == b"link"));
    fs::remove_file(temp.path().join("file-link")).unwrap();
    fs::remove_dir(temp.path().join("dir-link")).unwrap();
}
