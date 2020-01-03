use crate::AppState;
use hyper::Body;
use hyper::Response;
use std::sync::Arc;

use crate::api::ApiResponse;
use bytes::BytesMut;
use futures::Stream;
use futures_util::TryStreamExt;
use hyper::header::HeaderValue;
use tokio_util::codec::{BytesCodec, FramedRead};

use crate::media;
use crate::msg;

pub async fn get_media_files(state: Arc<AppState>) -> ApiResponse<()> {
    media::scan_media_files(&state.opts.dir, &state.opts.media_exts);
    Ok(())
}

pub async fn media_show(state: Arc<AppState>) -> ApiResponse<Response<Body>> {
    state
        .notifier
        .send(msg::NotifyMessage::EncodingStarted)
        .unwrap();
    let stream = media::encode("\\\\192.168.8.150\\Downloads\\Big.Buck.Bunny\\big_buck_bunny.mp4");
    let mut response = Response::new(Body::wrap_stream(stream));
    response
        .headers_mut()
        .insert("Content-Type", HeaderValue::from_static("video/mp4"));
    response
        .headers_mut()
        .insert("Cache-Control", HeaderValue::from_static("no-cache"));
    Ok(response)
}
