#![doc = include_str!("../README.md")]
mod cli;
mod ffmpeg;
mod media;

use anyhow::Result;

use crate::cli::{Args, Parser};
use crate::media::Media;

fn main() -> Result<()> {
    #[cfg(target_os = "windows")]
    {
        println!(
            "Warning: This program is capable of running on Windows, but it faces a lot of difficulties due to default Windows behavior."
        );
        println!(
            "The main issue is that video playback is likely going to be extremely slow. This is not a performance issue - Windows' printing API is just extremely slow."
        );

        use windows::Win32::System::Console::*;

        unsafe {
            let handle = GetStdHandle(STD_HANDLE(11))
                .with_context(|| "Could not communicate with output device.")?;

            SetConsoleMode(handle, CONSOLE_MODE(0x0001))
                .with_context(|| "Could not enable ANSI escapes.")?;
        }
    }

    let args = Args::parse();

    let mut media = Media::new(args)?;

    // media.transform();
    media.render()?;

    Ok(())
}
