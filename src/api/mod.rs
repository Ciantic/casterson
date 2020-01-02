use bytes::BytesMut;
use serde::Serializer;
use std::net::IpAddr;

use futures_util::TryFutureExt;
use futures_util::TryStreamExt;
use hyper::header::HeaderValue;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use hyper::{Method, StatusCode};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::fs::File;
use tokio_util::codec::{BytesCodec, FramedRead};

use crate::chromecast;
use crate::chromecast::BaseMediaReceiver;
use crate::media;
use crate::msg;
use crate::AppState;

#[derive(Debug)]
enum ApiError {
    NotFound,
    ChromecastError(chromecast::ChromecastError),
    JsonError(serde_json::error::Error),
    // HyperError(hyper::error::Error),
}

impl From<chromecast::ChromecastError> for ApiError {
    fn from(w: chromecast::ChromecastError) -> ApiError {
        ApiError::ChromecastError(w)
    }
}

impl From<serde_json::error::Error> for ApiError {
    fn from(w: serde_json::error::Error) -> ApiError {
        ApiError::JsonError(w)
    }
}

#[derive(Serialize)]
struct ApiJsonError {
    error: String,
    msg: String,
    // Ideally there would be union of actual data payload, but unions aren't
    // ready yet in Rust
    // data: ...
}

// Convert ApiError to response
impl Into<ApiJsonError> for ApiError {
    fn into(self) -> ApiJsonError {
        match self {
            ApiError::JsonError(err) => ApiJsonError {
                error: "JSON_ERROR".into(),
                msg: err.to_string(),
            },

            _ => ApiJsonError {
                error: "UNKNOWN".into(),
                msg: "".into(),
            },
        }
    }
}

type ApiResponse<S>
where
    S: Serializer,
= Result<S, ApiError>;

/// Create hyper server
pub async fn create_server(state: Arc<AppState>) {
    println!("Server listening at: {}:{}", state.opts.ip, state.opts.port);
    let addr = SocketAddr::from((state.opts.ip, state.opts.port));

    // Creates a service creator "MakeSvc"
    let make_svc = make_service_fn(move |_| {
        let state_con = Arc::clone(&state);
        async move {
            // Creates a "Service" from asyncfunction
            Ok::<_, Infallible>(service_fn(move |req: Request<Body>| {
                let state_req = Arc::clone(&state_con);
                async move { handle_request(&*state_req, req).await }
            }))
        }
    });
    let server = Server::bind(&addr).serve(make_svc);
    if let Err(e) = server.await {
        eprintln!("Server error: {}", e);
    }
    println!("Server closed");
}

async fn handle_request2(state: &AppState) -> ApiResponse<()> {
    unimplemented!()
}

async fn handle_request(
    state: &AppState,
    req: Request<Body>,
) -> Result<Response<Body>, Infallible> {
    let resp = {
        if req.uri().path().starts_with("/chromecast") {
            handle_chromecast_request(req).await
        } else {
            handle_other_request(state, req).await
        }
    };
    match resp {
        Ok(v) => Ok(v),
        Err(err) => Ok(Response::new(Body::from({
            let v: ApiJsonError = err.into();
            serde_json::to_string_pretty(&v).unwrap()
        }))),
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct ChromecastRequest {
    ip: IpAddr,
    port: Option<u16>,
    dest_id: Option<String>,
}

async fn handle_chromecast_request(req: Request<Body>) -> Result<Response<Body>, ApiError> {
    let method = req.method().clone();
    let uri = req.uri().clone();
    let body = hyper::body::to_bytes(req.into_body()).await.unwrap();
    let value = serde_json::from_slice::<ChromecastRequest>(&body)?;

    println!("value {:?}", value);

    let receiver = chromecast::get_default_media_receiver(&value.ip, value.port, value.dest_id);
    let mut response = Response::new(Body::empty());

    match (method, uri.path().trim_start_matches("/chromecast")) {
        (Method::POST, "/start") => {
            tokio::spawn(async move {
                // TODO: Handle panics! Send through the channel in AppState
                receiver
                    .cast("http://192.168.8.103:3000/file/encode")
                    .unwrap();
            });
            *response.status_mut() = StatusCode::OK;
        }

        (Method::POST, "/pause") => {
            receiver.pause()?;
            *response.status_mut() = StatusCode::OK;
        }

        (Method::POST, "/play") => {
            receiver.play()?;
            *response.status_mut() = StatusCode::OK;
        }

        (Method::POST, "/stop") => {
            receiver.stop()?;
            *response.status_mut() = StatusCode::OK;
        }

        (Method::POST, "/status") => {
            let status = receiver.get_status().unwrap();
            let json = serde_json::to_string(&status).unwrap();
            response = Response::new(Body::from(json));
            *response.status_mut() = StatusCode::OK;
        }

        // 404 not found
        _ => {
            *response.status_mut() = StatusCode::NOT_FOUND;
        }
    };
    Ok(response)
}

async fn handle_other_request(
    state: &AppState,
    req: Request<Body>,
) -> Result<Response<Body>, ApiError> {
    let mut response = Response::new(Body::empty());

    // let query = req.uri().query();
    // let parsedUri = Url::parse(&req.uri().to_string()).unwrap();
    // let params: HashMap<_, _> = parsedUri.query_pairs().into_owned().collect();

    match (req.method(), req.uri().path()) {
        (&Method::GET, "/media_files") => {
            let media_files = media::scan_media_files(&state.opts.dir, &state.opts.media_exts);
        }

        // Stream a file from a disk
        (&Method::GET, "/file") => {
            // let file =
            let stream = File::open("C:\\Source\\Backup_Ignore.txt")
                .map_ok(|file| FramedRead::new(file, BytesCodec::new()).map_ok(BytesMut::freeze))
                .try_flatten_stream();
            response = Response::new(Body::wrap_stream(stream));
        }

        (&Method::GET, "/file/encode") => {
            state
                .notifier
                .send(msg::NotifyMessage::EncodingStarted)
                .unwrap();
            let stdout =
                media::encode("\\\\192.168.8.150\\Downloads\\Big.Buck.Bunny\\big_buck_bunny.mp4");
            let st = FramedRead::new(stdout, BytesCodec::new()).map_ok(BytesMut::freeze);
            let s = Body::wrap_stream(st);
            response = Response::new(s);
            response
                .headers_mut()
                .insert("Content-Type", HeaderValue::from_static("video/mp4"));
            response
                .headers_mut()
                .insert("Cache-Control", HeaderValue::from_static("no-cache"));
        }

        // 404 not found
        _ => {
            *response.status_mut() = StatusCode::NOT_FOUND;
        }
    }
    Ok(response)
}
