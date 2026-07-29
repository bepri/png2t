use std::path::PathBuf;

use anyhow::{Context as _, Result};
use ffmpeg_next::{self as ffmpeg, Stream, format::context::Input};

pub fn get_input(path: &PathBuf) -> Result<Input> {
    ffmpeg::init()?;
    ffmpeg::format::input(&path).with_context(|| format!("Unable to open {}.", path.display()))
}

pub fn get_video_stream(input: &Input) -> Option<Stream<'_>> {
    input.streams().best(ffmpeg::media::Type::Video)
}

pub fn get_audio_stream(input: &Input) -> Option<Stream<'_>> {
    input.streams().best(ffmpeg::media::Type::Audio)
}

pub fn get_dimensions(input: &Input) -> Result<(u32, u32)> {
    let stream = input
        .streams()
        .best(ffmpeg::media::Type::Video)
        .with_context(|| "Video stream not found.")?;
    let decoder = ffmpeg::codec::Context::from_parameters(stream.parameters())?
        .decoder()
        .video()?;

    Ok((decoder.width(), decoder.height()))
}
