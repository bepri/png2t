use std::{
    fs::{self, File},
    io::{BufReader, Write},
    path::PathBuf,
    process::{Command, Stdio},
    time::Duration,
};

use crossterm::{
    cursor::{position, MoveDown, MoveTo, MoveToColumn, MoveUp},
    event::{poll, read, Event, KeyCode, KeyEvent, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use image::{
    imageops::{flip_horizontal_in_place, flip_vertical_in_place, resize, FilterType::Nearest},
    ImageBuffer, Rgba,
};
use itertools::Itertools;
use lazy_static::lazy_static;
use regex::Regex;
use rodio::{OutputStream, OutputStreamHandle};

use crate::Args;

pub type Image = ImageBuffer<Rgba<u8>, Vec<u8>>;

/// A wrapper for a media file.
///
/// This struct can represent a video of any length and stores it internally.
/// An external temporary directory is used to store media when creating an instance.
/// The `Drop` trait is implemented to clear this temp directory.
#[derive(Debug)]
pub struct Media<'args> {
    frames: Vec<Image>,
    config: &'args Args,
    storage: PathBuf,
    is_video: bool,
    has_audio: bool,
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
            is_video: false,
            has_audio: false,
        })
    }

    /// Unpacks the file specified in `self.config.file`
    ///
    /// This function takes every available frame from a media file and stores it as individual .pngs for display.
    /// It will also create a .mp3 with the associated audio if available.
    /// Storage location is whatever is returned by `Self::get_tmp_dir()`
    ///
    /// # Errors
    /// Generally the only failure possible at this point is ffmpeg not being installed, which will return an OS error 2.
    pub fn unpack_file(&mut self) -> Result<(), String> {
        // Separate out the individual frames
        Command::new("ffmpeg")
            .args([
                "-hide_banner",
                "-i",
                &self.config.file,
                self.storage.join("frame%d.exr").to_str().unwrap(),
                "-preset",
                "ultrafast",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap()
            .wait()
            .unwrap();

        // Pull out audio stream if present.
        self.has_audio = !self.config.mute && // If mute is set, ignore audio and set to false.
            Command::new("ffmpeg")
                .args([
                    "-hide_banner",
                    "-i",
                    &self.config.file,
                    self.storage.join("audio.mp3").to_str().unwrap(),
                    "-preset",
                    "ultrafast",
                ])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
                .unwrap()
                .wait()
                .unwrap()
                .success(); // Return whether or not the command succeeded.

        self.load_frames()
    }

    /// Read from `self.storage` and store every image in there in RGBA8 format into `self.frames`
    ///
    /// # Errors
    /// Can either fail to access the temporary storage directory or individual files, or encounter an invalid PNG.
    /// These issues are unlikely but could be caused by a race condition with another program modifying `self.storage` during execution.
    fn load_frames(&mut self) -> Result<(), String> {
        // Objective: get a list of all files in a directory in human-sorted order
        let frames: Vec<PathBuf> = fs::read_dir(&self.storage) // gets all files in `&self.storage`
            .unwrap()
            .map(|r| String::from(r.unwrap().path().to_str().unwrap())) // Unwrap ReadDir into a DirEntry, which is still not a sortable plain string. Thus, pull the `path()` from it, then cast it to a string, then wrap it in `String::from()` for ownership reasons
            .sorted_by(|a, b| human_sort::compare(a, b)) // Apply human-sort
            .map(PathBuf::from) // Cast the list of sorted strings into Path objects instead
            .filter(|p| p.extension().unwrap() == "exr")
            .collect(); // Collect into the final vector

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
                        format!(
                            "frame {}/{} of {}",
                            idx,
                            self.frames.len(),
                            self.config.file
                        )
                    },
                    e
                ));
            }

            // Parse file into RGBA8 format and push it into `self.frames`
            self.frames.push(decoder.unwrap().into_rgba8());
        }

        self.is_video = self.frames.len() > 1;

        Ok(())
    }

    /// Transform each frame based on command line flags
    ///
    /// Pulls all information from `self.config`.
    /// This function has potential to be the slowest in the rendering process if done with too many flags - be careful in here
    pub fn transform(&mut self) -> Result<(), String> {
        let (mut nwidth, mut nheight) = self.frames[0].dimensions();

        // The following block calculates the final image size. Multiple factors influence it so it's best to calculate it once.
        // This means we can't support dynamically resizing .mp4s and such, but I think that's okay... (sorry Discord trolls)
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
            // Set the longest side to be 64px, with the shorter side scaling down proportionally to preserve aspect ratio
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

    /// Plays the media file in the terminal. Must be initialized with `self.load_frames()` first.
    ///
    /// # Errors
    /// Can error out if `self` contains a video but the FPS cannot be determined.
    /// Also may fail on I/O or sound device errors.
    /// Can possibly fail on file I/O, but is only possible by race condition with another program modifying the storage directory.
    pub fn render(&self) -> Result<(), String> {
        // Create buffer space in the terminal for the image before printing
        let h = self.frames[0].dimensions().1 / 2;
        for _ in 0..h {
            println!();
        }

        // Turn off the fancy stuff in the terminal. I'm using this to later emulate C's `getchar`
        enable_raw_mode().unwrap();

        // Reset cursor to where the top-left pixel should print
        print!("{}{}", MoveToColumn(0), MoveUp(h as u16));

        // Save this location for quicker cursor resets when new frames are printed
        let pos = position().unwrap();

        // The code to play a video is a lot more complex, so it's not worthwhile to try to generalize this for photos vs. videos
        if self.is_video {
            // Following block uses regex to extract the video's fps from the output of `ffprobe`
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

            // Based on the fps, calculate how long to wait between each frame printing
            let delay = std::time::Duration::from_millis((1000.0 / fps) as u64);

            // Rust's deallocation methods kill the audio if it is in a separate block from the video rendering.
            // This means it won't be able to play if we slim down on repeated code by only using this if/else tree to spawn the audio when true.
            // This is my least favorite piece of code
            loop {
                // Spawn the audio and keep it from deallocating with `let`
                let res = if self.has_audio {
                    let _audio = self.spawn_audio();
                    self.play_video(delay, pos)
                } else {
                    self.play_video(delay, pos)
                };

                // Keep playing if true, otherwise the user requested an early exit (or loop_video == false)
                match res? {
                    true => continue,
                    false => break,
                };
            }
        } else {
            // If we just have an image, we simply gotta display it
            self.display_frame(&self.frames[0])?;
        }

        disable_raw_mode().unwrap();
        Ok(())
    }

    /// Interal function to display one image into the terminal.
    ///
    /// # Errors
    /// I/O errors can occur when flushing `stdout`
    fn display_frame(&self, frame: &Image) -> Result<(), String> {
        let (w, h) = frame.dimensions();
        let (mut x, mut y) = (0u32, 0u32);
        for _ in 0..(h / 2) * w {
            let upper = frame.get_pixel(x, y);
            let lower = frame.get_pixel(x, y + 1);

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

            if let Err(e) = std::io::stdout().flush() {
                return Err(format!("\nFailed to print image at ({}, {}): {}", x, y, e));
            }

            // Arithmetic to keep cursor in the right position to print
            if x == w - 1 {
                x = 0;
                y += 2;
                print!("{}{}", MoveDown(1), MoveToColumn(0));
            } else {
                x += 1;
            }
        }

        Ok(())
    }

    /// Plays a video stored in `self.frames`
    ///
    /// # Returns
    /// `Ok(bool)` will be true if the video should continue playing.
    /// This is only with regards to whether or not the user has attempted to "quit" the program, and does not concern the loop_video option.
    ///
    /// # Errors
    /// Can fail on I/O from `self.display_frame()`
    fn play_video(&self, delay: Duration, pos: (u16, u16)) -> Result<bool, String> {
        for frame in &self.frames {
            self.display_frame(frame)?;
            std::thread::sleep(delay); // Pause between frames to preserve framerate

            if poll(Duration::from_millis(1)).unwrap() {
                let event = read().unwrap();
                if [
                    Event::Key(KeyCode::Char('q').into()),
                    Event::Key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL)),
                ]
                .contains(&event)
                {
                    return Ok(false);
                }
            }

            // Reset cursor for next frame and overwrite old frame
            print!("{}", MoveTo(pos.0, pos.1));
        }

        Ok(self.config.loop_video)
    }

    /// Creates an audio thread to play sound exactly once.
    ///
    /// Pulls audio from `%self.storage%/audio.mp3` and returns a handle on the audio.
    fn spawn_audio(&self) -> (OutputStream, OutputStreamHandle) {
        use rodio::{source::Source, Decoder};

        // Open up audio handles and bind them to avoid deallocation. This line may panic if there is no audio device present.
        let (_stream, stream_handle) = OutputStream::try_default().unwrap();

        // Open up the audio file. Unwraps should never panic unless a race condition is created
        let source = Decoder::new(BufReader::new(
            File::open(self.storage.join("audio.mp3")).unwrap(),
        ));

        // Play!
        stream_handle
            .play_raw(source.unwrap().convert_samples())
            .unwrap();

        // This return ensures these are not deallocated, which would kill the audio thread.
        (_stream, stream_handle)
    }

    /// Generate a path to a temporary directory
    ///
    /// Does not create the directory. This mostly exists as an easy location to modify the temporary storage solution later if needed in later versions of this.
    fn get_tmp_dir() -> PathBuf {
        let mut res = std::env::current_exe().unwrap();
        res.pop();
        res.push("TEMP");
        res
    }
}

impl<'a> Drop for Media<'a> {
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
