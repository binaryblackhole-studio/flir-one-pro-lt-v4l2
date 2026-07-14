# FLIR One Pro LT / Gen 3 V4L2 Driver

> [!WARNING]
> This repository is vibe coded.

This directory contains a C-based Linux user-space driver for the **FLIR One Pro LT** and **FLIR One Gen 3** thermal cameras. It captures thermal and visual frames over USB via `libusb` and feeds them into `v4l2loopback` devices so they can be read by standard Linux video tools.

Unlike the FLIR One Pro (which has a 160x120 thermal resolution), the FLIR One Pro LT and FLIR One Gen 3 have a native thermal resolution of **80x60**. This driver is configured to process and display the 80x60 thermal resolution format.

## Requirements

You must install the following development packages and the kernel module for virtual loopback devices:

```bash
# Debian/Ubuntu dependencies
sudo apt-get update
sudo apt-get install -y libusb-1.0-0-dev libjpeg-dev pkg-config v4l2loopback-dkms
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
Start the driver by passing a color palette raw file. Several palettes are provided in the `palettes/` directory (e.g., Rainbow, Iron2, Grayscale):

```bash
sudo ./flirone palettes/Rainbow.raw
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
- [src/flirone.c](file:///home/rafael/BinaryBlackhole/flir-one-pro-lt-v4l2/src/flirone.c): Main driver logic, handles libusb configuration, bulk transfers, thermal RAW conversion, text box drawing, and v4l2 loopback output.
- [src/font5x7.h](file:///home/rafael/BinaryBlackhole/flir-one-pro-lt-v4l2/src/font5x7.h): ASCII font map for printing temperature text overlays on the frame.
- [src/plank.h](file:///home/rafael/BinaryBlackhole/flir-one-pro-lt-v4l2/src/plank.h): Constants and formulas used to convert raw infrared values into temperature (Celsius).
- [palettes/](file:///home/rafael/BinaryBlackhole/flir-one-pro-lt-v4l2/palettes): Contains raw palette files for mapping temperature ranges to distinct colors.
- [scripts/](file:///home/rafael/BinaryBlackhole/flir-one-pro-lt-v4l2/scripts): Example helper scripts to load loopback devices or launch gstreamer pipelines.
