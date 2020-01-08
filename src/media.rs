use bytes::BytesMut;
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::ffi::OsString;
use std::fmt::Display;
use std::fs::canonicalize;
use std::iter::Iterator;
use std::path::Path;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use futures::stream::TryStreamExt;
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
struct FFProbeStreams {
    pub codec_name: String,
    pub width: i32,
    pub height: i32,
    // Other omitted
}

#[derive(Default, Serialize, Eq, PartialEq, Deserialize, Debug)]
struct FFProbeFormat {
    pub duration: String,
}

#[derive(Default, Serialize, Eq, PartialEq, Deserialize, Debug)]
struct FFProbeResult {
    pub streams: (FFProbeStreams,), // Only first video stream
    pub format: FFProbeFormat,      // Format (more reliable duration)
                                    // Other omitted
}

#[derive(Default, Serialize, PartialEq, Deserialize, Debug)]
pub struct VideoInfo {
    codec_name: String,
    width: i32,
    height: i32,
    duration: f32,
}

/// Probe video information
pub async fn get_info<P>(file: P) -> Result<VideoInfo, std::io::Error>
where
    P: AsRef<Path>,
{
    // Fallback to string based error
    let strerr = |err| std::io::Error::new(std::io::ErrorKind::Other, err);

    // Getting duration is tricky, you can read about it in here:
    //
    // https://trac.ffmpeg.org/wiki/FFprobeTips#Formatcontainerduration
    //
    // There is three ways: stream (worse), format (better), ffmpeg decoding (most accurate), following uses the format.

    let mut cmd = Command::new("ffprobe");
    #[rustfmt::skip]
    cmd
        .arg("-v").arg("error")
        .arg("-select_streams").arg("v:0") // Only first video stream
        .arg("-show_entries").arg("stream=width,height,codec_name:format=duration")
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

    let ff_result: FFProbeResult =
        serde_json::from_str(&stdout).map_err(|_| strerr("Unable to parse json"))?;

    if stderr != "" {
        Err(strerr(&stderr))
    } else {
        let duration = ff_result
            .format
            .duration
            .parse()
            .map_err(|_| strerr("Unable to parse duration"))?;

        Ok(VideoInfo {
            codec_name: ff_result.streams.0.codec_name,
            duration: duration,
            width: ff_result.streams.0.width,
            height: ff_result.streams.0.height,
        })
    }
}

/// This is not safe or correct way to escape
fn ffmpeg_filter_escape(s: &str) -> String {
    s.replace("\\", "\\\\")
        .replace("'", "\\'")
        .replace(":", "\\:")
}

/// FFMpeg subtitle options
///
/// http://ffmpeg.org/ffmpeg-filters.html#subtitles-1
/// https://fileformats.fandom.com/wiki/SubStation_Alpha
#[derive(Debug, Serialize, Deserialize)]
pub struct FFMpegSubtitleOpts {
    // charenc:
    pub encoding: String,

    // force_style: ASS Style Opts
    pub alignment: i32,
    pub margin_left: i32,
    pub margin_right: i32,
    pub margin_vertical: i32,
    pub size: f32,
    pub spacing: f32,
    pub outline: f32,
    pub font_name: String,
}

impl Default for FFMpegSubtitleOpts {
    fn default() -> Self {
        FFMpegSubtitleOpts {
            // Subtitle opts
            encoding: "UTF-8".into(),

            // ASS Style Opts
            alignment: 1,
            margin_left: 50,
            margin_right: 50,
            margin_vertical: 30,
            size: 26.0,
            spacing: 0.0,
            outline: 1.5,
            font_name: "Arial".into(),
        }
    }
}

impl Display for FFMpegSubtitleOpts {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "charenc='{}':force_style='FontName='{}',Fontsize={},Spacing={},Outline={},MarginL={},MarginR={},MarginV={},Alignment={}'",
            ffmpeg_filter_escape(&self.encoding),
            ffmpeg_filter_escape(&self.font_name), 
            self.size, self.spacing, self.outline, self.margin_left, self.margin_right, self.margin_vertical, self.alignment)
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct EncodeOpts {
    pub seek_seconds: i32,
    pub disable_subtitles: bool,
    pub output_resolution: (i32, i32),
    pub crop_max_percent: i32,
    pub subtitle_opts: FFMpegSubtitleOpts,
}

/// Returns video stream as bytes or io::Error
pub async fn encode<P: AsRef<Path>>(
    file: P,
    opts: EncodeOpts,
) -> Result<impl Stream<Item = Result<bytes::Bytes, std::io::Error>>, std::io::Error> {
    // Fallback to string based error
    let strerr = |err| std::io::Error::new(std::io::ErrorKind::Other, err);

    println!("Start encoding...");

    let file_ = file.as_ref();
    let mut video_filters: Vec<String> = vec![];
    let subtitle_file = file_.with_extension("srt");
    let (output_width, output_height) = opts.output_resolution;

    if opts.crop_max_percent > 0 && output_width > 0 && output_height > 0 {
        if let Ok(video) = get_info(file_).await {
            // Crop from the left and right towards the output_resolution,
            // amount of cropping can be controlled by crop_max_percent
            let video_width: f64 = f64::from(video.width);
            let video_height: f64 = f64::from(video.height);
            // let video_ar: f64 = video_width / video_height;
            let output_ar: f64 = f64::from(output_width) / f64::from(output_height);
            let mut crop_width: f64 = output_ar * video_height;
            let crop_height: f64 = video_height;
            let crop_percent: f64 = 100.0f64 * (video_width - crop_width) / video_width;
            let crop_max_percent = f64::from(opts.crop_max_percent);

            if crop_percent > crop_max_percent {
                crop_width = (1f64 - (crop_max_percent / 100f64)) * video_width;
            }

            video_filters.push(format!(
                "crop={}:{}", crop_width,crop_height
            ));
        }
    }

    if !opts.disable_subtitles && subtitle_file.exists() {
        video_filters.push(format!("setpts=PTS+{}/TB", opts.seek_seconds));
        video_filters.push(format!("subtitles='{}':{}",
            ffmpeg_filter_escape(&subtitle_file.to_string_lossy()),
            opts.subtitle_opts
        ));
        video_filters.push("setpts=PTS-STARTPTS".into());
    }

    let mut cmd = Command::new("ffmpeg");
    #[rustfmt::skip]
    cmd
        .arg("-loglevel").arg("error")
        .arg("-stats")
        .arg("-ss").arg(opts.seek_seconds.to_string())
        .arg("-hwaccel").arg("dxva2")
        .arg("-i").arg(file_.as_os_str())
        .args(if video_filters.len() > 0 {
                vec!["-vf".into(), video_filters.join(",")]
            } else {
                vec![]
            })
        .arg("-acodec").arg("aac")
        .arg("-c:v").arg("h264_nvenc")
        .arg("-preset").arg("slow")
        .arg("-b:v").arg("8M")
        .arg("-movflags").arg("frag_keyframe+empty_moov")
        .arg("-f").arg("mp4")
        .arg("pipe:1")
        .stdout(Stdio::piped()); // redirect the stdout
        // .stderr(Stdio::piped()); // redirect the stderr (suppressed)

    let mut child = cmd.spawn()?;
    let stdout = child
        .stdout()
        .take()
        .map_or(Err(strerr("Unable to capture stdout")), Ok)?;

    Ok(FramedRead::new(stdout, BytesCodec::new()).map_ok(|v| BytesMut::freeze(v)))
}

// Unit tests
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_info() {
        let result = get_info(r"./test_data/big_buck_bunny.mp4").await.unwrap();
        assert_eq!(
            VideoInfo {
                codec_name: "h264".into(),
                width: 1920,
                height: 1080,
                duration: 596.50134
            },
            result
        );
    }
}
