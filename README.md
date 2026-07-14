# FLIR One Pro LT / Gen 3 V4L2 Driver

> [!WARNING]
> This repository is vibe coded.

This directory contains a Rust-based Linux user-space driver for the **FLIR One Pro LT** and **FLIR One Gen 3** thermal cameras. It captures thermal and visual frames over USB via `libusb` (wrapped safely in `rusb`) and feeds them into `v4l2loopback` devices so they can be read by standard Linux video tools.

Unlike the FLIR One Pro (which has a 160x120 thermal resolution), the FLIR One Pro LT and FLIR One Gen 3 have a native thermal resolution of **80x60**. This driver is configured to process and display the 80x60 thermal resolution format.

## Requirements

You must install the following development packages, Rust toolchain, and the kernel module for virtual loopback devices:

```bash
# Debian/Ubuntu dependencies
sudo apt-get update
sudo apt-get install -y libusb-1.0-0-dev libjpeg-dev pkg-config v4l2loopback-dkms cargo
```

## Compilation

Build the driver using `make`:

```bash
make
```

To clean build artifacts:

```bash
make clean
```

## Configuration & Usage

### 1. Load the v4l2loopback Module
The driver expects specific virtual video devices (usually `/dev/video1` for visual MJPEG stream, and `/dev/video3` for the colorized RGB thermal stream). You can load the `v4l2loopback` module with the required settings:

```bash
sudo modprobe v4l2loopback exclusive_caps=1,1,1 video_nr=1,2,3
```

*Note: Depending on your system's existing cameras, you may want to customize the `video_nr` devices.*

### 2. Run the Driver
Start the driver by passing a color palette raw file using the `-p` or `--palette` flag:

```bash
sudo ./flirone --palette palettes/Rainbow.raw
```

If you have other video devices active on your system, you can pass custom device paths as named arguments:

```bash
# Using long options
sudo ./flirone --palette palettes/Rainbow.raw --visual-device /dev/video4 --thermal-device /dev/video5

# Or short options
sudo ./flirone -p palettes/Rainbow.raw -v /dev/video4 -t /dev/video5
```
*(Running with `sudo` or having proper udev rules configured is required to allow `libusb` access to the USB device).*

### 3. View the Stream
You can view the colorized thermal stream on `/dev/video3` using standard tools such as `ffplay` or `gstreamer`:

```bash
# Using ffplay to view the thermal camera stream
ffplay /dev/video3

# Or using GStreamer to view the visual camera stream
gst-launch-1.0 v4l2src device=/dev/video2 ! decodebin ! autovideosink
```

## Project Structure
- [src/main.rs](src/main.rs): Complete safe Rust driver implementation, covering device scanning, bulk data processing, text boxes, and V4L2 raw writes.
- [Cargo.toml](Cargo.toml): Declares Rust project configuration and library dependencies (`rusb`, `nix`, `libc`, `chrono`).
- [palettes/](palettes): Contains raw palette files for mapping temperature ranges to distinct colors.
- [scripts/](scripts): Example helper scripts to load loopback devices or launch gstreamer pipelines.

## Credits
This driver is based on the reverse-engineered Linux driver implementation from **[fnoop/flirone-v4l2](https://github.com/fnoop/flirone-v4l2)**.
