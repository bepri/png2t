//! CLI parsing

use anyhow::{Context, Result};

pub use clap::Parser;
use clap::{crate_name, crate_version};

#[derive(Clone, Debug)]
pub struct Dimensions {
    pub width: u32,
    pub height: u32,
}

impl Dimensions {
    pub fn parse_arg(arg: &str) -> Result<Self> {
        const COORD_ERROR: &str =
            "Invalid coordinates supplied to `--size`: must be in format NUMxNUM";

        let (w, h) = arg.split_once('x').context(COORD_ERROR)?;
        let convert_dim = |dim: &str| -> Result<u32> { str::parse(dim).context(COORD_ERROR) };

        Ok(Self {
            width: convert_dim(w)?,
            height: convert_dim(h)?,
        })
    }
}

#[derive(Parser, Debug)]
#[command(
    name = crate_name!(),
    version = crate_version!(),
    about = "A command-line tool to render a PNG to the terminal."
)]
pub struct Args {
    #[arg(help = "Path to a media file to render.", name = "FILE")]
    pub file: String,

    #[arg(help = "Invert all color", long)]
    pub invert: bool,

    #[arg(help = "Flip image horizontally", long)]
    pub flip_h: bool,

    #[arg(help = "Flip image vertically", long)]
    pub flip_v: bool,

    #[arg(help = "Dimensions to adjust to, in the format NxN", long, value_parser=Dimensions::parse_arg)]
    pub size: Option<Dimensions>,

    #[arg(help = "Factor to scale by", long)]
    pub scale: Option<f32>,

    #[arg(help = "Avoid automatically resizing the image", long)]
    pub preserve_dims: bool,

    #[arg(long, id = "loop")]
    pub loop_video: bool,

    #[arg(help = "Mute audio if any is present", long)]
    pub mute: bool,
}
