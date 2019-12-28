extern crate clap;

use bytes::BytesMut;
use futures_util::TryFutureExt;
use futures_util::TryStreamExt;
use hyper::header::HeaderValue;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use hyper::{Method, StatusCode};
use std::collections::HashMap;
use std::convert::Infallible;
use std::net::SocketAddr;
use std::process::Stdio;
use tokio::fs::File;
use tokio::process::Command;
use tokio_util::codec::{BytesCodec, FramedRead};

use clap::{App, Arg};

use url::Url;

mod chromecast;
use chromecast::BaseMediaReceiver;

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

async fn handle_other_request(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    let mut response = Response::new(Body::empty());

    let query = req.uri().query();
    let parsedUri = Url::parse(&req.uri().to_string()).unwrap();
    let params: HashMap<_, _> = parsedUri.query_pairs().into_owned().collect();

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
        }

        // 404 not found
        _ => {
            *response.status_mut() = StatusCode::NOT_FOUND;
        }
    }
    Ok(response)
}

async fn handle_request(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    if req.uri().path().starts_with("/chromecast") {
        handle_chromecast_request(req).await
    } else {
        handle_other_request(req).await
    }
}

fn scan_media_files(dir: &str, extensions: Vec<&str>) -> Vec<String> {
    unimplemented!()
}

#[tokio::main]
async fn main() {
    let matches = App::new("Casterson")
        .version("0.1")
        .author("Jari Pennanen <ciantic@oksidi.com>")
        .about("It just keeps on casting")
        .arg(
            Arg::with_name("DIR")
                .help("Directories to scan for media files")
                .required(true)
                .multiple(true)
                .index(1),
        )
        .get_matches();

    let dirs = matches.values_of("DIR").unwrap();

    println!("Dirs {:?}", dirs);

    let addr = SocketAddr::from(([192, 168, 8, 103], 3000));
    let make_svc =
        make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle_request)) });
    let server = Server::bind(&addr).serve(make_svc);
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}
