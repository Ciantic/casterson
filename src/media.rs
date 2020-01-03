use bytes::BytesMut;
use futures::Stream;
use futures_util::TryStreamExt;
use std::ffi::OsStr;
use std::ffi::OsString;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use tokio_util::codec::{BytesCodec, FramedRead};
use walkdir;

/// Scan media files
pub fn scan_media_files<S: AsRef<OsStr>>(dirs: &[PathBuf], exts: &[S]) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = vec![];
    let exts_os: Vec<OsString> = exts.iter().map(OsString::from).collect();
    for dir in dirs {
        for entry in walkdir::WalkDir::new(dir) {
            let path = entry.unwrap().into_path();
            let ext = path
                .extension()
                .map_or(OsString::from(""), OsStr::to_os_string);
            if exts_os.contains(&ext) {
                println!("FOUND IT!");
                paths.push(path);
            }
        }
    }
    paths
}

/// Returns video stream as bytes or io::Error
pub fn encode<S: AsRef<OsStr>>(
    file: S,
) -> impl Stream<Item = Result<bytes::Bytes, std::io::Error>> {
    let mut cmd = Command::new("ffmpeg");
    #[rustfmt::skip]
    cmd
        .arg("-hwaccel").arg("dxva2")
        .arg("-i").arg(file)
        .arg("-acodec").arg("aac")
        .arg("-c:v").arg("h264_nvenc")
        .arg("-preset").arg("slow")
        .arg("-b:v").arg("8M")
        .arg("-movflags").arg("frag_keyframe+empty_moov")
        .arg("-f").arg("mp4")
        .arg("pipe:1")
        .stdout(Stdio::piped()) // redirect the stdout
        .stderr(Stdio::piped()); // redirect the stderr (suppressed)
    let mut child = cmd.spawn().expect("panic! failed to spawn");
    let stdout = child.stdout().take().expect("panic! stdout failed!");
    FramedRead::new(stdout, BytesCodec::new()).map_ok(BytesMut::freeze)
}
