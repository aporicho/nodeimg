use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use image::DynamicImage;
use uuid::Uuid;

use crate::execution::ExecutorError;

pub trait VideoDecodeBackend: Send + Sync {
    fn extract_frames(&self, input_path: &Path, output_dir: &Path) -> Result<(), ExecutorError>;
}

pub trait VideoEncodeBackend: Send + Sync {
    fn new_session(
        &self,
        path: PathBuf,
        fps: u32,
    ) -> Result<Box<dyn VideoEncodeSession>, ExecutorError>;
}

pub trait VideoEncodeSession: Send {
    fn push_frame(&mut self, frame_index: u32, frame: &DynamicImage) -> Result<(), ExecutorError>;
    fn finish(self: Box<Self>) -> Result<(), ExecutorError>;
    fn path(&self) -> &Path;
}

#[derive(Default)]
pub struct FfmpegVideoBackend;

impl VideoDecodeBackend for FfmpegVideoBackend {
    fn extract_frames(&self, input_path: &Path, output_dir: &Path) -> Result<(), ExecutorError> {
        fs::create_dir_all(output_dir).map_err(io_error)?;
        let output_pattern = output_dir.join("frame_%06d.png");

        run_ffmpeg([
            "-v",
            "error",
            "-y",
            "-i",
            path_arg(input_path).as_str(),
            "-start_number",
            "0",
            path_arg(&output_pattern).as_str(),
        ])
    }
}

impl VideoEncodeBackend for FfmpegVideoBackend {
    fn new_session(
        &self,
        path: PathBuf,
        fps: u32,
    ) -> Result<Box<dyn VideoEncodeSession>, ExecutorError> {
        Ok(Box::new(FfmpegEncodeSession::new(path, fps)?))
    }
}

struct FfmpegEncodeSession {
    path: PathBuf,
    fps: u32,
    frames_dir: PathBuf,
}

impl FfmpegEncodeSession {
    fn new(path: PathBuf, fps: u32) -> Result<Self, ExecutorError> {
        if fps == 0 {
            return Err(ExecutorError::InvalidInput {
                message: "save_video fps must be greater than 0".into(),
            });
        }

        let output_dir = path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| PathBuf::from("."));
        fs::create_dir_all(&output_dir).map_err(io_error)?;

        let frames_dir = std::env::temp_dir().join(format!(
            "nodeimg_video_encode_{}_{}",
            std::process::id(),
            Uuid::new_v4()
        ));
        fs::create_dir_all(&frames_dir).map_err(io_error)?;

        Ok(Self {
            path,
            fps,
            frames_dir,
        })
    }
}

impl VideoEncodeSession for FfmpegEncodeSession {
    fn push_frame(&mut self, frame_index: u32, frame: &DynamicImage) -> Result<(), ExecutorError> {
        let frame_path = self.frames_dir.join(format!("frame_{frame_index:06}.png"));
        frame.save(&frame_path).map_err(io_error)
    }

    fn finish(self: Box<Self>) -> Result<(), ExecutorError> {
        encode_frames_to_mp4(&self.frames_dir, &self.path, self.fps)?;
        let _ = fs::remove_dir_all(&self.frames_dir);
        Ok(())
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

pub fn decode_video_frames(path: &Path) -> Result<Vec<DynamicImage>, ExecutorError> {
    let backend = FfmpegVideoBackend;
    let temp_dir = std::env::temp_dir().join(format!(
        "nodeimg_video_decode_{}_{}",
        std::process::id(),
        Uuid::new_v4()
    ));
    backend.extract_frames(path, &temp_dir)?;
    let frames = load_png_sequence(&temp_dir)?;
    let _ = fs::remove_dir_all(&temp_dir);
    Ok(frames)
}

pub fn encode_frames_to_mp4(
    frames_dir: &Path,
    output_path: &Path,
    fps: u32,
) -> Result<(), ExecutorError> {
    if fps == 0 {
        return Err(ExecutorError::InvalidInput {
            message: "fps must be greater than 0".into(),
        });
    }

    let input_pattern = frames_dir.join("frame_%06d.png");
    run_ffmpeg([
        "-v",
        "error",
        "-y",
        "-framerate",
        &fps.to_string(),
        "-start_number",
        "0",
        "-i",
        path_arg(&input_pattern).as_str(),
        "-vf",
        "scale=trunc(iw/2)*2:trunc(ih/2)*2",
        "-c:v",
        "libx264",
        "-pix_fmt",
        "yuv420p",
        path_arg(output_path).as_str(),
    ])
}

pub fn load_png_sequence(dir: &Path) -> Result<Vec<DynamicImage>, ExecutorError> {
    let mut entries = fs::read_dir(dir)
        .map_err(io_error)?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "png"))
        .collect::<Vec<_>>();
    entries.sort();

    let mut frames = Vec::with_capacity(entries.len());
    for path in entries {
        frames.push(image::open(path).map_err(io_error)?);
    }
    Ok(frames)
}

fn run_ffmpeg<const N: usize>(args: [&str; N]) -> Result<(), ExecutorError> {
    let output = Command::new("ffmpeg")
        .args(args)
        .output()
        .map_err(io_error)?;
    if output.status.success() {
        return Ok(());
    }

    Err(ExecutorError::RuntimeFailed {
        message: String::from_utf8_lossy(&output.stderr).trim().to_string(),
    })
}

fn path_arg(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn io_error(error: impl std::fmt::Display) -> ExecutorError {
    ExecutorError::RuntimeFailed {
        message: error.to_string(),
    }
}
