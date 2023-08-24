use std::process::{Command, Stdio};

use clap::Parser;

mod helpers;
use crate::helpers::*;

#[derive(Parser, Debug)]
#[command(
    name = "png2t",
    author = "imani@bepri.dev",
    version = "0.1.2",
    about = "A command-line tool to render a PNG to the terminal."
)]
pub struct Args {
    #[arg(help = "Path to a media file to render.", name = "FILE")]
    file: String,

    #[arg(help = "Invert all color", long)]
    invert: bool,

    #[arg(help = "Flip image horizontally", long)]
    flip_h: bool,

    #[arg(help = "Flip image vertically", long)]
    flip_v: bool,

    #[arg(help = "Dimensions to adjust to, in the format NxN", long)]
    size: Option<String>,

    #[arg(help = "Factor to scale by", long)]
    scale: Option<f32>,

    #[arg(help = "Avoid automatically resizing the image", long)]
    preserve_dims: bool,

    #[arg(long, id = "loop")]
    loop_video: bool,

    #[arg(help = "Mute audio if any is present", long)]
    mute: bool,
}

fn main() -> Result<(), String> {
    if let Err(e) = Command::new("ffmpeg").stdout(Stdio::null()).stderr(Stdio::null()).spawn() {
        if let std::io::ErrorKind::NotFound = e.kind() {
            return Err(String::from(
                "Could not find ffmpeg! Please install first or ensure it is on your PATH.",
            ));
        }
    }

    #[cfg(target_os="windows")]
    {
        println!("Warning: This program is capable of running on Windows, but it faces a lot of difficulties due to default Windows behavior.");
        println!("The main issue is that video playback is likely going to be extremely slow. This is not a performance issue - Windows' printing API is just extremely slow.");

        use windows::Win32::System::Console::*;
        
        unsafe {
            let handle = match GetStdHandle(STD_HANDLE(11)) {
                Ok(h) => h,
                Err(e) => {
                    return Err(format!("Could not communicate with output device: {e}"));
                },
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
