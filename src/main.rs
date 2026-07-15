use clap::Parser;
use rusb::{Context, UsbContext};
use std::fs::File;
use std::io::Write;
use std::os::unix::io::AsRawFd;
use std::time::Duration;

// --- FLIR One G2/Gen3 Constants ---
const VENDOR_ID: u16 = 0x09cb;
const PRODUCT_ID: u16 = 0x1996;

// GEN3 80x60 Resolution Config
const FRAME_WIDTH0: usize = 80;
const FRAME_HEIGHT0: usize = 60;
const TEXTBOX_HEIGHT: usize = 16;

const FRAME_WIDTH1: usize = 640;
const FRAME_HEIGHT1: usize = 480;

const FRAME_WIDTH2: usize = FRAME_WIDTH0;
const FRAME_HEIGHT2: usize = FRAME_HEIGHT0 + TEXTBOX_HEIGHT;

const LINE_STRIDE: usize = FRAME_WIDTH0 * 164 / 160; // 82
const LINE_OFFSET: usize = 32;

const BUF85SIZE: usize = 1048576; // 1MB buffer

// --- Planck Calibration Constants ---
const PLANCK_R1: f64 = 16528.178;
const PLANCK_B: f64 = 1427.5;
const PLANCK_F: f64 = 1.0;
const PLANCK_O: f64 = -1307.0;
const PLANCK_R2: f64 = 0.012258549;

const TEMP_REFLECTED: f64 = 20.0; // [°C]
const EMISSIVITY: f64 = 0.95;

fn raw2temperature(raw: f64) -> f64 {
    let raw = raw * 4.0;
    let raw_refl = PLANCK_R1
        / (PLANCK_R2 * (((PLANCK_B / (TEMP_REFLECTED + 273.15)).exp()) - PLANCK_F))
        - PLANCK_O;
    let raw_obj = (raw - (1.0 - EMISSIVITY) * raw_refl) / EMISSIVITY;
    PLANCK_B / (PLANCK_R1 / (PLANCK_R2 * (raw_obj + PLANCK_O)) + PLANCK_F).ln() - 273.15
}

// --- Font Definition (5x7) ---
const MAX_CHARS: usize = 96;
const CHAR_OFFSET: usize = 0x20;

const FONT5X7_BASIC: [[u8; 5]; MAX_CHARS] = [
    [0x00, 0x00, 0x00, 0x00, 0x00], // (space)
    [0x00, 0x00, 0x5F, 0x00, 0x00], // !
    [0x00, 0x07, 0x00, 0x07, 0x00], // "
    [0x14, 0x7F, 0x14, 0x7F, 0x14], // #
    [0x24, 0x2A, 0x7F, 0x2A, 0x12], // $
    [0x23, 0x13, 0x08, 0x64, 0x62], // %
    [0x36, 0x49, 0x55, 0x22, 0x50], // &
    [0x00, 0x05, 0x03, 0x00, 0x00], // '
    [0x00, 0x1C, 0x22, 0x41, 0x00], // (
    [0x00, 0x41, 0x22, 0x1C, 0x00], // )
    [0x08, 0x2A, 0x1C, 0x2A, 0x08], // *
    [0x08, 0x08, 0x63, 0x08, 0x08], // +
    [0x00, 0x50, 0x30, 0x00, 0x00], // ,
    [0x08, 0x08, 0x08, 0x08, 0x08], // -
    [0x00, 0x60, 0x60, 0x00, 0x00], // .
    [0x20, 0x10, 0x08, 0x04, 0x02], // /
    [0x3E, 0x51, 0x49, 0x45, 0x3E], // 0
    [0x00, 0x42, 0x7F, 0x40, 0x00], // 1
    [0x42, 0x61, 0x51, 0x49, 0x46], // 2
    [0x21, 0x41, 0x45, 0x4B, 0x31], // 3
    [0x18, 0x14, 0x12, 0x7F, 0x10], // 4
    [0x27, 0x45, 0x45, 0x45, 0x39], // 5
    [0x3C, 0x4A, 0x49, 0x49, 0x30], // 6
    [0x01, 0x71, 0x09, 0x05, 0x03], // 7
    [0x36, 0x49, 0x49, 0x49, 0x36], // 8
    [0x06, 0x49, 0x49, 0x29, 0x1E], // 9
    [0x00, 0x36, 0x36, 0x00, 0x00], // :
    [0x00, 0x56, 0x36, 0x00, 0x00], // ;
    [0x00, 0x08, 0x14, 0x22, 0x41], // <
    [0x14, 0x14, 0x14, 0x14, 0x14], // =
    [0x41, 0x22, 0x14, 0x08, 0x00], // >
    [0x02, 0x01, 0x51, 0x09, 0x06], // ?
    [0x32, 0x49, 0x79, 0x41, 0x3E], // @
    [0x7E, 0x11, 0x11, 0x11, 0x7E], // A
    [0x7F, 0x49, 0x49, 0x49, 0x36], // B
    [0x3E, 0x41, 0x41, 0x41, 0x22], // C
    [0x7F, 0x41, 0x41, 0x22, 0x1C], // D
    [0x7F, 0x49, 0x49, 0x49, 0x41], // E
    [0x7F, 0x09, 0x09, 0x01, 0x01], // F
    [0x3E, 0x41, 0x41, 0x51, 0x32], // G
    [0x7F, 0x08, 0x08, 0x08, 0x7F], // H
    [0x00, 0x41, 0x7F, 0x41, 0x00], // I
    [0x20, 0x40, 0x41, 0x3F, 0x01], // J
    [0x7F, 0x08, 0x14, 0x22, 0x41], // K
    [0x7F, 0x40, 0x40, 0x40, 0x40], // L
    [0x7F, 0x02, 0x04, 0x02, 0x7F], // M
    [0x7F, 0x04, 0x08, 0x10, 0x7F], // N
    [0x3E, 0x41, 0x41, 0x41, 0x3E], // O
    [0x7F, 0x09, 0x09, 0x09, 0x06], // P
    [0x3E, 0x51, 0x49, 0x45, 0x3E], // Q
    [0x7F, 0x09, 0x19, 0x29, 0x46], // R
    [0x46, 0x49, 0x49, 0x49, 0x31], // S
    [0x01, 0x01, 0x7F, 0x01, 0x01], // T
    [0x3F, 0x40, 0x40, 0x40, 0x3F], // U
    [0x1F, 0x20, 0x40, 0x20, 0x1F], // V
    [0x7F, 0x20, 0x18, 0x20, 0x7F], // W
    [0x63, 0x14, 0x08, 0x14, 0x63], // X
    [0x03, 0x04, 0x78, 0x04, 0x03], // Y
    [0x61, 0x51, 0x49, 0x45, 0x43], // Z
    [0x00, 0x00, 0x7F, 0x41, 0x41], // [
    [0x02, 0x04, 0x08, 0x10, 0x20], // "\"
    [0x41, 0x41, 0x7F, 0x00, 0x00], // ]
    [0x04, 0x02, 0x01, 0x02, 0x04], // ^
    [0x40, 0x40, 0x40, 0x40, 0x40], // _
    [0x00, 0x01, 0x02, 0x04, 0x00], // `
    [0x20, 0x54, 0x54, 0x54, 0x78], // a
    [0x7F, 0x48, 0x44, 0x44, 0x38], // b
    [0x38, 0x44, 0x44, 0x44, 0x20], // c
    [0x38, 0x44, 0x44, 0x48, 0x7F], // d
    [0x38, 0x54, 0x54, 0x54, 0x18], // e
    [0x08, 0x7E, 0x09, 0x01, 0x02], // f
    [0x08, 0x14, 0x54, 0x54, 0x3C], // g
    [0x7F, 0x08, 0x04, 0x04, 0x78], // h
    [0x00, 0x44, 0x7D, 0x40, 0x00], // i
    [0x20, 0x40, 0x44, 0x3D, 0x00], // j
    [0x00, 0x7F, 0x10, 0x28, 0x44], // k
    [0x00, 0x41, 0x7F, 0x40, 0x00], // l
    [0x7C, 0x04, 0x18, 0x04, 0x78], // m
    [0x7C, 0x08, 0x04, 0x04, 0x78], // n
    [0x38, 0x44, 0x44, 0x44, 0x38], // o
    [0x7C, 0x14, 0x14, 0x14, 0x08], // p
    [0x08, 0x14, 0x14, 0x18, 0x7C], // q
    [0x7C, 0x08, 0x04, 0x04, 0x08], // r
    [0x48, 0x54, 0x54, 0x54, 0x20], // s
    [0x04, 0x3F, 0x44, 0x40, 0x20], // t
    [0x3C, 0x40, 0x40, 0x20, 0x7C], // u
    [0x1C, 0x20, 0x40, 0x20, 0x1C], // v
    [0x3C, 0x40, 0x30, 0x40, 0x3C], // w
    [0x44, 0x28, 0x10, 0x28, 0x44], // x
    [0x0C, 0x50, 0x50, 0x50, 0x3C], // y
    [0x44, 0x64, 0x54, 0x4C, 0x44], // z
    [0x00, 0x08, 0x36, 0x41, 0x00], // {
    [0x00, 0x00, 0x7F, 0x00, 0x00], // |
    [0x00, 0x41, 0x36, 0x08, 0x00], // }
    [0x08, 0x08, 0x2A, 0x1C, 0x08], // ->
    [0x08, 0x1C, 0x2A, 0x08, 0x08], // <-
];

fn font_write(fb: &mut [u8], mut x: usize, y: usize, text: &str) {
    for &ch in text.as_bytes() {
        let char_idx = ((ch & 0x7f) as usize).saturating_sub(CHAR_OFFSET);
        if char_idx < MAX_CHARS {
            for (ry, &font_row) in FONT5X7_BASIC[char_idx].iter().enumerate() {
                for rx in 0..7 {
                    let v = (font_row >> rx) & 1;
                    if v != 0 {
                        let target_idx = (y + rx) * FRAME_WIDTH2 + (x + ry);
                        if target_idx < fb.len() {
                            fb[target_idx] = 0; // transparent black overlay
                        }
                    }
                }
            }
        }
        x += 6;
    }
}

// --- V4L2 Raw ioctl setup ---
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct v4l2_capability {
    pub driver: [u8; 16],
    pub card: [u8; 32],
    pub bus_info: [u8; 32],
    pub version: u32,
    pub capabilities: u32,
    pub device_caps: u32,
    pub reserved: [u32; 3],
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct v4l2_pix_format {
    pub width: u32,
    pub height: u32,
    pub pixelformat: u32,
    pub field: u32,
    pub bytesperline: u32,
    pub sizeimage: u32,
    pub colorspace: u32,
    pub priv_field: u32,
    pub flags: u32,
    pub enc: u32, // union: ycbcr_enc/hsv_enc
    pub quantization: u32,
    pub xfer_func: u32,
}

#[repr(C)]
pub union v4l2_format_union {
    pub pix: v4l2_pix_format,
    pub raw_data: [u8; 200],
    pub align: usize, // Forces pointer-sized alignment (8 bytes on 64-bit, 4 on 32-bit)
}

#[repr(C)]
pub struct v4l2_format {
    pub type_: u32,
    pub fmt: v4l2_format_union,
}

#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::size_of::<v4l2_format>() == 208);
#[cfg(target_pointer_width = "32")]
const _: () = assert!(std::mem::size_of::<v4l2_format>() == 204);

const _: () = assert!(std::mem::size_of::<v4l2_capability>() == 104);

const V4L2_BUF_TYPE_VIDEO_OUTPUT: u32 = 2;
const V4L2_FIELD_NONE: u32 = 1;
const V4L2_COLORSPACE_SRGB: u32 = 8;

nix::ioctl_readwrite!(vidioc_g_fmt, b'V', 4, v4l2_format);
nix::ioctl_readwrite!(vidioc_s_fmt, b'V', 5, v4l2_format);
nix::ioctl_read!(vidioc_querycap, b'V', 0, v4l2_capability);

fn setup_v4l2_device(
    file: &File,
    width: u32,
    height: u32,
    pixelformat: u32,
    sizeimage: u32,
    bytesperline: u32,
    device_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let fd = file.as_raw_fd();

    let mut caps: v4l2_capability = unsafe { std::mem::zeroed() };
    unsafe { vidioc_querycap(fd, &mut caps) }
        .map_err(|e| format!("VIDIOC_QUERYCAP failed on {}: {} (Is this a valid V4L2 device?)", device_path, e))?;

    let mut fmt: v4l2_format = unsafe { std::mem::zeroed() };
    fmt.type_ = V4L2_BUF_TYPE_VIDEO_OUTPUT;

    // Read current settings
    let _ = unsafe { vidioc_g_fmt(fd, &mut fmt) };

    fmt.type_ = V4L2_BUF_TYPE_VIDEO_OUTPUT;
    unsafe {
        let pix = &mut fmt.fmt.pix;
        pix.width = width;
        pix.height = height;
        pix.pixelformat = pixelformat;
        pix.sizeimage = sizeimage;
        pix.field = V4L2_FIELD_NONE;
        pix.bytesperline = bytesperline;
        pix.colorspace = V4L2_COLORSPACE_SRGB;
    }

    // Set new format
    unsafe { vidioc_s_fmt(fd, &mut fmt) }
        .map_err(|e| format!(
            "VIDIOC_S_FMT failed on {}: {} (This device may not support video OUTPUT. Are you sure this is a v4l2loopback device and not a physical webcam?)",
            device_path, e
        ))?;

    Ok(())
}

fn fourcc(a: u8, b: u8, c: u8, d: u8) -> u32 {
    (a as u32) | ((b as u32) << 8) | ((c as u32) << 16) | ((d as u32) << 24)
}

struct DriverState {
    buf85: Vec<u8>,
    buf85pointer: usize,
    colormap: Vec<u8>,
    ffc_state: i32,
    file_visual: File,
    file_thermal: File,
}

impl DriverState {
    fn process_usb_data(&mut self, buf: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
        let magicbyte = [0xef, 0xbe, 0x00, 0x00];

        // Reset buffer if new chunk starts with magic bytes or buffer limit exceeded
        if buf.starts_with(&magicbyte) || (self.buf85pointer + buf.len()) >= BUF85SIZE {
            self.buf85pointer = 0;
        }

        // Copy chunk to main buffer
        let copy_len = buf.len().min(BUF85SIZE - self.buf85pointer);
        self.buf85[self.buf85pointer..self.buf85pointer + copy_len]
            .copy_from_slice(&buf[..copy_len]);
        self.buf85pointer += copy_len;

        // Check if we have the correct magic byte at start
        if !self.buf85.starts_with(&magicbyte) {
            self.buf85pointer = 0;
            println!("Reset buffer because of bad Magic Byte!");
            return Ok(());
        }

        if self.buf85pointer < 28 {
            return Ok(()); // Wait for headers
        }

        // Read sizes
        let frame_size = u32::from_le_bytes(self.buf85[8..12].try_into().unwrap()) as usize;
        let thermal_size = u32::from_le_bytes(self.buf85[12..16].try_into().unwrap()) as usize;
        let jpg_size = u32::from_le_bytes(self.buf85[16..20].try_into().unwrap()) as usize;

        // If full frame hasn't arrived yet, wait for next transfer
        if (frame_size + 28) > self.buf85pointer {
            return Ok(());
        }

        // Reset pointer for next frame
        self.buf85pointer = 0;

        let mut pix = vec![0u16; FRAME_WIDTH0 * FRAME_HEIGHT0];
        let mut min = 0x10000;
        let mut max = 0;
        let mut maxx = 0;
        let mut maxy = 0;

        // Extract thermal pixel values
        for y in 0..FRAME_HEIGHT0 {
            for x in 0..FRAME_WIDTH0 {
                let offset = if x < 80 {
                    2 * (y * LINE_STRIDE + x) + LINE_OFFSET
                } else {
                    2 * (y * LINE_STRIDE + x) + LINE_OFFSET + 4
                };
                let v = (self.buf85[offset] as u16) | ((self.buf85[offset + 1] as u16) << 8);
                pix[y * FRAME_WIDTH0 + x] = v;

                if (v as i32) < min {
                    min = v as i32;
                }
                if (v as i32) > max {
                    max = v as i32;
                    maxx = x;
                    maxy = y;
                }
            }
        }

        // Normalize thermal values and apply contrast scaling
        let delta = if max == min { 1 } else { max - min } as u32;
        let scale = 0x10000 / delta;

        let mut fb_proc = vec![128u8; FRAME_WIDTH2 * FRAME_HEIGHT2];
        for y in 0..FRAME_HEIGHT0 {
            for x in 0..FRAME_WIDTH0 {
                let v = (((pix[y * FRAME_WIDTH0 + x] as i32 - min) as u32 * scale) >> 8) as u8;
                fb_proc[y * FRAME_WIDTH0 + x] = v;
            }
        }

        // Generate overlay timestamp and temperatures
        let now_str = chrono::Local::now().format("%H:%M:%S").to_string();
        let cy = FRAME_HEIGHT0 / 2;
        let cx = FRAME_WIDTH0 / 2;
        let med = (pix[(cy - 1) * FRAME_WIDTH0 + (cx - 1)] as u32
            + pix[(cy - 1) * FRAME_WIDTH0 + cx] as u32
            + pix[cy * FRAME_WIDTH0 + (cx - 1)] as u32
            + pix[cy * FRAME_WIDTH0 + cx] as u32)
            / 4;

        let temp_str = format!(
            "{} {:.1}/{:.1}/{:.1}'C",
            now_str,
            raw2temperature(min as f64),
            raw2temperature(med as f64),
            raw2temperature(max as f64)
        );

        // Split text across multiple lines for GEN3 resolution
        let max_chars = FRAME_WIDTH0 / 6;
        let (line1, line2) = if temp_str.len() > max_chars {
            let split_idx = max_chars.min(temp_str.len());
            (&temp_str[..split_idx], &temp_str[split_idx..])
        } else {
            (temp_str.as_str(), "")
        };

        font_write(&mut fb_proc, 1, FRAME_HEIGHT0, line1);
        if !line2.is_empty() {
            let line2_trimmed = &line2[..max_chars.min(line2.len())];
            font_write(&mut fb_proc, 1, FRAME_HEIGHT0 + 8, line2_trimmed);
        }

        // Draw crosshairs
        font_write(
            &mut fb_proc,
            FRAME_WIDTH0 / 2 - 2,
            FRAME_HEIGHT0 / 2 - 3,
            "+",
        );

        let mut maxx_adj = (maxx as isize - 4).max(0);
        let mut maxy_adj = (maxy as isize - 4).max(0);
        if maxx_adj > (FRAME_WIDTH0 as isize - 10) {
            maxx_adj = FRAME_WIDTH0 as isize - 10;
        }
        if maxy_adj > (FRAME_HEIGHT0 as isize - 10) {
            maxy_adj = FRAME_HEIGHT0 as isize - 10;
        }

        font_write(&mut fb_proc, FRAME_WIDTH0 - 6, maxy_adj as usize, "<");
        font_write(&mut fb_proc, maxx_adj as usize, FRAME_HEIGHT0 - 8, "|");

        // Colorize thermal frame buffer using selected palette
        let mut fb_proc2 = vec![0u8; FRAME_WIDTH2 * FRAME_HEIGHT2 * 3];
        for y in 0..FRAME_HEIGHT2 {
            for x in 0..FRAME_WIDTH2 {
                let v = fb_proc[y * FRAME_WIDTH2 + x] as usize;
                let out_idx = 3 * (y * FRAME_WIDTH2 + x);
                fb_proc2[out_idx] = self.colormap[3 * v];
                fb_proc2[out_idx + 1] = self.colormap[3 * v + 1];
                fb_proc2[out_idx + 2] = self.colormap[3 * v + 2];
            }
        }

        // Write MJPEG visual stream directly to fdwr1
        let visual_offset = 28 + thermal_size;
        if visual_offset + jpg_size <= self.buf85.len() {
            let visual_data = &self.buf85[visual_offset..visual_offset + jpg_size];
            let _ = self.file_visual.write_all(visual_data);
        }

        // Check for FFC frame status
        let ffc_offset = 28 + thermal_size + jpg_size + 17;
        let is_ffc =
            ffc_offset + 3 <= self.buf85.len() && &self.buf85[ffc_offset..ffc_offset + 3] == b"FFC";

        if is_ffc {
            self.ffc_state = 1;
        } else {
            if self.ffc_state == 1 {
                self.ffc_state = 0; // Skip first frame after FFC
            } else {
                // Write thermal colorized RGB stream directly to fdwr2
                let _ = self.file_thermal.write_all(&fb_proc2);
            }
        }

        Ok(())
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about = "FLIR One Pro LT / Gen 3 Linux V4L2 Loopback Driver", long_about = None)]
struct CliArgs {
    /// Path to the color palette raw file (e.g. palettes/Rainbow.raw)
    #[arg(short, long)]
    palette: String,

    /// Path to the visual V4L2 loopback device
    #[arg(short, long, default_value = "/dev/video2")]
    visual_device: String,

    /// Path to the thermal V4L2 loopback device
    #[arg(short, long, default_value = "/dev/video3")]
    thermal_device: String,
}

#[allow(unreachable_code)]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = CliArgs::parse();

    let palette_path = &args.palette;
    let video_device1 = &args.visual_device;
    let video_device2 = &args.thermal_device;

    // Read colormap palette file
    let colormap = std::fs::read(palette_path)
        .map_err(|e| format!("Error opening palette file {}: {}", palette_path, e))?;
    if colormap.len() < 768 {
        return Err(format!(
            "Palette file {} is too short (must be at least 768 bytes)",
            palette_path
        )
        .into());
    }

    // Open output virtual loopback devices
    println!("Opening loopback visual device: {}", video_device1);
    let file_visual = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(video_device1)
        .map_err(|e| {
            format!(
                "Failed to open loopback visual device {}: {}",
                video_device1, e
            )
        })?;

    println!("Opening loopback thermal device: {}", video_device2);
    let file_thermal = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(video_device2)
        .map_err(|e| {
            format!(
                "Failed to open loopback thermal device {}: {}",
                video_device2, e
            )
        })?;

    // Setup V4L2 properties on loopback devices
    let mjpeg_fourcc = fourcc(b'M', b'J', b'P', b'G');
    let rgb24_fourcc = fourcc(b'R', b'G', b'B', b'3');

    setup_v4l2_device(
        &file_visual,
        FRAME_WIDTH1 as u32,
        FRAME_HEIGHT1 as u32,
        mjpeg_fourcc,
        (FRAME_WIDTH1 * FRAME_HEIGHT1) as u32,
        FRAME_WIDTH1 as u32,
        video_device1,
    )?;

    setup_v4l2_device(
        &file_thermal,
        FRAME_WIDTH2 as u32,
        FRAME_HEIGHT2 as u32,
        rgb24_fourcc,
        (FRAME_WIDTH2 * FRAME_HEIGHT2 * 3) as u32,
        FRAME_WIDTH2 as u32, // Match C version's bytesperline setting (width)
        video_device2,
    )?;

    let mut state = DriverState {
        buf85: vec![0u8; BUF85SIZE],
        buf85pointer: 0,
        colormap,
        ffc_state: 0,
        file_visual,
        file_thermal,
    };

    println!("Initializing USB subsystem...");
    let usb_context = Context::new()?;
    let handle = usb_context
        .open_device_with_vid_pid(VENDOR_ID, PRODUCT_ID)
        .ok_or("Could not open FLIR One USB device. Is it connected?")?;

    handle.set_active_configuration(3)?;
    handle.claim_interface(0)?;
    handle.claim_interface(1)?;
    handle.claim_interface(2)?;
    println!("Successfully claimed FLIR One interfaces 0, 1, 2");

    // USB Setup transfers
    println!("Configuring FLIR One G2/Gen3 USB streaming setup...");
    let _ = handle.write_control(1, 0x0b, 0, 2, &[], Duration::from_millis(100));
    let _ = handle.write_control(1, 0x0b, 0, 1, &[], Duration::from_millis(100));
    let _ = handle.write_control(1, 0x0b, 1, 1, &[], Duration::from_millis(100));
    let _ = handle.write_control(1, 0x0b, 1, 2, &[0, 0], Duration::from_millis(200));

    println!("Starting main USB capture loop...");
    let mut usb_buf = vec![0u8; 1048576];
    let mut last_error = None;

    loop {
        // Read streaming video chunk from endpoint 0x85
        match handle.read_bulk(0x85, &mut usb_buf, Duration::from_millis(100)) {
            Ok(bytes_read) => {
                if bytes_read > 0 {
                    let _ = state.process_usb_data(&usb_buf[..bytes_read]);
                }
            }
            Err(rusb::Error::Timeout) => {
                // Timeouts are expected when waiting for new frame packets
            }
            Err(e) => {
                if last_error != Some(e) {
                    last_error = Some(e);
                    eprintln!("USB error on EP 0x85 bulk read: {:?}", e);
                }
                std::thread::sleep(Duration::from_millis(1000));
            }
        }

        // Poll control endpoints 0x81 and 0x83 to keep connection active
        let mut poll_buf = [0u8; 1024];
        let _ = handle.read_bulk(0x81, &mut poll_buf, Duration::from_millis(10));
        let _ = handle.read_bulk(0x83, &mut poll_buf, Duration::from_millis(10));
    }

    Ok(())
}
