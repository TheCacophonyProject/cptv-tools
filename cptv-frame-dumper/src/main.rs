use std::fs::File;
use std::io;
use std::path::Path;
use std::time::Instant;

use argh::FromArgs;
use image::EncodableLayout;

use codec::decode::CptvDecoder;

#[derive(FromArgs)]
/// Simple utility to dump CPTV files to a sequence of PNG files.
struct CliConfig {
    /// path to the CPTV file you want to dump
    #[argh(option)]
    path: String,

    /// path to location where CPTV frames should be output
    #[argh(option)]
    output_path: Option<String>,

    /// enable to colorize using Viridis palette
    #[argh(switch)]
    colorize: bool,

    /// enable to normalization over the entire clip, rather than on a per-frame basis
    #[argh(switch)]
    normalize_over_clip: bool,

    /// the frame number to start dumping (first frame is frame 1)
    #[argh(option)]
    start_frame: Option<u32>,

    /// the frame number to end dumping
    #[argh(option)]
    end_frame: Option<u32>,

    /// format to dump - PNG, GIF, animated GIF, animated PNG
    #[argh(option)]
    format: Option<u32>,
}

fn u32_slice_to_u8_slice(slice: &[u32]) -> &[u8] {
    unsafe {
        std::slice::from_raw_parts(
            slice as *const [u32] as *const u8,
            slice.len() * std::mem::size_of::<u32>(),
        )
    }
}

/// This is intended to serve as a useful demo application of the `cptv-codec-rs` crate.
fn main() -> io::Result<()> {
    let start = Instant::now();
    let config: CliConfig = argh::from_env();
    let file = File::open(&Path::new(&config.path))?;
    let mut decoder = CptvDecoder::from(file)?;
    let mut num_frames = 0;
    let header = decoder.get_header()?;
    let mut clip_max = header.max_value;
    let mut clip_min = header.min_value;
    let start_frame = config.start_frame.unwrap_or(0) as isize;
    let end_frame = config.end_frame.unwrap_or(u32::MAX) as isize;

    if config.normalize_over_clip && clip_min.is_none() || clip_max.is_none() {
        println!("Normalizing over entire clip");
        // We need to get the min/max for the clip
        let mut min = u16::MAX;
        let mut max = u16::MIN;
        for frame in decoder
            .skip((start_frame - 1).max(0) as usize)
            .take((end_frame - (start_frame - 1).max(0)) as usize)
        {
            min = *frame
                .image_data
                .data()
                .iter()
                .min()
                .unwrap_or(&min)
                .min(&min);
            max = *frame
                .image_data
                .data()
                .iter()
                .max()
                .unwrap_or(&max)
                .max(&max);
        }
        clip_min = Some(min);
        clip_max = Some(max);
    }
    // Reopen the file to get a fresh decoder
    let file = File::open(&Path::new(&config.path))?;
    let decoder = CptvDecoder::from(file)?;
    let gradient = colorous::VIRIDIS;

    for (frame_num, frame) in decoder
        .enumerate()
        .skip((start_frame - 1).max(0) as usize)
        .take((end_frame - (start_frame - 1).max(0)) as usize)
    {
        num_frames += 1;
        let min = if config.normalize_over_clip {
            clip_min.unwrap()
        } else {
            *frame.image_data.data().iter().min().unwrap_or(&u16::MIN)
        };
        let max = if config.normalize_over_clip {
            clip_max.unwrap()
        } else {
            *frame.image_data.data().iter().max().unwrap_or(&u16::MAX)
        };
        let range = (max - min) as f32;
        if !config.colorize {
            let normalized: Vec<_> = frame
                .image_data
                .data()
                .iter()
                .map(|x| (((*x - min) as f32 / range) * u16::MAX as f32) as u16)
                .collect();
            image::save_buffer(
                format!("./output/frame-{}.png", frame_num + 1),
                &normalized.as_bytes(),
                header.width,
                header.height,
                image::ExtendedColorType::L16,
            )
            .unwrap();
        } else {
            // map into Rgb16 space
            let colorized: Vec<u32> = frame
                .image_data
                .data()
                .iter()
                .map(|val| {
                    let val = gradient.eval_continuous(((*val - min) as f32 / range) as f64);
                    255 << 24 | (val.b as u32) << 16 | (val.g as u32) << 8 | (val.r as u32)
                })
                .collect();
            image::save_buffer(
                format!("./output/frame-{}.png", frame_num + 1),
                u32_slice_to_u8_slice(&colorized),
                header.width,
                header.height,
                image::ExtendedColorType::Rgba8,
            )
            .unwrap();
        }
    }
    println!("Dumped {} frames", num_frames);
    println!("Took {:?}", Instant::now().duration_since(start));
    Ok(())
}
