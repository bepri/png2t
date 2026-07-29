use std::time::{Duration, Instant};

use anyhow::{Context as _, Result};
use ffmpeg_next::{
    Packet,
    format::{Pixel, context::Input},
    frame::Video,
    software::scaling::{Context as ScalerContext, Flags as ScalerFlags},
};

use super::common::{START_TIME, get_target_dimensions, map_deprecated_format};

use crate::{cli::Args, ffmpeg};

pub struct VideoInfo {
    pub index: usize,
    pub base: f64,
    pub decoder: ffmpeg_next::decoder::Video,
    pub scaler: ScalerContext,
    pub is_video: bool,
}

impl VideoInfo {
    pub fn new(handle: &Input, config: &Args) -> Result<Self> {
        let video_stream =
            ffmpeg::get_video_stream(handle).with_context(|| "No video stream to play.")?;
        let codec_context =
            ffmpeg_next::codec::Context::from_parameters(video_stream.parameters())?;
        let video_decoder = codec_context.decoder().video()?;
        let (target_width, target_height) =
            get_target_dimensions(config, video_decoder.width(), video_decoder.height());
        let input_format = map_deprecated_format(video_decoder.format());
        let video_scaler = ScalerContext::get(
            input_format,
            video_decoder.width(),
            video_decoder.height(),
            Pixel::RGBA,
            target_width,
            target_height,
            ScalerFlags::BILINEAR,
        )?;
        let is_video = video_stream.frames() > 1 || video_stream.duration() > 1;
        Ok(VideoInfo {
            index: video_stream.index(),
            base: f64::from(video_stream.time_base()),
            decoder: video_decoder,
            scaler: video_scaler,
            is_video,
        })
    }

    pub fn send(&mut self, packet: Packet) -> Result<()> {
        let mut raw = Video::empty();
        let mut cooked = Video::empty();
        let start_time = START_TIME.get_or_init(Instant::now);

        self.decoder.send_packet(&packet)?;
        while self.decoder.receive_frame(&mut raw).is_ok() {
            self.scaler.run(&raw, &mut cooked)?;

            if let Some(pts) = raw.pts() {
                let pts_seconds = pts as f64 * self.base;
                let expected_time = *start_time + Duration::from_secs_f64(pts_seconds);
                let now = Instant::now();
                if expected_time > now {
                    std::thread::sleep(expected_time - now);
                }
            }

            crate::media::playback::display_frame(&cooked)?;
        }

        Ok(())
    }
}
