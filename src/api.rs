use bytes::BytesMut;
use clap::{App, Arg};
use crossbeam::channel::{Receiver, Sender};
use crossbeam::unbounded;
use futures_util::TryFutureExt;
use futures_util::TryStreamExt;
use hyper::header::HeaderValue;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use hyper::{Method, StatusCode};
use std::convert::Infallible;
use std::net::IpAddr;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use structopt::StructOpt;
use tokio::fs::File;
use tokio::process::Command;
use tokio_util::codec::{BytesCodec, FramedRead};

#[path = "msg.rs"]
pub mod msg;

#[path = "chromecast.rs"]
mod chromecast;
use chromecast::BaseMediaReceiver;

pub struct RequestState {
    pub foo: u32,
    pub zoo: Vec<u32>,
    pub notifier: Sender<msg::NotifyMessage>,
}

pub async fn create_server(notify: Sender<msg::NotifyMessage>, ip: IpAddr, port: u16) {
    println!("Starting server at {}:{}", ip, port);
    let state = Arc::new(RequestState {
        foo: 321,
        zoo: vec![1, 2, 3],
        notifier: notify.clone(),
    });
    let addr = SocketAddr::from((ip, port));
    let (notify, rec) = unbounded::<msg::NotifyMessage>();
    let make_svc = make_service_fn(move |_| {
        let state_con = Arc::clone(&state);
        async move {
            Ok::<_, Infallible>(service_fn(move |req: Request<Body>| {
                let state_req = Arc::clone(&state_con);
                async move { handle_request(&*state_req, req).await }
            }))
        }
    });
    let server = Server::bind(&addr).serve(make_svc);
    tokio::spawn(async move {
        loop {
            let value = rec.recv().unwrap();
            match value {
                msg::NotifyMessage::EncodingStarted => {
                    println!("Encoding börjat");
                }
                msg::NotifyMessage::RequestClosed => println!("Request closed"),
            }
        }
        println!("Listener closed!");
    });
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}

pub async fn handle_request(
    state: &RequestState,
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
    state: &RequestState,
    req: Request<Body>,
) -> Result<Response<Body>, Infallible> {
    let mut response = Response::new(Body::empty());

    // let query = req.uri().query();
    // let parsedUri = Url::parse(&req.uri().to_string()).unwrap();
    // let params: HashMap<_, _> = parsedUri.query_pairs().into_owned().collect();

    match (req.method(), req.uri().path()) {
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
            let mut cmd = Command::new("ffmpeg");
            #[rustfmt::skip]
            cmd
                .arg("-hwaccel").arg("dxva2")
                .arg("-i").arg("\\\\192.168.8.150\\Downloads\\Big.Buck.Bunny\\big_buck_bunny.mp4")
                .arg("-acodec").arg("aac")
                .arg("-c:v").arg("h264_nvenc")
                .arg("-preset").arg("slow")
                .arg("-b:v").arg("8M")
                .arg("-movflags").arg("frag_keyframe+empty_moov")
                .arg("-f").arg("mp4")
                .arg("pipe:1")
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());
            let mut child = cmd.spawn().expect("panic! failed to spawn");
            let stdout = child.stdout().take().expect("panic! stdout failed!");
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
