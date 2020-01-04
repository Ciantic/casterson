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

/// Validates file against safe paths and extensions
///
/// File is valid if it's inside one of the safe directories, and it's
/// extensions is one of the safe ones.
///
/// Safe directory listing should be given in canonicalized form.
pub fn is_safe_file<P: AsRef<Path>, D: AsRef<Path>, E: AsRef<OsStr>>(
    file: P,
    safe_dirs: &[D],
    safe_exts: &[E],
) -> bool {
    // TODO: Nightly Rust map_or(false, |...|)
    canonicalize(&file)
        .map(|file_path| {
            let safe_ext = file_path.extension().map_or(false, |ext| {
                safe_exts.iter().map(AsRef::as_ref).any(|v| v == ext)
            });

            let safe_dir = safe_dirs.iter().any(|d| file_path.starts_with(d));

            safe_ext && safe_dir
        })
        .unwrap_or(false)
}

#[derive(Default, Serialize, Eq, PartialEq, Deserialize, Debug)]
pub struct FFProbeStreams {
    pub codec_name: String,
    pub width: i32,
    pub height: i32,
    // Other omitted
}

#[derive(Default, Serialize, Eq, PartialEq, Deserialize, Debug)]
pub struct FFProbeResult {
    pub streams: Vec<FFProbeStreams>,
    // Other omitted
}

/// Probe video information
pub async fn get_info<P>(file: P) -> Result<FFProbeResult, std::io::Error>
where
    P: AsRef<Path>,
{
    // Fallback to string based error
    let strerr = |err| std::io::Error::new(std::io::ErrorKind::Other, err);

    let mut cmd = Command::new("ffprobe");
    #[rustfmt::skip]
    cmd
        .arg("-v").arg("error")
        .arg("-select_streams").arg("v:0")
        .arg("-show_entries").arg("stream")
        .arg("-print_format").arg("json")
        .arg(file.as_ref())
        .stdout(Stdio::piped()) // redirect the stdout
        .stderr(Stdio::piped()); // redirect the stderr
    let out = cmd.output().await?;

    // Capture stderr and stdout
    let stderr =
        String::from_utf8(out.stderr).map_err(|_| strerr("Unable to decode stderr as UTF-8"))?;
    let stdout =
        String::from_utf8(out.stdout).map_err(|_| strerr("Unable to decode stdout as UTF-8"))?;

    if stderr != "" {
        Err(strerr(&stderr))
    } else {
        serde_json::from_str(&stdout).map_err(|_| strerr("Unable to parse json"))
    }
}

#[derive(Default, Serialize, Deserialize)]
pub struct EncodeVideoOpts {
    pub seek_seconds: i32,
    pub use_subtitles: bool,
    pub tv_resolution: Option<(i32, i32)>,
    pub crop_percent: i32,
}

/// Returns video stream as bytes or io::Error
pub async fn encode<P: AsRef<Path>>(
    file: P,
    opts: EncodeVideoOpts,
) -> Result<impl Stream<Item = Result<bytes::Bytes, std::io::Error>>, std::io::Error> {
    let file_ = file.as_ref();
    let mut video_filters: Vec<String> = vec![];
    let subtitle_file = file_.with_extension("srt");

    if opts.crop_percent > 0 {
        let video_info = get_info(file_).await?;
        if let Some((_tv_width, _tv_height)) = opts.tv_resolution {

            // let tv_ar = tv_width / tv
        }

        /*
        if CROP_MAX_PERCENT > 0:
            crop_width = tv_ar * movie_height
            crop_height = movie_height
            crop_percent = 100 * (movie_width - crop_width) / movie_width

            if crop_percent >= CROP_MAX_PERCENT:
                crop_width = (1.0 - (CROP_MAX_PERCENT / 100)) * movie_width
                crop_percent = 100 * (movie_width - crop_width) / movie_width

            ffmpeg_crop_filter = f"crop={crop_width}:{crop_height}"
            ffmpeg_video_filters.append(ffmpeg_crop_filter)
        */
        // let tv_ar = t
        // let crop_width =
    }

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
    Ok(FramedRead::new(stdout, BytesCodec::new()).map_ok(BytesMut::freeze))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_info() {
        let result = get_info(r"./test_data/big_buck_bunny.mp4").await.unwrap();
        assert_eq!(1920, result.streams[0].width);
    }
}
