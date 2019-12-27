use bytes::BytesMut;
use futures_util::TryFutureExt;
use futures_util::TryStreamExt;
use hyper::header::HeaderValue;
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use hyper::{Method, StatusCode};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::process::Stdio;
use tokio::fs::File;
use tokio::process::Command;
use tokio_util::codec::{BytesCodec, FramedRead};

mod chromecast;

async fn handle_request(req: Request<Body>) -> Result<Response<Body>, Infallible> {
    match (req.method(), req.uri().path()) {
        // Stream a file from a disk
        (&Method::GET, "/file") => {
            let stream = File::open("C:\\Source\\Backup_Ignore.txt")
                .map_ok(|file| FramedRead::new(file, BytesCodec::new()).map_ok(BytesMut::freeze))
                .try_flatten_stream();
            let s = Body::wrap_stream(stream);
            let mut response = Response::new(s);
            return Ok(response);
        }

        // Stream from shell execute, e.g. using "curl" executable
        //
        // Borrows from: https://github.com/tokio-rs/tokio/blob/master/tokio/src/process/mod.rs
        (&Method::GET, "/exec") => {
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
            let mut response = Response::new(s);
            response
                .headers_mut()
                .insert("Content-Type", HeaderValue::from_static("video/mp4"));
            // // Ensure the child process is spawned in the runtime so it can
            // // make progress on its own while we await for any output.
            // tokio::spawn(async {
            //     let status = child.await
            //         .expect("child process encountered an error");

            //     println!("child status was: {}", status);
            // });
            return Ok(response);
        }

        (&Method::GET, "/start") => {
            tokio::spawn(async {
                let receiver = chromecast::get_default_media_receiver("192.168.8.106");
                receiver.cast("http://192.168.8.103:3000/exec");
            });
            let mut response = Response::new(Body::empty());
            *response.status_mut() = StatusCode::OK;
            return Ok(response);
        }

        (&Method::GET, "/pause") => {
            let receiver = chromecast::get_default_media_receiver("192.168.8.106");
            receiver.pause();
            let mut response = Response::new(Body::empty());
            *response.status_mut() = StatusCode::OK;
            return Ok(response);
        }

        (&Method::GET, "/play") => {
            let receiver = chromecast::get_default_media_receiver("192.168.8.106");
            receiver.play();
            let mut response = Response::new(Body::empty());
            *response.status_mut() = StatusCode::OK;
            return Ok(response);
        }

        (&Method::GET, "/stop") => {
            let receiver = chromecast::get_default_media_receiver("192.168.8.106");
            receiver.stop();
            let mut response = Response::new(Body::empty());
            *response.status_mut() = StatusCode::OK;
            return Ok(response);
        }

        // 404 not found
        _ => {
            let mut response = Response::new(Body::empty());
            *response.status_mut() = StatusCode::NOT_FOUND;
            return Ok(response);
        }
    };
}

#[tokio::main]
async fn main() {
    let addr = SocketAddr::from(([192, 168, 8, 103], 3000));
    let make_svc =
        make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle_request)) });
    let server = Server::bind(&addr).serve(make_svc);
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }
}
