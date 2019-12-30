extern crate clap;

use crossbeam::unbounded;
use std::io::Result as IOResult;
use std::net::IpAddr;
use std::path::PathBuf;
use structopt::StructOpt;

pub mod api;
pub mod chromecast;
pub mod media;
pub mod msg;

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
    #[structopt(name = "DIR", parse(try_from_str = parse_path_canonicalized))]
    dir: Vec<PathBuf>,
}

#[tokio::main]
async fn main() {
    let opt = CliOpts::from_args();
    for dir in opt.dir {
        println!("Using media directory: {}", dir.display());
    }
    let (notify, rec) = unbounded::<msg::NotifyMessage>();
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
    });
    api::create_server(notify.clone(), opt.ip, opt.port).await;
}

/// Parse path and canonicalize
fn parse_path_canonicalized(src: &str) -> IOResult<PathBuf> {
    let p = PathBuf::from(src);
    p.canonicalize()
}
