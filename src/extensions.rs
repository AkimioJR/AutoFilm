// AutoFilm 需要识别的视频文件后缀；视频会被转换为同名 .strm 文件。
pub const VIDEO_EXTS: &[&str] = &[
    ".mp4", ".mkv", ".flv", ".avi", ".wmv", ".ts", ".rmvb", ".webm", ".mpg", ".m2ts",
];

// 可选下载的伴生文件类型，行为与 Python 版本保持一致。
pub const SUBTITLE_EXTS: &[&str] = &[".ass", ".srt", ".ssa", ".sub"];

pub const IMAGE_EXTS: &[&str] = &[".png", ".jpg"];

pub const NFO_EXTS: &[&str] = &[".nfo"];
