use anyhow::Result;
use ffmpeg_next::frame::Audio;
use ffmpeg_next::software::resampling::Context as ResamplerContext;
use ffmpeg_next::{Packet, format::context::Input};
use rodio::buffer::SamplesBuffer;
use std::num::NonZero;
use std::sync::Arc;

use crate::{Args, ffmpeg};

pub struct AudioInfo {
    index: usize,
    _base: f64,
    resampler: ResamplerContext,
    _device: rodio::MixerDeviceSink,
    _player: rodio::Player,
    decoder: ffmpeg_next::decoder::Audio,
    send: Arc<rodio::queue::SourcesQueueInput>,
}

impl AudioInfo {
    pub fn new(handle: &Input, config: &Args) -> Result<Option<Self>> {
        if config.mute {
            return Ok(None);
        }
        let audio_stream = ffmpeg::get_audio_stream(handle);
        match audio_stream {
            Some(stream) => {
                let decoder = ffmpeg_next::codec::Context::from_parameters(stream.parameters())?
                    .decoder()
                    .audio()?;
                let mut device = rodio::DeviceSinkBuilder::open_default_sink()?;
                device.log_on_drop(false);
                let player = rodio::Player::connect_new(device.mixer());
                let (send, recv) = rodio::queue::queue(true);
                player.append(recv);
                let resampler = ResamplerContext::get(
                    decoder.format(),
                    decoder.channel_layout(),
                    decoder.rate(),
                    ffmpeg_next::format::Sample::F32(ffmpeg_next::format::sample::Type::Packed),
                    ffmpeg_next::ChannelLayout::STEREO,
                    decoder.rate(),
                )?;
                Ok(Some(AudioInfo {
                    index: stream.index(),
                    _base: f64::from(stream.time_base()),
                    decoder,
                    resampler,
                    _device: device,
                    _player: player,
                    send,
                }))
            }
            None => Ok(None),
        }
    }

    pub fn send(&mut self, packet: Packet) -> Result<()> {
        let mut raw = Audio::empty();
        let mut cooked = Audio::empty();

        self.decoder.send_packet(&packet)?;
        while self.decoder.receive_frame(&mut raw).is_ok() {
            self.resampler.run(&raw, &mut cooked)?;

            let flattened_samples: Vec<f32> = cooked
                .plane::<(f32, f32)>(0)
                .iter()
                .flat_map(|&(left, right)| [left, right])
                .collect();

            let buffer = SamplesBuffer::new(
                NonZero::new(2).unwrap(),
                NonZero::new(cooked.rate()).unwrap(),
                flattened_samples,
            );

            self.send.append(buffer);
        }

        Ok(())
    }

    pub fn flush(&mut self) {
        self.decoder.flush();
    }

    pub fn index(&self) -> usize {
        self.index
    }
}
