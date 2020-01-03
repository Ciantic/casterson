use hyper::service::{make_service_fn, service_fn};
use hyper::Method;
use hyper::{Body, Request, Response, Server};
use serde::Serialize;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::sync::Arc;

pub mod chromecast;
pub mod ui;

use crate::chromecast as chromecast_main;
use crate::AppState;

#[derive(Debug)]
pub enum ApiError {
    NotFound,
    ChromecastError(chromecast_main::ChromecastError),
    JsonError(serde_json::error::Error),
    // HyperError(hyper::error::Error),
}

impl From<chromecast_main::ChromecastError> for ApiError {
    fn from(w: chromecast_main::ChromecastError) -> ApiError {
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

// pub struct ApiResponse<T>(Result<T, ApiError>)
// where
//     T: Serialize;

// impl<T> From<Result<Response<Body>, ApiError>> for ApiResponse<T>
// where
//     T: Serialize,
// {
//     fn from(err: Result<Response<Body>, ApiError>) -> Self {
//         unimplemented!()
//     }
// }

pub type ApiResponse<S> = Result<S, ApiError>;

fn to_response<T>(resp: ApiResponse<T>) -> Result<Response<Body>, ApiError>
where
    T: Serialize,
{
    resp.map(|v| {
        let json = serde_json::to_string(&v).unwrap();
        Response::new(Body::from(json))
    })
    // .map_err(|e| {
    //     let json_err: ApiJsonError = e.into();
    //     let json = serde_json::to_string(&json_err).unwrap();
    //     Response::new(Body::from(json))
    // })
    // .unwrap_or_else(|e| e)
}

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
                async move { handle_request(state_req, req).await }
            }))
        }
    });
    let server = Server::bind(&addr).serve(make_svc);
    if let Err(e) = server.await {
        eprintln!("Server error: {}", e);
    }
    println!("Server closed");
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
        "/chromecast/cast" => to_response(api.cast().await),
        "/chromecast/pause" => to_response(api.pause().await),
        "/chromecast/play" => to_response(api.play().await),
        "/chromecast/stop" => to_response(api.stop().await),
        "/chromecast/status" => to_response(api.status().await),
        _ => Err(ApiError::NotFound),
    }
}

async fn handle_other_request(
    state: Arc<AppState>,
    req: Request<Body>,
) -> Result<Response<Body>, ApiError> {
    match (req.method(), req.uri().path()) {
        (&Method::GET, "/get_media_files") => to_response(ui::get_media_files(state).await),
        (&Method::GET, "/media_show") => ui::media_show(state).await,
        _ => Err(ApiError::NotFound),
    }
}
