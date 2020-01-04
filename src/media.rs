use bytes::BytesMut;
use futures::Stream;
use futures_util::TryStreamExt;
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::ffi::OsString;
use std::fs::canonicalize;
use std::path::Path;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use tokio_util::codec::{BytesCodec, FramedRead};
use walkdir;

/// Scan media files
pub fn scan_media_files<E: AsRef<OsStr>, P: AsRef<Path>>(dirs: &[P], exts: &[E]) -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = vec![];
    let exts_os: Vec<OsString> = exts.iter().map(OsString::from).collect();
    for dir in dirs {
        for entry in walkdir::WalkDir::new(dir) {
            let path = entry.unwrap().into_path();
            let ext = path
                .extension()
                .map_or(OsString::from(""), OsStr::to_os_string);
            if exts_os.contains(&ext) {
                paths.push(path);
            }
        }
    }
    paths
}

/// Validates media file
///
/// File is valid if it's inside one of the safe directories, and it's
/// extensions is one of the safe ones.
///
/// Safe directory listing should be in canonicalized form
pub fn is_valid_media_file<P: AsRef<Path>, D: AsRef<Path>, E: AsRef<OsStr>>(
    file: P,
    dirs: &[D],
    exts: &[E],
) -> bool
where
    E: Into<OsString>,
{
    // I bet there is a better way than recreating the collection?
    let exts_as_ostrings: Vec<OsString> = exts.iter().map(|v| v.into()).collect();

    canonicalize(&file)
        .map(|file_path| {
            let safe_ext = file_path
                .extension()
                .map(|ext| exts_as_ostrings.contains(&ext.to_os_string()))
                .unwrap_or(false);

            let safe_dir = dirs.iter().any(|d| file_path.starts_with(d));

            safe_ext && safe_dir
        })
        .unwrap_or(false)
}

#[derive(Default, Serialize, Deserialize)]
pub struct EncodeVideoOpts {
    pub seek_seconds: i32,
    pub use_subtitles: bool,
    pub tv_resolution: Option<(i32, i32)>,
    pub crop_percent: i32,
}

/// Returns video stream as bytes or io::Error
pub fn encode<P: AsRef<Path>>(
    file: P,
    opts: EncodeVideoOpts,
) -> impl Stream<Item = Result<bytes::Bytes, std::io::Error>> {
    let file_ = file.as_ref();
    let mut video_filters: Vec<String> = vec![];
    let subtitle_file = file_.with_extension("srt");

    println!("subtitle file {}", subtitle_file.to_string_lossy());
    if opts.use_subtitles && subtitle_file.exists() {
        let ffmpeg_subtitle_filename = subtitle_file
            .to_string_lossy()
            .replace("\\", "\\\\")
            .replace("'", "\\'")
            .replace(":", "\\:");

        // Subtitle alignment
        //
        // Values may be 1=Left, 2=Centered, 3=Right. Add 4 to the value for a
        // "Toptitle". Add 8 to the value for a "Midtitle". eg. 5 =
        // left-justified toptitle
        let subtitle_alignment = 1;

        let subtitle_margin_left = 50;
        let subtitle_margin_right = 50;
        let subtitle_margin_vertical = 30;
        let subtitle_encoding = "UTF-8";

        let ffmpeg_subtitle_filter = format!("subtitles='{subtitle_filename}':charenc='{subtitle_encoding}':force_style='FontName='Arial',Fontsize=32,Outline=2,MarginL={subtitle_margin_left},MarginR={subtitle_margin_right},MarginV={subtitle_margin_vertical},Alignment={subtitle_alignment}'", 
                subtitle_filename = ffmpeg_subtitle_filename,
                subtitle_encoding = subtitle_encoding,
                subtitle_margin_left = subtitle_margin_left,
                subtitle_margin_right = subtitle_margin_right,
                subtitle_margin_vertical = subtitle_margin_vertical,
                subtitle_alignment = subtitle_alignment);

        video_filters.push(format!("setpts=PTS+{}/TB", opts.seek_seconds));
        video_filters.push(ffmpeg_subtitle_filter);
        video_filters.push("setpts=PTS-STARTPTS".into());
    }

    let video_filters_arg: Vec<String> = {
        let vfs = video_filters.join(",");
        if vfs != "" {
            vec!["-vf".into(), vfs]
        } else {
            vec![]
        }
    };

    let mut cmd = Command::new("ffmpeg");
    #[rustfmt::skip]
    cmd
        .arg("-ss").arg(opts.seek_seconds.to_string())
        .arg("-hwaccel").arg("dxva2")
        .arg("-i").arg(file_.as_os_str())
        .args(video_filters_arg)
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
