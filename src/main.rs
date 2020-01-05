extern crate clap;

use crossbeam::channel::Sender;
use crossbeam::unbounded;
use std::io::Result as IOResult;
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::Arc;
use structopt::StructOpt;

pub mod api;
pub mod chromecast;
pub mod media;
pub mod msg;

#[derive(StructOpt, Debug)]
#[structopt(name = "basic")]
pub struct CliOpts {
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
    #[structopt(name = "DIR", parse(try_from_str = parse_path_canonicalized))]
    dir: Vec<PathBuf>,
}

#[derive(Debug)]
pub struct AppState {
    pub opts: CliOpts,
    pub notifier: Sender<msg::NotifyMessage>,
}

#[tokio::main]
async fn main() {
    let (notify, rec) = unbounded::<msg::NotifyMessage>();
    let state = Arc::new(AppState {
        opts: CliOpts::from_args(),
        notifier: notify.clone(),
    });
    for dir in &*state.opts.dir {
        println!("Using media directory: {}", dir.display());
    }
    tokio::spawn(async move {
        loop {
            let value = rec.recv().unwrap();
            match value {
                msg::NotifyMessage::ErrorDuringCasting(err) => {
                    println!("Error during casting {:?}", err);
                }
                _ => (),
            }
        }
    });
    api::create_server(state).await;
}

/// Parse path and canonicalize
fn parse_path_canonicalized(src: &str) -> IOResult<PathBuf> {
    let p = PathBuf::from(src);
    p.canonicalize()
}
