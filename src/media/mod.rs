//! Media processing

mod audio;
mod common;
mod playback;
mod video;

use anyhow::{Context as _, Result};
use crossterm::{
    cursor::{MoveToColumn, MoveUp, position},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use ffmpeg_next::format::context::Input;

use crate::{
    Args,
    ffmpeg::{self, get_input},
};

use self::common::get_target_dimensions;

/// A wrapper for a media file.
///
/// This struct can represent a video of any length and stores it internally.
/// An external temporary directory is used to store media when creating an instance.
/// The `Drop` trait is implemented to clear this temp directory.
pub struct Media {
    handle: Input,
    config: Args,
}

impl Media {
    pub fn new(config: Args) -> Result<Self> {
        Ok(Media {
            handle: get_input(&config.file)
                .with_context(|| format!("Unable to open {}.", config.file.display()))?,
            config,
        })
    }

    /// Transform each frame based on command line flags
    ///
    /// Pulls all information from `self.config`.
    /// This function has potential to be the slowest in the rendering process if done with too many flags - be careful in here
    #[expect(unused)]
    pub fn transform(&mut self) -> Result<()> {
        unimplemented!()
    }

    /// Plays the media file in the terminal. Must be initialized with `self.load_frames()` first.
    ///
    /// # Errors
    /// Can error out if `self` contains a video but the FPS cannot be determined.
    /// Also may fail on I/O or sound device errors.
    /// Can possibly fail on file I/O, but is only possible by race condition with another program modifying the storage directory.
    pub fn render(&mut self) -> Result<()> {
        // Create buffer space in the terminal for the image before printing
        let (w, h) = ffmpeg::get_dimensions(&self.handle)?;
        let (_, target_height) = get_target_dimensions(&self.config, w, h);
        for _ in 0..(target_height / 2) {
            println!();
        }

        // Turn off the fancy stuff in the terminal. I'm using this to later emulate C's `getchar`
        enable_raw_mode()?;

        // Reset cursor to where the top-left pixel should print
        print!("{}{}", MoveToColumn(0), MoveUp((target_height / 2) as u16));

        // Save this location for quicker cursor resets when new frames are printed
        let pos = position().unwrap_or((0, 0));

        playback::play_media(&mut self.handle, &self.config, pos)?;

        disable_raw_mode()?;
        Ok(())
    }
}
