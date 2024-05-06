use crate::common::cptv_field_type::FieldType;
use crate::common::{HEIGHT, WIDTH};
use alloc::string::String;
use core::fmt::{Debug, Formatter};
use log::warn;
use nom::bytes::streaming::{tag, take};
use nom::character::streaming::char;
use nom::error::{ContextError, ErrorKind};
use nom::number::streaming::{le_f32, le_u16, le_u32, le_u64, le_u8};
use serde::Serialize;

#[cfg(feature = "std")]
#[cfg_attr(feature = "std", derive(Serialize))]
#[derive(Clone, PartialEq, Eq)]
pub struct CptvString {
    inner: String,
}

impl Debug for CptvString {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "\"{}\"", self.inner)
    }
}

impl Into<String> for CptvString {
    fn into(self) -> String {
        self.inner
    }
}
#[cfg(feature = "std")]
impl CptvString {
    pub fn new() -> CptvString {
        CptvString {
            inner: String::new(),
        }
    }

    pub fn from_bytes(val: &[u8]) -> CptvString {
        CptvString {
            inner: String::from_utf8_lossy(val).into(),
        }
    }

    pub fn as_string(&self) -> String {
        self.inner.clone()
    }
}

#[cfg(not(feature = "std"))]
#[derive(Clone, Debug)]
struct CptvString {
    inner: [u8; 257],
}
#[cfg(not(feature = "std"))]
impl CptvString {
    pub fn new() -> CptvString {
        CptvString { inner: [0u8; 257] }
    }

    pub fn from_bytes(bytes: &[u8]) -> CptvString {
        let string = CptvString { inner: [0; 257] };
        string.inner[0..bytes.len().min(string.inner.len())].copy_from_slice(bytes);
        string
    }
}

#[cfg_attr(feature = "std", derive(Serialize))]
#[derive(Debug, Clone)]
pub struct CptvHeader {
    pub version: u8,
    pub timestamp: u64,
    pub width: u32,
    pub height: u32,
    pub compression: u8,

    #[cfg_attr(feature = "std", serde(rename = "deviceName"))]
    pub device_name: CptvString,

    pub fps: u8,
    pub brand: Option<CptvString>,
    pub model: Option<CptvString>,
    #[cfg_attr(feature = "std", serde(rename = "deviceId"))]
    pub device_id: Option<u32>,
    #[cfg_attr(feature = "std", serde(rename = "serialNumber"))]
    pub serial_number: Option<u32>,
    #[cfg_attr(feature = "std", serde(rename = "firmwareVersion"))]
    pub firmware_version: Option<CptvString>,
    #[cfg_attr(feature = "std", serde(rename = "motionConfig"))]
    pub motion_config: Option<CptvString>,
    #[cfg_attr(feature = "std", serde(rename = "previewSecs"))]
    pub preview_secs: Option<u8>,
    pub latitude: Option<f32>,
    pub longitude: Option<f32>,
    #[cfg_attr(feature = "std", serde(rename = "locTimestamp"))]
    pub loc_timestamp: Option<u64>,
    pub altitude: Option<f32>,
    pub accuracy: Option<f32>,
    #[cfg_attr(feature = "std", serde(rename = "hasBackgroundFrame"))]
    pub has_background_frame: bool,

    #[cfg_attr(feature = "std", serde(rename = "totalFrames"))]
    pub total_frame_count: Option<u16>,
    #[cfg_attr(feature = "std", serde(rename = "minValue"))]
    pub min_value: Option<u16>,
    #[cfg_attr(feature = "std", serde(rename = "maxValue"))]
    pub max_value: Option<u16>,
}

impl Default for CptvHeader {
    fn default() -> Self {
        CptvHeader {
            version: 0,
            timestamp: 0,
            width: WIDTH as u32,
            height: HEIGHT as u32,
            compression: 0,
            device_name: CptvString::new(),
            fps: 9,
            brand: None,
            model: None,
            device_id: None,
            serial_number: None,
            firmware_version: None,
            motion_config: None,
            preview_secs: None,
            latitude: None,
            longitude: None,
            loc_timestamp: None,
            altitude: None,
            accuracy: None,
            total_frame_count: None,
            min_value: None,
            max_value: None,
            has_background_frame: false,
        }
    }
}
impl CptvHeader {
    pub fn from_bytes(i: &[u8]) -> nom::IResult<&[u8], CptvHeader> {
        let (i, val) = take(4usize)(i)?;
        let (_, _) = tag(b"CPTV")(val)?;
        let (i, version) = le_u8(i)?;
        match version {
            1 | 2 => {
                let mut meta = CptvHeader::default();
                meta.version = version;
                let (i, val) = take(1usize)(i)?;
                let (_, _) = char('H')(val)?;
                let (i, num_header_fields) = le_u8(i)?;
                let mut outer = i;
                for _ in 0..num_header_fields {
                    let (i, field_length) = le_u8(outer)?;
                    let (i, field) = take(1usize)(i)?;
                    let (_, field) = char(field[0] as char)(field)?;
                    let (i, val) = take(field_length)(i)?;
                    outer = i;
                    let field_type = FieldType::from(field);
                    match field_type {
                        FieldType::Timestamp => {
                            meta.timestamp = le_u64(val)?.1;
                        }
                        FieldType::Width => {
                            meta.width = le_u32(val)?.1;
                        }
                        FieldType::Height => {
                            meta.height = le_u32(val)?.1;
                        }
                        FieldType::Compression => {
                            meta.compression = le_u8(val)?.1;
                        }
                        FieldType::DeviceName => {
                            meta.device_name = CptvString::from_bytes(val);
                        }

                        // Optional fields
                        FieldType::FrameRate => meta.fps = le_u8(val)?.1,
                        FieldType::CameraSerial => meta.serial_number = Some(le_u32(val)?.1),
                        FieldType::FirmwareVersion => {
                            meta.firmware_version = Some(CptvString::from_bytes(val));
                        }
                        FieldType::Model => {
                            meta.model = Some(CptvString::from_bytes(val));
                        }
                        FieldType::Brand => {
                            meta.brand = Some(CptvString::from_bytes(val));
                        }
                        FieldType::DeviceID => {
                            meta.device_id = Some(le_u32(val)?.1);
                        }
                        FieldType::MotionConfig => {
                            meta.motion_config = Some(CptvString::from_bytes(val));
                        }
                        FieldType::PreviewSecs => {
                            meta.preview_secs = Some(le_u8(val)?.1);
                        }
                        FieldType::Latitude => {
                            meta.latitude = Some(le_f32(val)?.1);
                        }
                        FieldType::Longitude => {
                            meta.longitude = Some(le_f32(val)?.1);
                        }
                        FieldType::LocTimestamp => {
                            meta.loc_timestamp = Some(le_u64(val)?.1);
                        }
                        FieldType::Altitude => {
                            meta.altitude = Some(le_f32(i)?.1);
                        }
                        FieldType::Accuracy => {
                            meta.accuracy = Some(le_f32(val)?.1);
                        }
                        FieldType::NumFrames => {
                            meta.total_frame_count = Some(le_u16(val)?.1);
                        }
                        FieldType::MinValue => {
                            meta.min_value = Some(le_u16(val)?.1);
                        }
                        FieldType::MaxValue => {
                            meta.max_value = Some(le_u16(val)?.1);
                        }
                        FieldType::BackgroundFrame => {
                            let has_background_frame = le_u8(val)?.1;
                            // NOTE: We expect this to always be 1 if present
                            meta.has_background_frame = has_background_frame == 1;
                        }
                        _ => {
                            warn!("Unknown header field type {}, {}", field, field_length);
                        }
                    }
                }
                Ok((outer, meta))
            }
            _ => Err(nom::Err::Failure(ContextError::add_context(
                i,
                "Unknown CPTV version",
                nom::error::Error::new(i, ErrorKind::Tag),
            ))),
        }
    }
}
