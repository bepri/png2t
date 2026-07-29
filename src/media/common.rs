use std::{sync::OnceLock, time::Instant};

use ffmpeg_next::format::Pixel;

use crate::Args;

pub static START_TIME: OnceLock<Instant> = OnceLock::new();

pub fn map_deprecated_format(format: Pixel) -> Pixel {
    match format {
        Pixel::YUVJ420P => Pixel::YUV420P,
        Pixel::YUVJ422P => Pixel::YUV422P,
        Pixel::YUVJ440P => Pixel::YUV440P,
        Pixel::YUVJ444P => Pixel::YUV444P,
        format => format,
    }
}

pub fn get_target_dimensions(config: &Args, curr_w: u32, curr_h: u32) -> (u32, u32) {
    if let Some(s) = &config.size {
        (s.width, s.height)
    } else if !config.preserve_dims {
        if curr_w > curr_h {
            (64, (64 * curr_h) / curr_w)
        } else {
            ((64 * curr_w) / curr_h, 64)
        }
    } else {
        (curr_w, curr_h)
    }
}
