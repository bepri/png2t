use std::{
    fs,
    io::Write,
    path::PathBuf,
    process::{Command, Stdio},
};

use image::{
    imageops::{flip_horizontal_in_place, flip_vertical_in_place, resize, FilterType::Nearest},
    ImageBuffer, Rgba,
};
use itertools::Itertools;
use lazy_static::lazy_static;
use regex::Regex;

use crate::Args;

pub type Image = ImageBuffer<Rgba<u8>, Vec<u8>>;

#[derive(Debug)]
pub struct Media<'args> {
    frames: Vec<Image>,
    config: &'args Args,
    storage: PathBuf,
}

impl<'args> Media<'args> {
    pub fn new(config: &'args Args) -> Result<Self, String> {
        let storage = Self::get_tmp_dir();

        if !storage.exists() {
            if let Err(e) = fs::create_dir(&storage) {
                return Err(format!(
                    "Unable to create output directory at {}: {}",
                    storage.display(),
                    e
                ));
            }
        }

        Ok(Media::<'args> {
            frames: Vec::default(),
            config,
            storage,
        })
    }

    pub fn generate_frames(&self) {
        Command::new("ffmpeg")
            .args([
                "-hide_banner",
                "-i",
                &self.config.file,
                self.storage.join("frame%d.png").to_str().unwrap(),
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap()
            .wait()
            .unwrap();
    }

    pub fn load_frames(&mut self) -> Result<(), String> {
        let frames: Vec<PathBuf> = fs::read_dir(&self.storage)
            .unwrap()
            .map(|r| String::from(r.unwrap().path().to_str().unwrap()))
            .sorted_by(|a, b| human_sort::compare(a, b))
            .map(PathBuf::from)
            .collect();

        for (idx, frame) in frames.iter().enumerate() {
            let reader = image::io::Reader::open(frame);
            if let Err(e) = reader {
                return Err(format!(
                    "Unable to read from temp directory {}: {}",
                    self.storage.display(),
                    e
                ));
            }

            let decoder = reader.unwrap().decode();
            if let Err(e) = decoder {
                return Err(format!(
                    "Unable to decode {}: {}",
                    if self.frames.len() == 1 {
                        self.config.file.clone()
                    } else {
                        format!("frame {}/{} of {}", idx, self.frames.len(), self.config.file)
                    },
                    e
                ));
            }

            self.frames.push(decoder.unwrap().into_rgba8());
        }

        Ok(())
    }

    pub fn transform(&mut self) -> Result<(), String> {
        let (mut nwidth, mut nheight) = self.frames[0].dimensions();

        if let Some(s) = &self.config.size {
            let coords: Vec<u32> = s.split('x').map(|c| str::parse(c).unwrap_or(0)).collect();
            if coords.contains(&0) {
                return Err(String::from(
                    "Invalid coordinates supplied to --size tag: must be in format NUMxNUM",
                ));
            }

            nwidth = coords[0];
            nheight = coords[1];
        } else if !self.config.preserve_dims {
            (nwidth, nheight) = match nwidth > nheight {
                true => (64, (64f64 * (nheight as f64 / nwidth as f64)) as u32),
                false => ((64f64 * (nwidth as f64 / nheight as f64)) as u32, 64),
            };
        }

        if let Some(scale) = self.config.scale {
            nwidth = (nwidth as f32 * scale) as u32;
            nheight = (nheight as f32 * scale) as u32;
        }

        for frame in &mut self.frames {
            *frame = resize(frame, nwidth, nheight, Nearest);

            for pixel in frame.chunks_exact_mut(4) {
                if self.config.invert {
                    pixel[0] = u8::MAX - pixel[0];
                    pixel[1] = u8::MAX - pixel[1];
                    pixel[2] = u8::MAX - pixel[2];
                }
            }

            if self.config.flip_h {
                flip_horizontal_in_place(frame)
            }

            if self.config.flip_v {
                flip_vertical_in_place(frame)
            }
        }

        Ok(())
    }

    pub fn render(&self) -> Result<(), String> {
        let (w, h) = self.frames[0].dimensions();
        if self.frames.len() == 1 {
            self.display_frame(&self.frames[0], w, h)?;
        } else {
            // we have a video, so let's determine that framerate
            lazy_static! {
                static ref RE: Regex = Regex::new(r#"(\d*\.?\d*) fps"#).unwrap();
            }

            let fps: f32;
            if let Some(m) = RE
                .captures(
                    &String::from_utf8(
                        Command::new("ffprobe")
                            .args(["-hide_banner", "-i", &self.config.file])
                            .output()
                            .unwrap()
                            .stderr,
                    )
                    .unwrap(),
                )
                .unwrap()
                .get(1)
            {
                fps = str::parse(m.as_str()).unwrap();
            } else {
                return Err(String::from("Could not determine framerate of video!"));
            }

            let delay = std::time::Duration::from_millis((1000.0 / fps) as u64);

            while {
                for frame in &self.frames {
                    self.display_frame(frame, w, h)?;
                    std::thread::sleep(delay);
    
                    for _ in 0..h/2 {
                        print!("\x1b[1A\x1b[2K");
                    }
                }
                self.config.loop_video
            } {}

        }
        Ok(())
    }

    fn display_frame(&self, frame: &Image, w: u32, h: u32) -> Result<(), String> {
        let (mut x, mut y) = (0u32, 0u32);
        for _ in 0..(h / 2) * w {
            let upper = frame.get_pixel(x, y);
            let lower = frame.get_pixel(x, y + 1);

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

            if let Err(e) = std::io::stdout().flush() {
                return Err(format!("\nFailed to print image at ({}, {}): {}", x, y, e));
            }

            if x == w - 1 {
                x = 0;
                y += 2;
                println!();
            } else {
                x += 1;
            }
        }

        Ok(())
    }

    fn get_tmp_dir() -> PathBuf {
        let mut res = std::env::current_exe().unwrap();
        res.pop();
        res.push("TEMP");
        res
    }
}

impl<'args> Drop for Media<'args> {
    fn drop(&mut self) {
        if let Err(e) = fs::remove_dir_all(&self.storage) {
            panic!(
                "Failed to clean temp directory {}: {}",
                self.storage.display(),
                e
            );
        }
    }
}
