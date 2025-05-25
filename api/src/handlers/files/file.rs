use axum::extract::{Multipart, State};
use axum::http::HeaderMap;
use nanoid::nanoid;
use std::ffi::OsStr;
use std::path::Path;

use abi::errors::Error;

use crate::AppState;
use crate::api_utils::custom_extract::PathExtractor;

pub async fn upload(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<String, Error> {
    let mut filename = String::new();
    if let Some(field) = multipart.next_field().await.map_err(Error::internal)? {
        filename = field.file_name().unwrap_or_default().to_string();
        let extension = Path::new(&filename).extension().and_then(OsStr::to_str);
        // filename = extension.map(|v| format!("{}.{}", nanoid!(), v)).unwrap_or(nanoid!());
        filename = match extension {
            None => nanoid!(),
            Some(e) => format!("{}.{}", nanoid!(), e),
        };

        let data = field.bytes().await.map_err(Error::internal)?;
        state.oss.upload_file(&filename, data.into()).await?;
    }

    Ok(filename)
}

pub async fn get_file_by_name(
    State(state): State<AppState>,
    PathExtractor(filename): PathExtractor<String>,
) -> Result<(HeaderMap, Vec<u8>), Error> {
    let bytes = state.oss.download_file(&filename).await?;
    let mut headers = HeaderMap::with_capacity(1);
    headers.insert(
        "Cache-Control",
        "private, max-age=31536000".parse().unwrap(),
    );
    Ok((headers, bytes.into()))
}

pub async fn get_avatar_by_name(
    State(state): State<AppState>,
    PathExtractor(filename): PathExtractor<String>,
) -> Result<(HeaderMap, Vec<u8>), Error> {
    let bytes = state.oss.download_avatar(&filename).await?;
    let mut headers = HeaderMap::with_capacity(1);
    headers.insert(
        "Cache-Control",
        "private, max-age=31536000".parse().unwrap(),
    );
    Ok((headers, bytes.into()))
}

pub async fn upload_avatar(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<String, Error> {
    let mut filename = String::new();
    if let Some(field) = multipart.next_field().await.map_err(Error::internal)? {
        filename = field.file_name().unwrap_or_default().to_string();
        let extension = Path::new(&filename).extension().and_then(OsStr::to_str);
        filename = match extension {
            None => nanoid!(),
            Some(e) => format!("{}.{}", nanoid!(), e),
        };

        let data = field.bytes().await.map_err(Error::internal)?;
        state.oss.upload_avatar(&filename, data.into()).await?;
    }

    Ok(filename)
}
