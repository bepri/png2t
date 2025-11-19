use std::process::{Command, Stdio};

mod cli;
mod helpers;

use crate::cli::{Args, Parser};
use crate::helpers::*;

fn main() -> Result<(), String> {
    if let Err(e) = Command::new("ffmpeg")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        if let std::io::ErrorKind::NotFound = e.kind() {
            return Err(String::from(
                "Could not find ffmpeg! Please install first or ensure it is on your PATH.",
            ));
        }
    }

    #[cfg(target_os = "windows")]
    {
        println!("Warning: This program is capable of running on Windows, but it faces a lot of difficulties due to default Windows behavior.");
        println!("The main issue is that video playback is likely going to be extremely slow. This is not a performance issue - Windows' printing API is just extremely slow.");

        use windows::Win32::System::Console::*;

        unsafe {
            let handle = match GetStdHandle(STD_HANDLE(11)) {
                Ok(h) => h,
                Err(e) => {
                    return Err(format!("Could not communicate with output device: {e}"));
                }
            };
            if let Err(e) = SetConsoleMode(handle, CONSOLE_MODE(0x0001)) {
                return Err(format!("Could not enable ANSI escapes: {e}"));
            }
        }
    }

    let args = Args::parse();

    let mut media = match Media::new(&args) {
        Err(e) => return Err(format!("Couldn't load file {}: {}", args.file, e)),
        Ok(m) => m,
    };

    media.unpack_file()?;
    media.transform()?;
    media.render()?;

    Ok(())
}
