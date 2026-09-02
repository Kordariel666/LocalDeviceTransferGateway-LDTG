use super::journal::{smooth_transfer_speed, TRANSFER_EVENT_BYTES};
use super::*;
use crate::domain::settings::ShareSettings;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::sync::Condvar;

fn test_state(root: &Path) -> Result<TransferServiceState, String> {
    let settings = AppSettings {
        upload_share: ShareSettings {
            enabled: true,
            path: root.display().to_string(),
        },
        ..AppSettings::default()
    };
    let interface = NetworkInterfaceInfo {
        id: "lan|192.168.10.2".into(),
        name: "lan".into(),
        profile_name: "Testnetz".into(),
        address: Ipv4Addr::new(192, 168, 10, 2),
        prefix_length: 24,
        network_id: "lan-test".into(),
        category: "Privat".into(),
        profile_resolved: true,
        preferred: true,
        netmask: Ipv4Addr::new(255, 255, 255, 0),
    };
    TransferServiceState::new(
        settings,
        interface,
        ShareRoots {
            download: Some(root.to_path_buf()),
            upload: Some(root.to_path_buf()),
        },
        None,
    )
}

#[tokio::test]
async fn status_waits_for_complete_session_and_transfer_snapshots() {
    let temp = tempfile::tempdir().unwrap();
    let state = Arc::new(test_state(temp.path()).unwrap());
    let session = state
        .create_session(
            IpAddr::V4(Ipv4Addr::new(192, 168, 10, 20)),
            "Browser".into(),
        )
        .await
        .unwrap();
    let transfer_id = Uuid::new_v4().to_string();
    state
        .record_transfer_with_id(
            &transfer_id,
            &session.id,
            TransferDirection::Download,
            "status.bin",
            2048,
        )
        .await;
    let sessions = state.sessions.lock().await;
    let transfers = state.transfers.lock().await;
    let mut pending = Box::pin(state.status());
    assert!(
        tokio::time::timeout(Duration::from_millis(20), &mut pending)
            .await
            .is_err(),
        "status must wait instead of fabricating empty snapshots"
    );
    drop(sessions);
    assert!(
        tokio::time::timeout(Duration::from_millis(20), &mut pending)
            .await
            .is_err(),
        "status must also wait for the transfer snapshot"
    );
    drop(transfers);
    let status = tokio::time::timeout(Duration::from_secs(1), pending)
        .await
        .expect("status snapshot should complete after both locks are released");
    assert_eq!(status.sessions.len(), 1);
    assert_eq!(status.sessions[0].id, session.id);
    assert_eq!(status.transfers.len(), 1);
    assert_eq!(status.transfers[0].id, transfer_id);
    assert_eq!(status.transfers[0].session_id, session.id);
}

#[test]
fn describes_common_clients_without_exposing_raw_user_agents() {
    let iphone = "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 Version/18.0 Mobile/15E148 Safari/604.1";
    let android = "Mozilla/5.0 (Linux; Android 15; Pixel 9) AppleWebKit/537.36 Chrome/128.0 Mobile Safari/537.36";
    let edge = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/128.0 Safari/537.36 Edg/128.0";

    assert_eq!(describe_user_agent(iphone), "Safari auf iPhone");
    assert_eq!(describe_user_agent(android), "Chrome auf Android");
    assert_eq!(describe_user_agent(edge), "Microsoft Edge auf Windows");
    assert_eq!(
        describe_user_agent("komplett-unbekannter-client"),
        "Unbekannter Browser"
    );
}

#[tokio::test]
async fn transfer_progress_notifications_are_byte_throttled_but_terminal_updates_are_immediate() {
    let temp = tempfile::tempdir().unwrap();
    let state = test_state(temp.path()).unwrap();
    let id = state
        .record_transfer(
            TransferDirection::Download,
            "gross.bin",
            TRANSFER_EVENT_BYTES * 2,
        )
        .await;

    state.update_transfer(&id, 64 * 1024, None).await;
    assert_eq!(
        state.transfers.lock().await[0].transferred_bytes,
        64 * 1024,
        "the authoritative state still tracks every progress update"
    );
    assert_eq!(
        state
            .transfer_notifications
            .lock()
            .await
            .get(&id)
            .unwrap()
            .transferred_bytes,
        0,
        "small immediate progress must not advance the notification watermark"
    );

    state.update_transfer(&id, TRANSFER_EVENT_BYTES, None).await;
    assert_eq!(
        state
            .transfer_notifications
            .lock()
            .await
            .get(&id)
            .unwrap()
            .transferred_bytes,
        TRANSFER_EVENT_BYTES,
        "one MiB of progress must advance the notification watermark"
    );

    state
        .update_transfer(&id, TRANSFER_EVENT_BYTES + 1, Some(TransferState::Complete))
        .await;
    assert!(
        !state.transfer_notifications.lock().await.contains_key(&id),
        "terminal updates must emit immediately and release throttle state"
    );
}

#[tokio::test]
async fn clearing_transfer_history_preserves_active_transfers() {
    let temp = tempfile::tempdir().unwrap();
    let state = test_state(temp.path()).unwrap();
    let active = state
        .record_transfer(TransferDirection::Upload, "aktiv.bin", 100)
        .await;
    let complete = state
        .record_transfer(TransferDirection::Download, "fertig.bin", 200)
        .await;
    state
        .update_transfer(&complete, 200, Some(TransferState::Complete))
        .await;

    let finished = state
        .transfers
        .lock()
        .await
        .iter()
        .find(|item| item.id == complete)
        .cloned()
        .unwrap();
    let first_finished_at = finished.finished_at.clone();
    assert!(first_finished_at.is_some());
    assert_eq!(first_finished_at, Some(finished.updated_at));
    state
        .update_transfer(&complete, 200, Some(TransferState::Complete))
        .await;
    assert_eq!(
        state.transfers.lock().await[1].finished_at,
        first_finished_at,
        "repeated terminal updates must preserve the original end time"
    );

    assert_eq!(state.clear_transfer_history().await, 1);
    let status = state.status().await;
    assert_eq!(status.active_transfers, 1);
    assert_eq!(status.transfers.len(), 1);
    assert_eq!(status.transfers[0].id, active);
    assert_eq!(status.transfers[0].finished_at, None);
}

#[tokio::test]
async fn transfer_progress_models_monotonic_smoothed_speed_and_timestamps() {
    let first_exact = smooth_transfer_speed(None, 1024, Duration::from_secs(1)).unwrap();
    let second_exact =
        smooth_transfer_speed(Some(first_exact), 2048, Duration::from_secs(1)).unwrap();
    assert_eq!(first_exact, 1024.0);
    assert_eq!(second_exact, 1280.0);

    let temp = tempfile::tempdir().unwrap();
    let state = test_state(temp.path()).unwrap();
    let id = state
        .record_transfer(TransferDirection::Upload, "tempo.bin", 8 * 1024)
        .await;
    {
        let mut notifications = state.transfer_notifications.lock().await;
        notifications.get_mut(&id).unwrap().sampled_at = Instant::now() - Duration::from_secs(1);
    }
    state.update_transfer(&id, 1024, None).await;
    let first = state.transfers.lock().await[0].clone();
    let first_speed = first.bytes_per_second.unwrap();
    assert!(first_speed.is_finite() && first_speed > 0.0);
    assert_eq!(first.speed_sample_count, 1);
    assert!(first.last_progress_at.is_some());
    assert!(first.started_at <= first.last_progress_at.clone().unwrap());

    {
        let mut notifications = state.transfer_notifications.lock().await;
        notifications.get_mut(&id).unwrap().sampled_at = Instant::now() - Duration::from_secs(1);
    }
    state.update_transfer(&id, 3 * 1024, None).await;
    let second = state.transfers.lock().await[0].clone();
    let second_speed = second.bytes_per_second.unwrap();
    assert!(second_speed.is_finite() && second_speed > 0.0);
    assert_eq!(second.speed_sample_count, 2);
    assert_eq!(second.transferred_bytes, 3 * 1024);
}

#[test]
fn preserves_unmanaged_dmdc_directory() {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir(temp.path().join(PARTIAL_DIR_NAME)).unwrap();
    let user_file = temp.path().join(PARTIAL_DIR_NAME).join("user.txt");
    fs::write(&user_file, b"behalten").unwrap();
    assert!(test_state(temp.path()).is_err());
    assert_eq!(fs::read(user_file).unwrap(), b"behalten");
}

#[test]
fn preserves_untracked_uuid_partials_even_with_public_marker() {
    let temp = tempfile::tempdir().unwrap();
    let state = test_state(temp.path()).unwrap();
    let partial_dir = state.partial_dir.as_ref().unwrap();
    let owned = partial_dir.join(format!("{}.part", Uuid::new_v4()));
    let unrelated = partial_dir.join("notes.part");
    fs::write(&owned, b"teil").unwrap();
    fs::write(&unrelated, b"behalten").unwrap();
    cleanup_owned_partials(partial_dir).unwrap();
    assert_eq!(fs::read(owned).unwrap(), b"teil");
    assert_eq!(fs::read(unrelated).unwrap(), b"behalten");
}

#[test]
fn inbox_limits_include_files_that_existed_before_service_start() {
    let temp = tempfile::tempdir().unwrap();
    fs::write(temp.path().join("existing.bin"), b"1234").unwrap();
    let settings = AppSettings {
        upload_share: ShareSettings {
            enabled: true,
            path: temp.path().display().to_string(),
        },
        max_upload_bytes: Some(5),
        max_inbox_bytes: 5,
        max_inbox_files: 1,
        ..AppSettings::default()
    };
    let interface = NetworkInterfaceInfo {
        id: "lan|192.168.10.2".into(),
        name: "lan".into(),
        profile_name: "Testnetz".into(),
        address: Ipv4Addr::new(192, 168, 10, 2),
        prefix_length: 24,
        network_id: "lan-test".into(),
        category: "Privat".into(),
        profile_resolved: true,
        preferred: true,
        netmask: Ipv4Addr::new(255, 255, 255, 0),
    };
    let state = TransferServiceState::new(
        settings,
        interface,
        ShareRoots {
            download: None,
            upload: Some(fs::canonicalize(temp.path()).unwrap()),
        },
        None,
    )
    .unwrap();

    assert_eq!(state.reserve_upload_object(), Err("files"));
    assert_eq!(state.reserve_upload_bytes(2), Err("bytes"));
    assert!(state.reserve_upload_bytes(1).is_ok());
    state.release_upload_bytes(1);
}

#[test]
fn access_codes_have_eight_decimal_digits() {
    let code = new_code();
    assert_eq!(code.len(), ACCESS_CODE_DIGITS);
    assert!(code.bytes().all(|value| value.is_ascii_digit()));
}

#[test]
fn global_cooldown_is_checked_before_the_access_code() {
    let temp = tempfile::tempdir().unwrap();
    let state = test_state(temp.path()).unwrap();
    let now = Instant::now();
    state.auth_attempts.lock().unwrap().global_blocked_until = Some(now + AUTH_BLOCK_DURATION);
    let code = state.access_code.read().unwrap().clone();
    assert_eq!(
        state.verify_access_code_at(IpAddr::V4(Ipv4Addr::new(192, 168, 10, 50)), &code, now,),
        AuthDecision::GlobalBlocked
    );
    assert_eq!(
        state.verify_access_code_at(IpAddr::V4(Ipv4Addr::new(192, 168, 10, 51)), "00000000", now,),
        AuthDecision::GlobalBlocked
    );
}

#[test]
fn auth_attempt_records_expire_and_remain_capacity_bounded() {
    let temp = tempfile::tempdir().unwrap();
    let state = test_state(temp.path()).unwrap();
    let expired_address = IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1));
    assert_eq!(
        state.verify_access_code(expired_address, ""),
        AuthDecision::Invalid
    );
    {
        let mut throttle = state.auth_attempts.lock().unwrap();
        throttle
            .attempts
            .get_mut(&expired_address)
            .unwrap()
            .last_seen = Instant::now() - AUTH_ATTEMPT_TTL - Duration::from_secs(1);
    }
    assert_eq!(
        state.verify_access_code(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 2)), ""),
        AuthDecision::Invalid
    );
    assert!(!state
        .auth_attempts
        .lock()
        .unwrap()
        .attempts
        .contains_key(&expired_address));

    let now = Instant::now();
    let mut throttle = state.auth_attempts.lock().unwrap();
    throttle.attempts.clear();
    for index in 0..MAX_AUTH_ATTEMPT_RECORDS {
        throttle.attempts.insert(
            IpAddr::V6(Ipv6Addr::from(index as u128 + 1)),
            AttemptRecord {
                failures: 1,
                blocked_until: None,
                last_seen: now,
            },
        );
    }
    throttle.global_failures = 0;
    throttle.global_window_started = now;
    drop(throttle);
    assert_eq!(
        state.verify_access_code(IpAddr::V6(Ipv6Addr::from(5_000_u128)), ""),
        AuthDecision::Invalid
    );
    let throttle = state.auth_attempts.lock().unwrap();
    assert_eq!(throttle.attempts.len(), MAX_AUTH_ATTEMPT_RECORDS);
    assert_eq!(throttle.global_failures, 1);
}

#[tokio::test]
async fn limits_concurrent_requests_per_address() {
    let temp = tempfile::tempdir().unwrap();
    let state = test_state(temp.path()).unwrap();
    let address = IpAddr::V4(Ipv4Addr::new(192, 168, 10, 50));
    let mut permits = Vec::new();
    for _ in 0..MAX_REQUESTS_PER_ADDRESS {
        permits.push(state.begin_request(address, true).await.unwrap());
    }
    assert!(state.begin_request(address, true).await.is_none());
    drop(permits.pop());
    assert!(state.begin_request(address, true).await.is_some());
}

#[tokio::test]
async fn partitions_authenticated_request_capacity() {
    let temp = tempfile::tempdir().unwrap();
    let state = test_state(temp.path()).unwrap();
    let mut permits = Vec::new();
    for index in 0..MAX_AUTHENTICATED_REQUESTS_GLOBAL {
        permits.push(
            state
                .begin_request(IpAddr::V6(Ipv6Addr::from(index as u128 + 1)), true)
                .await
                .unwrap(),
        );
    }
    assert!(state
        .begin_request(IpAddr::V6(Ipv6Addr::from(10_000_u128)), true)
        .await
        .is_none());
    assert!(state
        .begin_request(IpAddr::V6(Ipv6Addr::from(20_000_u128)), false)
        .await
        .is_some());
    drop(permits.pop());
    assert!(state
        .begin_request(IpAddr::V6(Ipv6Addr::from(10_000_u128)), true)
        .await
        .is_some());
}

#[test]
fn limits_upload_chunks_per_id_and_globally_before_body_work() {
    let temp = tempfile::tempdir().unwrap();
    let state = test_state(temp.path()).unwrap();
    let records = (0..=MAX_UPLOAD_CHUNKS_ACTIVE)
        .map(|index| UploadRecord {
            id: format!("upload-{index}"),
            owner_session: format!("session-{index}"),
            owner_address: IpAddr::V6(Ipv6Addr::from(index as u128 + 1)),
            name: format!("file-{index}.bin"),
            declared_size: 1,
            offset: 0,
            last_modified: 1,
            client_token: format!("token-{index}"),
            created_at: Instant::now(),
            last_activity: Instant::now(),
            cancelled: false,
            finalizing: false,
            cancel_signal: Arc::new(AtomicBool::new(false)),
            chunk_slots: Arc::new(Semaphore::new(1)),
            partial_path: temp.path().join(format!("upload-{index}.part")),
            partial_file: Arc::new(
                fs::File::create(temp.path().join(format!("upload-{index}.part"))).unwrap(),
            ),
            transfer_id: format!("transfer-{index}"),
        })
        .collect::<Vec<_>>();

    let first = state.begin_upload_chunk(&records[0]).unwrap();
    assert!(matches!(
        state.begin_upload_chunk(&records[0]),
        Err("upload")
    ));
    let mut leases = vec![first];
    for record in &records[1..MAX_UPLOAD_CHUNKS_ACTIVE] {
        leases.push(state.begin_upload_chunk(record).unwrap());
    }
    assert!(matches!(
        state.begin_upload_chunk(&records[MAX_UPLOAD_CHUNKS_ACTIVE]),
        Err("global")
    ));
    drop(leases.pop());
    assert!(state
        .begin_upload_chunk(&records[MAX_UPLOAD_CHUNKS_ACTIVE])
        .is_ok());
}

#[tokio::test]
async fn limits_upload_io_globally_and_per_address() {
    let temp = tempfile::tempdir().unwrap();
    let state = test_state(temp.path()).unwrap();
    let first_address = IpAddr::V4(Ipv4Addr::new(192, 168, 10, 50));
    let first = state.begin_upload_io(first_address).await.unwrap();
    let second = state.begin_upload_io(first_address).await.unwrap();
    assert!(state.begin_upload_io(first_address).await.is_none());

    let mut permits = vec![first, second];
    for index in 0..(MAX_UPLOAD_IO_ACTIVE - MAX_UPLOAD_IO_PER_ADDRESS) {
        permits.push(
            state
                .begin_upload_io(IpAddr::V6(Ipv6Addr::from(index as u128 + 1)))
                .await
                .unwrap(),
        );
    }
    assert!(state
        .begin_upload_io(IpAddr::V6(Ipv6Addr::from(100_u128)))
        .await
        .is_none());
    drop(permits.pop());
    assert!(state
        .begin_upload_io(IpAddr::V6(Ipv6Addr::from(101_u128)))
        .await
        .is_some());
}

#[tokio::test]
async fn dropped_upload_io_waiters_do_not_release_running_blocking_work() {
    let temp = tempfile::tempdir().unwrap();
    let state = test_state(temp.path()).unwrap();
    let release = Arc::new((StdMutex::new(false), Condvar::new()));
    let (started, started_rx) = std::sync::mpsc::channel();
    for index in 0..MAX_UPLOAD_IO_ACTIVE {
        let permit = state
            .begin_upload_io(IpAddr::V6(Ipv6Addr::from(index as u128 + 1)))
            .await
            .unwrap();
        let release = release.clone();
        let started = started.clone();
        let task = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            started.send(()).unwrap();
            let (flag, wake) = &*release;
            let mut released = flag.lock().unwrap();
            while !*released {
                released = wake.wait(released).unwrap();
            }
        });
        drop(task);
    }
    for _ in 0..MAX_UPLOAD_IO_ACTIVE {
        started_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("blocking upload I/O task must start");
    }
    assert!(state
        .begin_upload_io(IpAddr::V6(Ipv6Addr::from(100_u128)))
        .await
        .is_none());

    let (flag, wake) = &*release;
    *flag.lock().unwrap() = true;
    wake.notify_all();
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            if state
                .begin_upload_io(IpAddr::V6(Ipv6Addr::from(101_u128)))
                .await
                .is_some()
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("capacity must recover after the blocking upload I/O exits");
}

#[tokio::test]
async fn limits_parallel_directory_work() {
    let temp = tempfile::tempdir().unwrap();
    let state = test_state(temp.path()).unwrap();
    let mut permits = Vec::new();
    for index in 0..MAX_DIRECTORY_LISTINGS_ACTIVE {
        permits.push(
            state
                .begin_listing(
                    &format!("session-{index}"),
                    IpAddr::V6(Ipv6Addr::from(index as u128 + 1)),
                )
                .await
                .unwrap(),
        );
    }
    assert!(state
        .begin_listing("overflow", IpAddr::V6(Ipv6Addr::from(100_u128)))
        .await
        .is_none());
    drop(permits.pop());
    assert!(state
        .begin_listing("recovered", IpAddr::V6(Ipv6Addr::from(101_u128)))
        .await
        .is_some());
}

#[tokio::test]
async fn one_session_or_address_cannot_consume_all_active_listing_slots() {
    let temp = tempfile::tempdir().unwrap();
    let state = test_state(temp.path()).unwrap();
    let address = IpAddr::V4(Ipv4Addr::new(192, 168, 10, 50));
    let first = state.begin_listing("session-a", address).await.unwrap();
    assert!(state.begin_listing("session-a", address).await.is_none());
    let second = state.begin_listing("session-b", address).await.unwrap();
    assert!(state.begin_listing("session-c", address).await.is_none());
    assert!(state
        .begin_listing("session-c", IpAddr::V4(Ipv4Addr::new(192, 168, 10, 51)),)
        .await
        .is_some());
    drop((first, second));
}

#[tokio::test]
async fn dropped_listing_waiters_do_not_release_running_blocking_work() {
    let temp = tempfile::tempdir().unwrap();
    let state = test_state(temp.path()).unwrap();
    let release = Arc::new((StdMutex::new(false), Condvar::new()));
    let (started, started_rx) = std::sync::mpsc::channel();
    for index in 0..MAX_DIRECTORY_LISTINGS_ACTIVE {
        let permit = state
            .begin_listing(
                &format!("session-{index}"),
                IpAddr::V6(Ipv6Addr::from(index as u128 + 1)),
            )
            .await
            .unwrap();
        let release = release.clone();
        let started = started.clone();
        let task = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            started.send(()).unwrap();
            let (flag, wake) = &*release;
            let mut released = flag.lock().unwrap();
            while !*released {
                released = wake.wait(released).unwrap();
            }
        });
        drop(task);
    }
    for _ in 0..MAX_DIRECTORY_LISTINGS_ACTIVE {
        started_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("blocking listing task must start");
    }
    assert!(state
        .begin_listing("overflow", IpAddr::V6(Ipv6Addr::from(100_u128)))
        .await
        .is_none());

    let (flag, wake) = &*release;
    *flag.lock().unwrap() = true;
    wake.notify_all();
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            if state
                .begin_listing("recovered", IpAddr::V6(Ipv6Addr::from(101_u128)))
                .await
                .is_some()
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("capacity must recover after the blocking work exits");
}

#[tokio::test]
async fn dropped_lookup_waiters_do_not_release_running_blocking_work() {
    let temp = tempfile::tempdir().unwrap();
    let state = test_state(temp.path()).unwrap();
    let release = Arc::new((StdMutex::new(false), Condvar::new()));
    let (started, started_rx) = std::sync::mpsc::channel();
    for index in 0..MAX_FILESYSTEM_LOOKUPS_ACTIVE {
        let permit = state
            .begin_filesystem_lookup(IpAddr::V6(Ipv6Addr::from(index as u128 + 1)))
            .await
            .unwrap();
        let release = release.clone();
        let started = started.clone();
        let task = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            started.send(()).unwrap();
            let (flag, wake) = &*release;
            let mut released = flag.lock().unwrap();
            while !*released {
                released = wake.wait(released).unwrap();
            }
        });
        drop(task);
    }
    for _ in 0..MAX_FILESYSTEM_LOOKUPS_ACTIVE {
        started_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("blocking lookup task must start");
    }
    assert!(state
        .begin_filesystem_lookup(IpAddr::V6(Ipv6Addr::from(100_u128)))
        .await
        .is_none());

    let (flag, wake) = &*release;
    *flag.lock().unwrap() = true;
    wake.notify_all();
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            if state
                .begin_filesystem_lookup(IpAddr::V6(Ipv6Addr::from(101_u128)))
                .await
                .is_some()
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("capacity must recover after the blocking lookup exits");
}

#[tokio::test]
async fn limits_directory_cursors_per_session_without_blocking_other_sessions() {
    let temp = tempfile::tempdir().unwrap();
    let state = test_state(temp.path()).unwrap();
    let session_a = state
        .create_session(
            IpAddr::V4(Ipv4Addr::new(192, 168, 10, 50)),
            "session-a".into(),
        )
        .await
        .unwrap();
    let session_b = state
        .create_session(
            IpAddr::V4(Ipv4Addr::new(192, 168, 10, 51)),
            "session-b".into(),
        )
        .await
        .unwrap();
    for index in 0..MAX_DIRECTORY_LISTINGS_PER_SESSION {
        state
            .create_directory_listing(
                &session_a,
                format!("path-{index}"),
                String::new(),
                temp.path().to_path_buf(),
                fs::read_dir(temp.path()).unwrap(),
            )
            .await
            .unwrap();
    }
    assert!(matches!(
        state
            .create_directory_listing(
                &session_a,
                "overflow".into(),
                String::new(),
                temp.path().to_path_buf(),
                fs::read_dir(temp.path()).unwrap(),
            )
            .await,
        Err("session")
    ));
    assert!(state
        .create_directory_listing(
            &session_b,
            "other".into(),
            String::new(),
            temp.path().to_path_buf(),
            fs::read_dir(temp.path()).unwrap(),
        )
        .await
        .is_ok());
}

#[tokio::test]
async fn limits_persistent_directory_cursors_per_address() {
    let temp = tempfile::tempdir().unwrap();
    let state = test_state(temp.path()).unwrap();
    let address = IpAddr::V4(Ipv4Addr::new(192, 168, 10, 50));
    let sessions = [
        state
            .create_session(address, "session-0".into())
            .await
            .unwrap(),
        state
            .create_session(address, "session-1".into())
            .await
            .unwrap(),
    ];
    for index in 0..MAX_DIRECTORY_LISTINGS_PER_ADDRESS {
        state
            .create_directory_listing(
                &sessions[index / MAX_DIRECTORY_LISTINGS_PER_SESSION],
                format!("path-{index}"),
                String::new(),
                temp.path().to_path_buf(),
                fs::read_dir(temp.path()).unwrap(),
            )
            .await
            .unwrap();
    }
    let overflow = state
        .create_session(address, "session-overflow".into())
        .await
        .unwrap();
    assert!(matches!(
        state
            .create_directory_listing(
                &overflow,
                "overflow".into(),
                String::new(),
                temp.path().to_path_buf(),
                fs::read_dir(temp.path()).unwrap(),
            )
            .await,
        Err("address")
    ));
    let other = state
        .create_session(
            IpAddr::V4(Ipv4Addr::new(192, 168, 10, 51)),
            "other-address".into(),
        )
        .await
        .unwrap();
    assert!(state
        .create_directory_listing(
            &other,
            "other".into(),
            String::new(),
            temp.path().to_path_buf(),
            fs::read_dir(temp.path()).unwrap(),
        )
        .await
        .is_ok());
}

#[tokio::test]
async fn cursor_lookup_does_not_refresh_activity_before_work_admission() {
    let temp = tempfile::tempdir().unwrap();
    let state = test_state(temp.path()).unwrap();
    let session = state
        .create_session(
            IpAddr::V4(Ipv4Addr::new(192, 168, 10, 50)),
            "session".into(),
        )
        .await
        .unwrap();
    let (id, listing) = state
        .create_directory_listing(
            &session,
            String::new(),
            String::new(),
            temp.path().to_path_buf(),
            fs::read_dir(temp.path()).unwrap(),
        )
        .await
        .unwrap();
    let before = Instant::now() - Duration::from_secs(30);
    listing.cursor.lock().unwrap().last_activity = before;
    assert!(state
        .directory_listing(&id, &session.id, "", "")
        .await
        .is_some());
    assert_eq!(listing.cursor.lock().unwrap().last_activity, before);
}

#[tokio::test]
async fn session_limits_reject_new_sessions_without_evicting_existing_ones() {
    let temp = tempfile::tempdir().unwrap();
    let state = test_state(temp.path()).unwrap();
    let first_address = IpAddr::V4(Ipv4Addr::new(192, 168, 10, 1));
    let first = state
        .create_session(first_address, "first".into())
        .await
        .unwrap();
    let mut lease = state
        .begin_download(&first.id, first.address, "datei", 10)
        .await
        .unwrap();

    for index in 2..=MAX_SESSIONS_GLOBAL {
        let address = IpAddr::V6(Ipv6Addr::from(index as u128));
        state
            .create_session(address, format!("client-{index}"))
            .await
            .unwrap();
    }
    assert!(matches!(
        state
            .create_session(IpAddr::V6(Ipv6Addr::LOCALHOST), "overflow".into())
            .await,
        Err(SessionCreateError::GlobalLimit)
    ));
    assert!(state.session_is_active(&first).await);
    assert!(lease.cancel.has_changed().is_ok_and(|changed| !changed));

    state.revoke_all().await;
    for index in 0..MAX_SESSIONS_PER_ADDRESS {
        state
            .create_session(first_address, format!("same-{index}"))
            .await
            .unwrap();
    }
    assert!(matches!(
        state.create_session(first_address, "too-many".into()).await,
        Err(SessionCreateError::AddressLimit)
    ));
    let _ = lease.cancel.changed().await;
}

#[tokio::test]
async fn expired_sessions_are_reclaimed_atomically_and_cancel_resources() {
    let temp = tempfile::tempdir().unwrap();
    let state = test_state(temp.path()).unwrap();
    let address = IpAddr::V4(Ipv4Addr::new(192, 168, 10, 50));
    let expired = state
        .create_session(address, "expired".into())
        .await
        .unwrap();
    let mut lease = state
        .begin_download(&expired.id, address, "datei", 1)
        .await
        .unwrap();
    for index in 1..MAX_SESSIONS_PER_ADDRESS {
        state
            .create_session(address, format!("fresh-{index}"))
            .await
            .unwrap();
    }
    state
        .sessions
        .lock()
        .await
        .get_mut(&expired.token)
        .unwrap()
        .last_activity_instant = Instant::now() - SESSION_IDLE_TIMEOUT - Duration::from_secs(1);

    let replacement = state
        .create_session(address, "replacement".into())
        .await
        .expect("stale capacity must be reclaimed during admission");
    assert!(!state.session_is_active(&expired).await);
    assert!(state.session_is_active(&replacement).await);
    lease.cancel.changed().await.unwrap();
    assert!(*lease.cancel.borrow());
    assert_eq!(state.sessions.lock().await.len(), MAX_SESSIONS_PER_ADDRESS);
}

#[tokio::test]
async fn authentication_rejects_idle_and_absolute_session_expiry() {
    let temp = tempfile::tempdir().unwrap();
    let state = test_state(temp.path()).unwrap();
    let address = IpAddr::V4(Ipv4Addr::new(192, 168, 10, 50));
    let idle = state.create_session(address, "idle".into()).await.unwrap();
    state
        .sessions
        .lock()
        .await
        .get_mut(&idle.token)
        .unwrap()
        .last_activity_instant = Instant::now() - SESSION_IDLE_TIMEOUT - Duration::from_secs(1);
    assert!(state.authenticate(&idle.token, address).await.is_none());

    let absolute = state
        .create_session(address, "absolute".into())
        .await
        .unwrap();
    {
        let mut sessions = state.sessions.lock().await;
        let record = sessions.get_mut(&absolute.token).unwrap();
        record.created_at_instant = Instant::now() - SESSION_MAX_LIFETIME - Duration::from_secs(1);
        record.last_activity_instant = Instant::now();
    }
    assert!(state.authenticate(&absolute.token, address).await.is_none());
}

#[tokio::test]
async fn revoke_all_does_not_remove_resources_of_a_concurrently_new_session() {
    let temp = tempfile::tempdir().unwrap();
    let state = Arc::new(test_state(temp.path()).unwrap());
    state
        .create_session(IpAddr::V4(Ipv4Addr::new(192, 168, 10, 49)), "old".into())
        .await
        .unwrap();
    let filesystem = state.upload_fs_lock.clone().lock_owned().await;
    let revoke_state = state.clone();
    let revoke = tokio::spawn(async move { revoke_state.revoke_all().await });
    tokio::time::timeout(Duration::from_secs(1), async {
        while !state.sessions.lock().await.is_empty() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("bulk revoke must clear its initial session set");

    let fresh = state
        .create_session(IpAddr::V4(Ipv4Addr::new(192, 168, 10, 50)), "fresh".into())
        .await
        .unwrap();
    let (cursor_id, _) = state
        .create_directory_listing(
            &fresh,
            String::new(),
            String::new(),
            temp.path().to_path_buf(),
            fs::read_dir(temp.path()).unwrap(),
        )
        .await
        .unwrap();
    drop(filesystem);
    revoke.await.unwrap();

    assert!(state.session_is_active(&fresh).await);
    assert!(state
        .directory_listing(&cursor_id, &fresh.id, "", "")
        .await
        .is_some());
}

#[tokio::test]
async fn cursor_creation_is_ordered_with_concurrent_session_revocation() {
    let temp = tempfile::tempdir().unwrap();
    let state = Arc::new(test_state(temp.path()).unwrap());
    let session = state
        .create_session(
            IpAddr::V4(Ipv4Addr::new(192, 168, 10, 50)),
            "session".into(),
        )
        .await
        .unwrap();
    let listings = state.directory_listings.lock().await;
    let create_state = state.clone();
    let create_session = session.clone();
    let root = temp.path().to_path_buf();
    let create = tokio::spawn(async move {
        create_state
            .create_directory_listing(
                &create_session,
                String::new(),
                String::new(),
                root.clone(),
                fs::read_dir(root).unwrap(),
            )
            .await
    });
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            if state.sessions.try_lock().is_err() {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("cursor creation must hold the session admission lock");
    let revoke_state = state.clone();
    let session_id = session.id.clone();
    let revoke = tokio::spawn(async move { revoke_state.revoke_session(&session_id).await });
    drop(listings);

    let (cursor_id, _) = create.await.unwrap().unwrap();
    assert!(revoke.await.unwrap());
    assert!(!state.session_is_active(&session).await);
    assert!(state
        .directory_listing(&cursor_id, &session.id, "", "")
        .await
        .is_none());
}

#[tokio::test]
async fn limits_downloads_per_session() {
    let temp = tempfile::tempdir().unwrap();
    let state = test_state(temp.path()).unwrap();
    let address = IpAddr::V4(Ipv4Addr::new(192, 168, 10, 50));
    let first = state
        .begin_download("session", address, "a", 1)
        .await
        .unwrap();
    let second = state
        .begin_download("session", address, "b", 1)
        .await
        .unwrap();
    let third = state
        .begin_download("session", address, "c", 1)
        .await
        .unwrap();
    assert!(matches!(
        state.begin_download("session", address, "d", 1).await,
        Err("session")
    ));
    drop((first, second, third));
}

#[tokio::test]
async fn one_address_cannot_consume_all_global_download_slots() {
    let temp = tempfile::tempdir().unwrap();
    let state = test_state(temp.path()).unwrap();
    let address = IpAddr::V4(Ipv4Addr::new(192, 168, 10, 50));
    let other = IpAddr::V4(Ipv4Addr::new(192, 168, 10, 51));
    let mut leases = Vec::new();
    for index in 0..MAX_DOWNLOADS_PER_ADDRESS {
        leases.push(
            state
                .begin_download(&format!("session-{index}"), address, "datei", 1)
                .await
                .unwrap(),
        );
    }
    assert!(matches!(
        state.begin_download("overflow", address, "datei", 1).await,
        Err("address")
    ));
    leases.push(
        state
            .begin_download("other", other, "datei", 1)
            .await
            .expect("a second address retains global capacity"),
    );
}

#[tokio::test]
async fn download_lease_has_an_absolute_deadline() {
    let temp = tempfile::tempdir().unwrap();
    let state = test_state(temp.path()).unwrap();
    let address = IpAddr::V4(Ipv4Addr::new(192, 168, 10, 50));
    let mut lease = state
        .begin_download("session", address, "datei", 1)
        .await
        .unwrap();
    lease.started_at = Instant::now() - DOWNLOAD_MAX_DURATION - Duration::from_secs(1);
    assert!(lease.expired());
}

#[tokio::test]
async fn revoked_session_cancels_its_download() {
    let temp = tempfile::tempdir().unwrap();
    let state = test_state(temp.path()).unwrap();
    let session = state
        .create_session(IpAddr::V4(Ipv4Addr::new(192, 168, 10, 50)), "Test".into())
        .await
        .unwrap();
    let mut lease = state
        .begin_download(&session.id, session.address, "datei", 10)
        .await
        .unwrap();
    assert!(state.revoke_session(&session.id).await);
    lease.cancel.changed().await.unwrap();
    assert!(*lease.cancel.borrow());
}

#[tokio::test]
async fn expires_inactive_upload_and_removes_only_its_partial() {
    let temp = tempfile::tempdir().unwrap();
    let state = test_state(temp.path()).unwrap();
    let id = Uuid::new_v4().to_string();
    let partial_path = state
        .partial_dir
        .as_ref()
        .unwrap()
        .join(format!("{id}.part"));
    let mut partial_file = create_upload_partial(&partial_path).unwrap();
    partial_file.write_all(b"teil").unwrap();
    let transfer_id = state
        .record_transfer(TransferDirection::Upload, "datei.txt", 100)
        .await;
    state.uploads.lock().await.insert(
        id.clone(),
        Arc::new(Mutex::new(UploadRecord {
            id: id.clone(),
            owner_session: "session".into(),
            owner_address: IpAddr::V4(Ipv4Addr::new(192, 168, 10, 50)),
            name: "datei.txt".into(),
            declared_size: 100,
            offset: 4,
            last_modified: 1,
            client_token: "expired-token".into(),
            created_at: Instant::now(),
            last_activity: Instant::now() - UPLOAD_IDLE_TIMEOUT - Duration::from_secs(1),
            cancelled: false,
            finalizing: false,
            cancel_signal: Arc::new(AtomicBool::new(false)),
            chunk_slots: Arc::new(Semaphore::new(1)),
            partial_path: partial_path.clone(),
            partial_file: Arc::new(partial_file),
            transfer_id: transfer_id.clone(),
        })),
    );

    state.expire_stale_uploads().await;
    tokio::time::timeout(Duration::from_secs(1), async {
        while partial_path.exists() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("expired upload cleanup must remove its partial");
    assert!(!state.uploads.lock().await.contains_key(&id));
    assert_eq!(
        state
            .transfers
            .lock()
            .await
            .iter()
            .find(|item| item.id == transfer_id)
            .unwrap()
            .state,
        TransferState::Expired
    );
}

#[tokio::test]
async fn expires_upload_after_absolute_lifetime_despite_recent_progress() {
    let temp = tempfile::tempdir().unwrap();
    let state = test_state(temp.path()).unwrap();
    let id = Uuid::new_v4().to_string();
    let partial_path = state
        .partial_dir
        .as_ref()
        .unwrap()
        .join(format!("{id}.part"));
    let mut partial_file = create_upload_partial(&partial_path).unwrap();
    partial_file.write_all(b"x").unwrap();
    let transfer_id = state
        .record_transfer(TransferDirection::Upload, "langsam.txt", 100)
        .await;
    state.uploads.lock().await.insert(
        id.clone(),
        Arc::new(Mutex::new(UploadRecord {
            id: id.clone(),
            owner_session: "session".into(),
            owner_address: IpAddr::V4(Ipv4Addr::new(192, 168, 10, 50)),
            name: "langsam.txt".into(),
            declared_size: 100,
            offset: 1,
            last_modified: 1,
            client_token: "lifetime-token".into(),
            created_at: Instant::now() - UPLOAD_MAX_LIFETIME - Duration::from_secs(1),
            last_activity: Instant::now(),
            cancelled: false,
            finalizing: false,
            cancel_signal: Arc::new(AtomicBool::new(false)),
            chunk_slots: Arc::new(Semaphore::new(1)),
            partial_path: partial_path.clone(),
            partial_file: Arc::new(partial_file),
            transfer_id,
        })),
    );

    state.expire_stale_uploads().await;
    tokio::time::timeout(Duration::from_secs(1), async {
        while partial_path.exists() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("absolute upload expiry must remove its partial");
    assert!(!state.uploads.lock().await.contains_key(&id));
}
