extern crate clap;

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

mod api;
mod msg;

fn scan_media_files(dir: &str, extensions: Vec<&str>) -> Vec<String> {
    unimplemented!()
}

#[derive(StructOpt, Debug)]
#[structopt(name = "basic")]
struct CliOpts {
    // The number of occurrences of the `v/verbose` flag
    /// Verbose mode (-v, -vv, -vvv, etc.)
    #[structopt(short, long, parse(from_occurrences))]
    verbose: u8,

    /// IP address of the casterson server
    #[structopt(short, long, default_value = "0.0.0.0")]
    ip: IpAddr,

    /// Port of casterson server
    #[structopt(short, long, default_value = "3000")]
    port: u16,

    /// Media extensions
    #[structopt(short, long, default_value = "mp4,mkv,avi,mov", value_delimiter = ",")]
    media_exts: Vec<String>,

    /// Directories of media files
    #[structopt(name = "DIR", parse(from_os_str))]
    dir: Vec<PathBuf>,
}

#[tokio::main]
async fn main() {
    let opt = CliOpts::from_args();

    let matches = App::new("Casterson")
        .version("0.1")
        .author("Jari Pennanen <ciantic@oksidi.com>")
        .about("It just keeps on casting")
        .arg(
            Arg::with_name("IP")
                .long("ip")
                .help("IP address of the casterson server")
                .default_value("0.0.0.0")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("PORT")
                .long("port")
                .help("Port of casterson server")
                .default_value("3000")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("MEDIA_EXTS")
                .long("media-exts")
                .short("e")
                .help("Media file extensions")
                .default_value("mp4,mkv,avi,mov")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("DIR")
                .help("Directories to scan for media files")
                .required(true)
                .multiple(true)
                .index(1),
        )
        .get_matches();

    // let addr = SocketAddr::from((opt.ip, opt.port));
    let (notify, rec) = unbounded::<api::msg::NotifyMessage>();
    api::create_server(notify.clone(), opt.ip, opt.port).await;

    // let state = Arc::new(api::RequestState {
    //     foo: 321,
    //     zoo: vec![1, 2, 3],
    //     notifier: notify.clone(),
    // });
    // let make_svc = make_service_fn(move |_| {
    //     let state_con = Arc::clone(&state);
    //     async move {
    //         Ok::<_, Infallible>(service_fn(move |req: Request<Body>| {
    //             let state_req = Arc::clone(&state_con);
    //             async move { api::handle_request(&*state_req, req).await }
    //         }))
    //     }
    // });
    // let server = Server::bind(&addr).serve(make_svc);
    // tokio::spawn(async move {
    //     loop {
    //         let value = rec.recv().unwrap();
    //         match value {
    //             api::NotifyMessage::EncodingStarted => {
    //                 println!("Encoding börjat");
    //             }
    //             api::NotifyMessage::RequestClosed => println!("Request closed"),
    //         }
    //     }
    //     println!("Listener closed!");
    // });
    // if let Err(e) = server.await {
    //     eprintln!("server error: {}", e);
    // }
}
