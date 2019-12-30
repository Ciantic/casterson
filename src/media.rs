use std::ffi::OsStr;
use std::ffi::OsString;
use std::path::PathBuf;
use walkdir;

pub fn scan_media_files(dirs: &[PathBuf], exts: &[String]) -> Vec<PathBuf> {
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
