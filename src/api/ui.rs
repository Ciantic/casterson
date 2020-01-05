use crate::AppState;
use hyper::Body;
use hyper::Response;
use std::sync::Arc;

use crate::api::ApiResponse;
use futures::stream::StreamExt;
use hyper::header::HeaderValue;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::api::ApiError;
use crate::media;
use crate::msg;
use std::convert::Infallible;

#[derive(Serialize)]
pub struct MediaFilesResult {
    files: Vec<String>,
}

pub async fn get_media_files(state: Arc<AppState>) -> ApiResponse<MediaFilesResult> {
    let files: Vec<String> = media::scan_media_files(&state.opts.dir, &state.opts.media_exts)
        .iter()
        .filter_map(|v: &PathBuf| v.to_str())
        .map(|v| v.to_string())
        .collect();
    Ok(MediaFilesResult { files })
}

#[derive(Deserialize)]
pub struct MediaShowRequest {
    pub file: String,
    pub seek_seconds: i32,
    pub use_subtitles: bool,
}

pub async fn media_show(
    state: Arc<AppState>,
    request: MediaShowRequest,
) -> ApiResponse<Response<Body>> {
    let file = request.file;
    let mut opts = media::EncodeVideoOpts::default();
    opts.use_subtitles = request.use_subtitles;
    opts.seek_seconds = request.seek_seconds;
    opts.tv_resolution = (1280, 720);
    opts.crop_percent = 12;
    // println!(
    //     "Validate file {} {:?} {:?} {:?}",
    //     file,
    //     &state.opts.dir,
    //     &state.opts.media_exts,
    //     std::fs::canonicalize(&file).map(|v| v.to_string_lossy().into_owned())
    // );
    if !media::is_safe_file(&file, &state.opts.dir, &state.opts.media_exts) {
        return Err(ApiError::InvalidMediaFile(file));
    }
    state
        .notifier
        .send(msg::NotifyMessage::EncodingStarted)
        .unwrap();
    let stream = media::encode(file, opts).await;
    // let mut response = Response::new(Body::from(stream));
    let mut response = Response::new(Body::wrap_stream(stream.map(|e| Ok::<_, Infallible>(e))));

    // Headers
    response
        .headers_mut()
        .insert("Content-Type", HeaderValue::from_static("video/mp4"));
    response
        .headers_mut()
        .insert("Cache-Control", HeaderValue::from_static("no-cache"));
    Ok(response)
}
