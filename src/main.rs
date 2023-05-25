use clap::Parser;

mod helpers;
use crate::helpers::*;

#[derive(Parser, Debug)]
#[command(
    name = "png2t",
    author = "imani@bepri.dev",
    version = "0.1.1",
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
