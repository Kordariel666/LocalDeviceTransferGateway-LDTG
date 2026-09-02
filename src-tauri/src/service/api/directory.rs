use super::*;

#[derive(Deserialize)]
pub(super) struct ListQuery {
    #[serde(default)]
    path: String,
    cursor: Option<String>,
    page: Option<u64>,
    q: Option<String>,
}

pub(super) fn relative_url_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .components()
        .filter_map(|part| match part {
            std::path::Component::Normal(value) => Some(value.to_string_lossy().into_owned()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/")
}

pub(super) fn natural_compare(left: &str, right: &str) -> Ordering {
    let left = left.to_lowercase();
    let right = right.to_lowercase();
    let left = left.as_bytes();
    let right = right.as_bytes();
    let (mut left_index, mut right_index) = (0, 0);
    while left_index < left.len() && right_index < right.len() {
        if left[left_index].is_ascii_digit() && right[right_index].is_ascii_digit() {
            let left_end = (left_index..left.len())
                .find(|index| !left[*index].is_ascii_digit())
                .unwrap_or(left.len());
            let right_end = (right_index..right.len())
                .find(|index| !right[*index].is_ascii_digit())
                .unwrap_or(right.len());
            let left_significant = (left_index..left_end)
                .find(|index| left[*index] != b'0')
                .unwrap_or(left_end.saturating_sub(1));
            let right_significant = (right_index..right_end)
                .find(|index| right[*index] != b'0')
                .unwrap_or(right_end.saturating_sub(1));
            let length_order = (left_end - left_significant).cmp(&(right_end - right_significant));
            if length_order != Ordering::Equal {
                return length_order;
            }
            let number_order =
                left[left_significant..left_end].cmp(&right[right_significant..right_end]);
            if number_order != Ordering::Equal {
                return number_order;
            }
            let zero_order = (left_end - left_index).cmp(&(right_end - right_index));
            if zero_order != Ordering::Equal {
                return zero_order;
            }
            left_index = left_end;
            right_index = right_end;
        } else {
            let order = left[left_index].cmp(&right[right_index]);
            if order != Ordering::Equal {
                return order;
            }
            left_index += 1;
            right_index += 1;
        }
    }
    left.len().cmp(&right.len())
}

pub(super) fn read_directory_page(
    listing: &DirectoryListing,
    requested_page: u64,
) -> ApiResult<DirectoryPage> {
    let mut cursor = listing.cursor.lock().map_err(|_| {
        ApiFailure::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DIRECTORY_CURSOR_FAILED",
            "Der Ordnercursor ist nicht mehr verfügbar.",
        )
    })?;
    if cursor.last_activity.elapsed() >= crate::service::state::DIRECTORY_CURSOR_TTL {
        return Err(ApiFailure::new(
            StatusCode::BAD_REQUEST,
            "CURSOR_INVALID",
            "Der Ordnercursor ist abgelaufen.",
        ));
    }
    if let Some(page) = cursor
        .last_page
        .as_ref()
        .filter(|page| page.sequence == requested_page)
        .cloned()
    {
        cursor.last_activity = Instant::now();
        return Ok(page);
    }
    if requested_page != cursor.next_page || cursor.exhausted {
        return Err(ApiFailure::new(
            StatusCode::BAD_REQUEST,
            "CURSOR_PAGE_INVALID",
            "Die angeforderte Ordnerseite passt nicht zum Cursor.",
        ));
    }
    let mut entries = Vec::new();
    for _ in 0..DIRECTORY_SCAN_LIMIT {
        let Some(item) = cursor.iterator.next() else {
            cursor.exhausted = true;
            break;
        };
        let Ok(item) = item else {
            continue;
        };
        let requested_path = Path::new(&listing.path).join(item.file_name());
        let relative_path = relative_url_path(&listing.root, &listing.root.join(requested_path));
        let Ok(checked_path) = safe_existing(&listing.root, &relative_path, None) else {
            continue;
        };
        let Ok((resolved, _file, metadata)) = listing.root_anchor.open_existing(&checked_path)
        else {
            continue;
        };
        if is_hidden_or_managed(&resolved, &metadata) {
            continue;
        }
        let relative_path = relative_url_path(&listing.root, &resolved);
        let name = resolved
            .file_name()
            .map(|value| value.to_string_lossy().into_owned())
            .unwrap_or_else(|| item.file_name().to_string_lossy().into_owned());
        if !listing.filter.is_empty() && !name.to_lowercase().contains(&listing.filter) {
            continue;
        }
        let modified_at = metadata
            .modified()
            .ok()
            .map(|value| DateTime::<Utc>::from(value).to_rfc3339());
        entries.push(DirectoryEntry {
            name,
            path: relative_path,
            kind: if metadata.is_dir() {
                DirectoryEntryKind::Directory
            } else {
                DirectoryEntryKind::File
            },
            size: if metadata.is_file() {
                metadata.len()
            } else {
                0
            },
            modified_at,
        });
        if entries.len() >= DIRECTORY_PAGE_SIZE {
            break;
        }
    }
    entries.sort_by(|left, right| {
        (left.kind != DirectoryEntryKind::Directory)
            .cmp(&(right.kind != DirectoryEntryKind::Directory))
            .then_with(|| natural_compare(&left.name, &right.name))
            .then_with(|| left.name.cmp(&right.name))
    });
    cursor.last_activity = Instant::now();
    let page = DirectoryPage {
        sequence: requested_page,
        entries,
        exhausted: cursor.exhausted,
    };
    cursor.next_page = cursor.next_page.saturating_add(1);
    cursor.last_page = Some(page.clone());
    Ok(page)
}

pub(super) async fn list_downloads(
    State(state): State<Arc<TransferServiceState>>,
    ConnectInfo(client): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    query: Result<Query<ListQuery>, QueryRejection>,
) -> ApiResult<Json<DirectoryResponse>> {
    let Query(query) = query.map_err(|error| {
        ApiFailure::new(
            error.status(),
            "QUERY_INVALID",
            "Die Ordneranfrage ist ungültig.",
        )
    })?;
    let session = authorized(&state, &headers, client.ip(), false).await?;
    let root = state.roots.download.as_ref().cloned().ok_or_else(|| {
        ApiFailure::new(
            StatusCode::NOT_FOUND,
            "DOWNLOAD_DISABLED",
            "Die Downloadfreigabe ist nicht aktiv.",
        )
    })?;
    let filter = query.q.unwrap_or_default().to_lowercase();
    let requested_page = match (&query.cursor, query.page) {
        (Some(_), Some(page)) => page,
        (Some(_), None) => {
            return Err(ApiFailure::new(
                StatusCode::BAD_REQUEST,
                "CURSOR_PAGE_REQUIRED",
                "Für einen Ordnercursor muss die Seitennummer mitgesendet werden.",
            ))
        }
        (None, Some(page)) if page != 0 => {
            return Err(ApiFailure::new(
                StatusCode::BAD_REQUEST,
                "CURSOR_PAGE_INVALID",
                "Eine neue Ordnerabfrage beginnt mit Seite null.",
            ))
        }
        _ => 0,
    };
    let (cursor_id, listing, created) = if let Some(cursor_id) = query.cursor {
        let listing = state
            .directory_listing(&cursor_id, &session.id, &query.path, &filter)
            .await
            .ok_or_else(|| {
                ApiFailure::new(
                    StatusCode::BAD_REQUEST,
                    "CURSOR_INVALID",
                    "Der Ordnercursor ist abgelaufen oder gehört zu einer anderen Anfrage.",
                )
            })?;
        (cursor_id, listing, false)
    } else {
        let open_permit = state
            .begin_listing(&session.id, session.address)
            .await
            .ok_or_else(|| {
                ApiFailure::new(
                    StatusCode::TOO_MANY_REQUESTS,
                    "DIRECTORY_LIMIT",
                    "Es werden bereits zu viele Ordnerseiten vorbereitet.",
                )
            })?;
        let open_root = root.clone();
        let open_path = query.path.clone();
        let iterator = tokio::task::spawn_blocking(move || {
            let _permit = open_permit;
            let folder = safe_existing(&open_root, &open_path, Some(true)).map_err(|_| {
                ApiFailure::new(
                    StatusCode::NOT_FOUND,
                    "PATH_UNAVAILABLE",
                    "Ordner ist nicht verfügbar.",
                )
            })?;
            fs::read_dir(folder).map_err(|_| {
                ApiFailure::new(
                    StatusCode::NOT_FOUND,
                    "PATH_UNAVAILABLE",
                    "Ordner ist nicht verfügbar.",
                )
            })
        })
        .await
        .map_err(|_| {
            ApiFailure::new(
                StatusCode::INTERNAL_SERVER_ERROR,
                "DIRECTORY_TASK_FAILED",
                "Der Ordner konnte nicht sicher vorbereitet werden.",
            )
        })??;
        let (cursor_id, listing) = state
            .create_directory_listing(&session, query.path.clone(), filter.clone(), root, iterator)
            .await
            .map_err(|scope| {
                if scope == "invalid" {
                    return ApiFailure::new(
                        StatusCode::UNAUTHORIZED,
                        "SESSION_INVALID",
                        "Die Sitzung ist nicht mehr gültig.",
                    );
                }
                let message = match scope {
                    "session" => "Diese Sitzung hat bereits zu viele offene Ordnercursor.",
                    "address" => {
                        "Von dieser Geräteadresse bestehen bereits zu viele offene Ordnercursor."
                    }
                    _ => "Es bestehen bereits zu viele offene Ordnercursor.",
                };
                ApiFailure::new(
                    StatusCode::TOO_MANY_REQUESTS,
                    "DIRECTORY_CURSOR_LIMIT",
                    message,
                )
            })?;
        (cursor_id, listing, true)
    };
    let page_permit = match state.begin_listing(&session.id, session.address).await {
        Some(permit) => permit,
        None => {
            if created {
                state.remove_directory_listing(&cursor_id).await;
            }
            return Err(ApiFailure::new(
                StatusCode::TOO_MANY_REQUESTS,
                "DIRECTORY_LIMIT",
                "Es werden bereits zu viele Ordnerseiten vorbereitet.",
            ));
        }
    };
    let page_listing = listing.clone();
    let page = tokio::task::spawn_blocking(move || {
        let _permit = page_permit;
        read_directory_page(&page_listing, requested_page)
    })
    .await
    .map_err(|_| {
        ApiFailure::new(
            StatusCode::INTERNAL_SERVER_ERROR,
            "DIRECTORY_TASK_FAILED",
            "Die Ordnerseite konnte nicht sicher gelesen werden.",
        )
    })??;
    ensure_session_active(&state, &session).await?;
    let next_cursor = (!page.exhausted).then_some(cursor_id);
    let next_page = (!page.exhausted).then_some(page.sequence.saturating_add(1));
    Ok(Json(DirectoryResponse {
        path: query.path,
        query: filter,
        entries: page.entries,
        next_cursor,
        next_page,
    }))
}
