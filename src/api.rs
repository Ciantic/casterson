use bytes::BytesMut;

use futures_util::TryFutureExt;
use futures_util::TryStreamExt;
use hyper::header::HeaderValue;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use hyper::{Method, StatusCode};
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

async fn handle_request(
    state: &AppState,
    req: Request<Body>,
) -> Result<Response<Body>, Infallible> {
    if req.uri().path().starts_with("/chromecast") {
        handle_chromecast_request(req).await
    } else {
        handle_other_request(state, req).await
    }
}

async fn handle_chromecast_request(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let receiver = chromecast::get_default_media_receiver("192.168.8.106");
    let mut response = Response::new(Body::empty());
    match (
        req.method(),
        req.uri().path().trim_start_matches("/chromecast"),
    ) {
        (&Method::GET, "/start") => {
            tokio::spawn(async move {
                receiver.cast("http://192.168.8.103:3000/file/encode");
            });
            *response.status_mut() = StatusCode::OK;
        }

        (&Method::GET, "/pause") => {
            receiver.pause();
            *response.status_mut() = StatusCode::OK;
        }

        (&Method::GET, "/play") => {
            receiver.play();
            *response.status_mut() = StatusCode::OK;
        }

        (&Method::GET, "/stop") => {
            receiver.stop();
            *response.status_mut() = StatusCode::OK;
        }

        (&Method::GET, "/status") => {
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
) -> Result<Response<Body>, Infallible> {
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
