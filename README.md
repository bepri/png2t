<div align="center">
  <h1>🖼️ <b>png2t</b> 🎬</h1>
  <p>
    <strong>Who needs a GUI anyways?</strong>
  </p>
</div>

## Examples
TODO lol

## What?
png2t is a fun little program that allows you to print out image or play a video format in a shell, so long as the shell supports ANSI escape codes!

## How?
png2t was written entirely in Rust! It currently depends on FFMPEG being installed to the system, but later versions of this will hopefully use an internal image library rather than external shell calls. png2t decomposes videos into .exr image files of each of their frames, loads them into memory, then prints them as individual RGB pixels using ANSI Truecolor sequences. It even plays the sound!

## Why?
It's cool!

# Building
## Unix
### 1. Install `cargo` to your system
```
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

### 2. Install `ffmpeg`
Debian-based:
```
sudo apt install -y ffmpeg
```
Fedora-based:
```
sudo dnf install -y ffmpeg
```

### 3. Clone this repository
```
git clone git@github.com:bepri/png2t.git && cd png2t
```

### 4. Build
```
cargo build --release
```
Resulting binary will be at `target/release/png2t`

## Windows
### 1. Install `cargo` on your system:
- [Go here](https://rustup.rs) and download `rustup-init.exe`
- Run & follow prompts
- Install [Visual Studio or the Visual C++ Build Tools](https://visualstudio.microsoft.com/downloads/), being sure to check the boxes for "C++ Tools" and "Windows 10 SDK"

### 2. Install FFMPEG for Windows
Downloads can be found at [this link](https://ffmpeg.org/download.html).

### 3. Clone this repository
```
git clone git@github.com:bepri/png2t.git && cd png2t
```

### 4. Build
```
cargo build --release
```
Resulting binary will be at `target/release/png2t.exe`
