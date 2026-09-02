use super::state::{
    AuthDecision, CompletedUpload, DirectoryListing, DirectoryPage, DownloadLease,
    SessionCreateError, SessionRecord, TransferServiceState, UploadChunkLease, UploadIoPermit,
    UploadRecord, ACCESS_CODE_DIGITS, CHUNK_SIZE, DISK_RESERVE, MAX_DEVICE_NAME_CHARS,
    MAX_UPLOADS_PER_ADDRESS,
};
use super::ConnectionSecurity;
use crate::domain::{
    network::same_subnet,
    shares::{
        create_upload_partial, delete_open_upload, is_hidden_or_managed, publish_open_upload,
        safe_existing, safe_file_name_for_root,
    },
    types::{
        CompleteResponse, DirectoryEntry, DirectoryEntryKind, DirectoryResponse, ErrorBody,
        SessionResponse, TransferDirection, TransferState, UploadResponse,
    },
};
use axum::{
    body::{Body, Bytes},
    extract::{
        rejection::{BytesRejection, JsonRejection, PathRejection, QueryRejection},
        ConnectInfo, DefaultBodyLimit, Path as AxumPath, Query, State,
    },
    http::{header, HeaderMap, HeaderValue, Method, Request, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Extension, Json, Router,
};
use chrono::{DateTime, Utc};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use rust_embed::RustEmbed;
use serde::Deserialize;
use std::{
    cmp::Ordering,
    fs,
    net::{IpAddr, SocketAddr},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering as AtomicOrdering},
        Arc,
    },
    time::{Duration, Instant},
};
use subtle::ConstantTimeEq;
use tokio::io::{AsyncReadExt, AsyncSeekExt};
use tokio::sync::Mutex;
use uuid::Uuid;

mod assets;
mod auth;
mod common;
mod directory;
mod download;
mod upload;

use assets::*;
use auth::*;
use common::*;
use directory::*;
use download::*;
use upload::*;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(2 * 60);
const DIRECTORY_PAGE_SIZE: usize = 200;
const DIRECTORY_SCAN_LIMIT: usize = 256;

pub fn router(state: Arc<TransferServiceState>) -> Router {
    Router::new()
        .route(
            "/api/v1/auth",
            post(auth).layer(DefaultBodyLimit::max(1024)),
        )
        .route("/api/v1/session", get(session))
        .route("/api/v1/logout", post(logout))
        .route("/api/v1/downloads", get(list_downloads))
        .route("/api/v1/download", get(download).head(download_head))
        .route(
            "/api/v1/uploads",
            post(create_upload).layer(DefaultBodyLimit::max(32 * 1024)),
        )
        .route(
            "/api/v1/uploads/{id}",
            get(upload_status).patch(upload_chunk).delete(cancel_upload),
        )
        .route("/api/v1/uploads/{id}/complete", post(complete_upload))
        .method_not_allowed_fallback(method_not_allowed)
        .fallback(static_asset)
        .layer(DefaultBodyLimit::max(CHUNK_SIZE + 1024))
        .layer(middleware::from_fn_with_state(state.clone(), request_guard))
        .with_state(state)
}

#[cfg(test)]
mod tests;
