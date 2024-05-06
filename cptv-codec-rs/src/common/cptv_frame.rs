#[cfg(feature = "std")]
use alloc::format;
#[cfg(feature = "std")]
use core::fmt;

#[cfg(feature = "std")]
use core::fmt::Formatter;

#[cfg(feature = "std")]
use core::time::Duration;

use crate::common::cptv_field_type::FieldType;
use crate::common::{HEIGHT, WIDTH};
use byteorder::{ByteOrder, LittleEndian};
use core::fmt::Debug;
use core::ops::{Index, IndexMut};
use log::warn;
use nom::bytes::streaming::take;
use nom::character::streaming::char;
use nom::number::streaming::{le_f32, le_u32, le_u8};
use serde::Serialize;

#[inline(always)]
fn reverse_twos_complement(v: u32, width: u8) -> i32 {
    if v & (1 << (width - 1)) as u32 == 0 {
        v as i32
    } else {
        -(((!v + 1) & ((1 << width as u32) - 1)) as i32)
    }
}

pub struct BitUnpacker<'a> {
    input: &'a [u8],
    offset: usize,
    bit_width: u8,
    num_bits: u8,
    bits: u32,
}

impl<'a> BitUnpacker<'a> {
    pub fn new(input: &'a [u8], bit_width: u8) -> BitUnpacker {
        BitUnpacker {
            input,
            offset: 0,
            bit_width,
            num_bits: 0,
            bits: 0,
        }
    }
}

// TODO: This could probably be optimised somewhat.
impl<'a> Iterator for BitUnpacker<'a> {
    type Item = i32;
    fn next(&mut self) -> Option<Self::Item> {
        while self.num_bits < self.bit_width {
            match self.input.get(self.offset) {
                Some(byte) => {
                    self.bits |= (*byte as u32) << (24 - self.num_bits) as u32;
                    self.num_bits += 8;
                }
                None => return None,
            }
            self.offset += 1;
        }
        let out =
            reverse_twos_complement(self.bits >> (32 - self.bit_width) as u32, self.bit_width);
        self.bits = self.bits << self.bit_width as u32;
        self.num_bits -= self.bit_width;
        Some(out)
    }
}
fn u8_slice_as_u16_slice(p: &[u8]) -> &[u16] {
    assert_eq!(
        (p.len() / 2) * 2,
        p.len(),
        "slice must be evenly divisible by 2"
    );
    unsafe { core::slice::from_raw_parts((p as *const [u8]) as *const u16, p.len() / 2) }
}
fn unpack_frame_v2(
    prev_frame: &CptvFrame,
    data: &[u8],
    bit_width: u8,
    snake_sequence: &[usize],
) -> FrameData {
    let initial_px = LittleEndian::read_i32(&data[0..4]);
    let i = &data[4..];
    let mut current_px = initial_px;

    // Seed the initial pixel value
    let prev_px = prev_frame.image_data[0][0] as i32;
    assert!(prev_px + current_px <= u16::MAX as i32);
    assert!(prev_px + current_px >= 0);
    let mut image_data = FrameData::new();
    image_data[0][0] = (prev_px + current_px) as u16;
    let num_remaining_px = (WIDTH * HEIGHT) - 1;

    if bit_width == 16 {
        for (&index, delta) in snake_sequence
            .iter()
            .zip(u8_slice_as_u16_slice(i).iter().take(num_remaining_px))
        {
            current_px += (*delta as i16) as i32;
            let prev_px = unsafe { *prev_frame.image_data.data.get_unchecked(index) } as i32;
            let px = (prev_px + current_px) as u16;
            (*unsafe { image_data.data.get_unchecked_mut(index) }) = px;
        }
    } else if bit_width == 8 {
        for (&index, delta) in snake_sequence.iter().zip(i.iter().take(num_remaining_px)) {
            current_px += (*delta as i8) as i32;
            let prev_px = unsafe { *prev_frame.image_data.data.get_unchecked(index) } as i32;
            let px = (prev_px + current_px) as u16;
            (*unsafe { image_data.data.get_unchecked_mut(index) }) = px;
        }
    } else {
        for (&index, delta) in snake_sequence
            .iter()
            .zip(BitUnpacker::new(i, bit_width).take(num_remaining_px))
        {
            current_px += delta;
            let prev_px = unsafe { *prev_frame.image_data.data.get_unchecked(index) } as i32;
            let px = (prev_px + current_px) as u16;
            (*unsafe { image_data.data.get_unchecked_mut(index) }) = px;
        }
    }
    image_data
}

#[cfg_attr(feature = "std", derive(Serialize))]
#[derive(Clone)]
pub struct CptvFrame {
    #[cfg_attr(feature = "std", serde(rename = "timeOnMs"))]
    pub time_on: u32,

    // Some cameras may not have FFC information, so this is optional.
    #[cfg_attr(feature = "std", serde(rename = "lastFfcTimeMs"))]
    pub last_ffc_time: u32,
    #[cfg_attr(feature = "std", serde(rename = "lastFfcTempC"))]
    pub last_ffc_temp_c: f32,
    #[cfg_attr(feature = "std", serde(rename = "frameTempC"))]
    pub frame_temp_c: f32,

    #[cfg_attr(feature = "std", serde(rename = "isBackgroundFrame"))]
    pub is_background_frame: bool,

    // Raw image data?
    #[cfg_attr(feature = "std", serde(rename = "imageData"))]
    pub image_data: FrameData,
}

impl CptvFrame {
    pub fn new() -> CptvFrame {
        CptvFrame {
            time_on: 0,
            last_ffc_time: 0,
            last_ffc_temp_c: 0.0,
            frame_temp_c: 0.0,
            is_background_frame: false,
            image_data: FrameData::new(),
        }
    }

    pub fn empty() -> CptvFrame {
        CptvFrame {
            time_on: 0,
            last_ffc_time: 0,
            last_ffc_temp_c: 0.0,
            frame_temp_c: 0.0,
            is_background_frame: false,
            image_data: FrameData::new(),
        }
    }

    pub fn from_bytes<'a>(
        data: &'a [u8],
        prev_frame: &Option<CptvFrame>,
        sequence: &[usize],
    ) -> nom::IResult<&'a [u8], CptvFrame, (&'a [u8], nom::error::ErrorKind)> {
        let (i, val) = take(1usize)(data)?;
        let (_, _) = char('F')(val)?;
        let (i, num_frame_fields) = le_u8(i)?;

        let mut is_background_frame = false;
        let mut frame_temp_c = 0.0;
        let mut time_on = 0;
        let mut last_ffc_time = 0;
        let mut last_ffc_temp_c = 0.0;

        let mut bit_width = 0;
        let mut frame_size = 0;

        let mut outer = i;
        for _ in 0..num_frame_fields as usize {
            let (i, field_length) = le_u8(outer)?;
            let (i, field) = take(1usize)(i)?;
            let (_, field_code) = char(field[0] as char)(field)?;
            let (i, val) = take(field_length)(i)?;
            outer = i;
            let fc = FieldType::from(field_code);
            match fc {
                FieldType::TimeOn => {
                    // NOTE: In version 1, time_on was just an offset to the timestamp in the cptv header.
                    time_on = le_u32(val)?.1;
                }
                FieldType::BitsPerPixel => {
                    bit_width = le_u8(val)?.1;
                }
                FieldType::FrameSize => {
                    frame_size = le_u32(val)?.1;
                }
                FieldType::LastFfcTime => {
                    // NOTE: Last ffc time is relative to time_on, so we need to adjust it accordingly
                    // when printing the value.
                    last_ffc_time = le_u32(val)?.1;
                }
                FieldType::LastFfcTempC => {
                    last_ffc_temp_c = le_f32(val)?.1;
                }
                FieldType::FrameTempC => {
                    frame_temp_c = le_f32(val)?.1;
                }
                FieldType::BackgroundFrame => {
                    is_background_frame = le_u8(val)?.1 == 1;
                }
                _ => {
                    warn!(
                        "Unknown frame field type '{}', length: {}",
                        field_code as char, field_length
                    );
                }
            }
        }

        assert!(frame_size > 0);
        let (i, data) = take(frame_size as usize)(outer)?;
        // Now try to decode frame data.
        let prev_frame = prev_frame.as_ref();
        let empty_frame;
        let prev_frame = match prev_frame {
            Some(frame) => frame,
            None => {
                empty_frame = CptvFrame::empty();
                &empty_frame
            }
        };
        let image_data = unpack_frame_v2(prev_frame, data, bit_width, sequence);
        Ok((
            i,
            CptvFrame {
                time_on,
                last_ffc_time,
                last_ffc_temp_c,
                frame_temp_c,
                is_background_frame,
                image_data,
            },
        ))
    }
}

#[cfg_attr(feature = "std", derive(Serialize))]
#[derive(Clone)]
pub struct FrameData {
    #[cfg_attr(feature = "std", serde(skip_serializing))]
    data: [u16; WIDTH * HEIGHT],
}

impl FrameData {
    pub fn new() -> FrameData {
        FrameData {
            data: [0; WIDTH * HEIGHT],
        }
    }

    pub fn view(&self, x: usize, y: usize, w: usize, h: usize) -> impl Iterator<Item = &u16> {
        let start_row = y * WIDTH;
        let end_row = (y + h) * WIDTH;
        self.data[start_row..end_row]
            .chunks_exact(WIDTH)
            .into_iter()
            .flat_map(move |row| row.iter().skip(x).take(w))
    }

    pub fn view_mut(
        &mut self,
        x: usize,
        y: usize,
        w: usize,
        h: usize,
    ) -> impl Iterator<Item = &mut u16> {
        let start_row = y * WIDTH;
        let end_row = (y + h) * WIDTH;
        self.data[start_row..end_row]
            .chunks_exact_mut(WIDTH)
            .into_iter()
            .flat_map(move |row| row.iter_mut().skip(x).take(w))
    }
    pub fn data(&self) -> &[u16] {
        &self.data
    }

    pub fn as_slice(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(
                &self.data as *const u16 as *const u8,
                core::mem::size_of::<u16>() * self.data.len(),
            )
        }
    }

    pub fn snaking_iter(&self) -> impl Iterator {
        (0..WIDTH)
            .chain((WIDTH - 1)..=0)
            .cycle()
            .take(WIDTH * HEIGHT)
    }
}

// Row indices for FrameData
impl Index<usize> for FrameData {
    type Output = [u16];

    #[inline(always)]
    fn index(&self, index: usize) -> &Self::Output {
        debug_assert!(index < HEIGHT);
        &self.data[(index * WIDTH)..(index * WIDTH) + WIDTH]
    }
}

impl IndexMut<usize> for FrameData {
    #[inline(always)]
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        debug_assert!(index < HEIGHT);
        &mut self.data[(index * WIDTH)..(index * WIDTH) + WIDTH]
    }
}

#[cfg(feature = "std")]
impl Debug for CptvFrame {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("CptvFrame")
            .field("last_ffc_time", &{
                format!(
                    "{}s",
                    Duration::from_millis(self.time_on as u64 - self.last_ffc_time as u64)
                        .as_secs()
                )
            })
            .field("time_on", &{
                let seconds = Duration::from_millis(self.time_on as u64).as_secs();
                let minutes = seconds / 60;
                let hours = minutes / 60;
                let minutes = minutes - (hours * 60);
                let seconds = seconds - ((hours * 60 * 60) + (minutes * 60));
                if hours > 0 {
                    // Minutes
                    format!("{}h, {}m, {}s", hours, minutes, seconds)
                } else if minutes > 0 {
                    format!("{}m, {}s", minutes, seconds)
                } else {
                    format!("{}s", seconds)
                }
            })
            .field("frame_temp_c", &self.frame_temp_c)
            .field("last_ffc_temp_c", &self.last_ffc_temp_c)
            .field("is_background_frame", &self.is_background_frame)
            .field("image_data", &format!("FrameData({}x{})", WIDTH, HEIGHT))
            .finish()
    }
}
