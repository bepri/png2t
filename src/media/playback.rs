use std::{io::Write as _, time::Duration};

use anyhow::{Context as _, Result};
use crossterm::{
    cursor::{MoveDown, MoveTo, MoveToColumn},
    event::{Event, KeyCode, KeyEvent, KeyModifiers, poll, read},
};
use ffmpeg_next::format::context::Input;

use super::audio::AudioInfo;
use super::video::VideoInfo;
use crate::Args;

/// Plays or displays the media file (video or image) stored in `handle`.
pub fn play_media(handle: &mut Input, config: &Args, pos: (u16, u16)) -> Result<()> {
    let mut video_info = VideoInfo::new(handle, config)?;
    let mut audio_info = AudioInfo::new(handle, config)?;

    match video_info.is_video {
        true => play_video(handle, config, pos, &mut video_info, &mut audio_info),
        false => play_image(handle, pos, &mut video_info),
    }
}

/// Plays a static image file.
fn play_image(handle: &mut Input, pos: (u16, u16), video_info: &mut VideoInfo) -> Result<()> {
    if let Some((_, packet)) = handle.packets().next() {
        video_info.send(packet)?;
    }
    // Move cursor to the bottom of the printed frame before returning
    print!(
        "{}",
        MoveTo(pos.0, pos.1 + (video_info.decoder.height() / 2) as u16)
    );
    Ok(())
}

/// Plays a video/GIF file.
fn play_video(
    handle: &mut Input,
    config: &Args,
    pos: (u16, u16),
    video_info: &mut VideoInfo,
    audio_info: &mut Option<AudioInfo>,
) -> Result<()> {
    // Hot loop to display the video.
    loop {
        handle.seek(0, ..)?;
        video_info.decoder.flush();
        if let Some(audio) = audio_info {
            audio.flush();
        }

        for (stream, packet) in handle.packets() {
            let index = stream.index();

            if index == video_info.index {
                video_info.send(packet)?;
            } else if let Some(audio_info) = audio_info
                && audio_info.index() == index
            {
                audio_info.send(packet)?;
            }

            if poll(Duration::from_millis(1)).unwrap() {
                let event = read().unwrap();
                if [
                    Event::Key(KeyCode::Char('q').into()),
                    Event::Key(KeyCode::Esc.into()),
                    Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
                ]
                .contains(&event)
                {
                    break;
                }
            }

            // Reset cursor for next frame and overwrite old frame
            print!("{}", MoveTo(pos.0, pos.1));
        }

        if !config.loop_video {
            break;
        }
    }

    // Move cursor to the bottom of the printed frame before returning
    print!(
        "{}",
        MoveTo(pos.0, pos.1 + video_info.decoder.height() as u16)
    );

    Ok(())
}

/// Internal function to display one frame into the terminal.
///
/// # Errors
/// I/O errors can occur when flushing `stdout`
pub fn display_frame(frame: &ffmpeg_next::frame::Video) -> Result<()> {
    let data = frame.data(0);
    let stride = frame.stride(0);
    let get_pixel = |x: usize, y: usize| -> [u8; 4] {
        let offset = (y * stride) + (x * 4);
        *data[offset..].first_chunk().unwrap()
    };

    for y in (0..frame.height() as usize).step_by(2) {
        for x in 0..frame.width() as usize {
            let upper = get_pixel(x, y);
            let lower = get_pixel(x, y + 1);

            // This if/else is to handle image transparency, but the first case is the simplest to understand.
            // Using the unicode ▄ symbol, we can use ANSI Truecolor to color its foreground and background.
            // This means that each character in the terminal can represent two pixels, one higher and one lower.
            // If [3] is 0 on a pixel, this means it should be transparent, so we leave the foreground/background uncolored
            // or use other means to keep that pixel transparent.
            if upper[3] != 0 && lower[3] != 0 {
                print!(
                    "\x1b[48;2;{};{};{}m\x1b[38;2;{};{};{}m▄\x1b[0m",
                    upper[0], upper[1], upper[2], lower[0], lower[1], lower[2]
                );
            } else if upper[3] == 0 && lower[3] == 0 {
                print!(" ");
            } else if upper[3] != 0 && lower[3] == 0 {
                print!("\x1b[38;2;{};{};{}m▀\x1b[0m", upper[0], upper[1], upper[2]);
            } else {
                print!("\x1b[38;2;{};{};{}m▄\x1b[0m", lower[0], lower[1], lower[2]);
            }

            std::io::stdout()
                .flush()
                .with_context(|| format!("\nFailed to print image at ({x}, {y})"))?;
        }
        print!("{}{}", MoveDown(1), MoveToColumn(0));
    }

    Ok(())
}
