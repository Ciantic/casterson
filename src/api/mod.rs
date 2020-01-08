use crate::chromecast as chromecast_main;
use crate::AppState;
use derive_more::From;
use hyper::service::{make_service_fn, service_fn};
use hyper::Method;
use hyper::{Body, Request, Response, Server};
use percent_encoding::percent_decode_str;
use serde::Serialize;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;
pub mod chromecast;
pub mod ui;

#[derive(Debug, From)]
pub enum ApiError {
    NotFound,
    InvalidMediaFile(String),
    ChromecastError(chromecast_main::ChromecastError),
    JsonError(serde_json::error::Error),
    IoError(std::io::Error),
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

            ApiError::InvalidMediaFile(file) => ApiJsonError {
                error: "INVALID_MEDIA_FILE".into(),
                msg: file,
            },

            ApiError::NotFound => ApiJsonError {
                error: "NOT_FOUND".into(),
                msg: "404 Not found".into(),
            },

            _ => ApiJsonError {
                error: "UNKNOWN".into(),
                msg: "".into(),
            },
        }
    }
}
pub type ApiResponse<S> = Result<S, ApiError>;

fn to_response<T>(resp: ApiResponse<T>) -> Result<Response<Body>, ApiError>
where
    T: Serialize,
{
    resp.map(|v| {
        let json = serde_json::to_string(&v).unwrap();
        Response::new(Body::from(json))
    })
}

/// Create hyper server
pub async fn start_server(state: Arc<AppState>) -> Result<(), hyper::error::Error> {
    println!(
        "Trying to start server at: {}:{} ...",
        state.opts.ip, state.opts.port
    );
    let addr = SocketAddr::from((state.opts.ip, state.opts.port));

    // Creates a service creator "MakeSvc"
    let make_svc = make_service_fn(move |_| {
        let state_con = Arc::clone(&state);
        async move {
            // Creates a "Service" from asyncfunction
            Ok::<_, Infallible>(service_fn(move |req: Request<Body>| {
                let state_req = Arc::clone(&state_con);
                async move { handle_request(state_req, req).await }
            }))
        }
    });
    let builder = Server::try_bind(&addr)?;
    println!("Server is now listening.");
    builder.serve(make_svc).await
}

async fn handle_request(
    state: Arc<AppState>,
    req: Request<Body>,
) -> Result<Response<Body>, Infallible> {
    let resp = {
        if req.uri().path().starts_with("/chromecast") {
            handle_chromecast_request(state, req).await
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

async fn handle_chromecast_request(
    state: Arc<AppState>,
    req: Request<Body>,
) -> Result<Response<Body>, ApiError> {
    if req.method() != Method::POST {
        return Err(ApiError::NotFound);
    }
    let uri = req.uri().clone();
    let body = hyper::body::to_bytes(req.into_body()).await.unwrap();
    let request: chromecast::ChromecastRequest = serde_json::from_slice(&body)?;
    let api = chromecast::ChromecastApi { state, request };

    match uri.path() {
        "/chromecast/cast" => to_response(api.cast(serde_json::from_slice(&body)?).await),
        "/chromecast/pause" => to_response(api.pause().await),
        "/chromecast/play" => to_response(api.play().await),
        "/chromecast/stop" => to_response(api.stop().await),
        "/chromecast/status" => to_response(api.status().await),
        _ => Err(ApiError::NotFound),
    }
}

async fn handle_other_request(
    state: Arc<AppState>,
    request: Request<Body>,
) -> Result<Response<Body>, ApiError> {
    // This is dead simple, but unorthodox. Query string is interpreted as JSON string e.g.
    // http://localhost/something?{"test":5}
    let query = percent_decode_str(request.uri().query().unwrap_or(""))
        .decode_utf8_lossy()
        .into_owned();

    match (request.method(), request.uri().path()) {
        (&Method::GET, "/get_media_files") => to_response(ui::get_media_files(state).await),
        (&Method::GET, "/media_show") => {
            // Chrome is spamming with multiple requests on HTTP hosts, it causes ffmpeg to freak
            // out. This may have something to do that first request has
            // "Update-Insecure-Requests=1". This is mostly a testing problem but it seems to help if
            // I wait a little bit between requests.
            if let Some(v) = request.headers().get(hyper::header::USER_AGENT) {
                if let Ok(vv) = v.to_str() {
                    if vv.find("Chrome/").is_some() {
                        tokio::time::delay_for(tokio::time::Duration::from_millis(500)).await;
                    }
                }
            }

            ui::media_show(state, serde_json::from_str(&query)?).await
        }
        _ => Err(ApiError::NotFound),
    }
}
