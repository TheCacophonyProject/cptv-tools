#[repr(u8)]
#[derive(PartialEq, Debug, Copy, Clone)]
pub enum FieldType {
    // K remaining
    Header = b'H',
    Timestamp = b'T',
    Width = b'X',
    Height = b'Y',
    Compression = b'C',
    DeviceName = b'D',
    MotionConfig = b'M',
    PreviewSecs = b'P',
    Latitude = b'L',
    Longitude = b'O',
    LocTimestamp = b'S',
    Altitude = b'A',
    Accuracy = b'U',
    Model = b'E',
    Brand = b'B',
    DeviceID = b'I',
    FirmwareVersion = b'V',
    CameraSerial = b'N',
    FrameRate = b'Z',
    BackgroundFrame = b'g',

    // TODO: Other header fields added to V2
    MinValue = b'Q',
    MaxValue = b'K',
    NumFrames = b'J',

    // Reserved for future seekable CPTV format
    FramesPerIframe = b'G',

    FrameHeader = b'F',

    BitsPerPixel = b'w',
    FrameSize = b'f',
    LastFfcTime = b'c',
    FrameTempC = b'a',
    LastFfcTempC = b'b',
    TimeOn = b't',
    Unknown = b';',
}
impl From<char> for FieldType {
    fn from(val: char) -> Self {
        use FieldType::*;
        match val {
            'H' => Header,
            'T' => Timestamp,
            'X' => Width,
            'Y' => Height,
            'C' => Compression,
            'D' => DeviceName,
            'E' => Model,
            'B' => Brand,
            'I' => DeviceID,
            'M' => MotionConfig,
            'P' => PreviewSecs,
            'L' => Latitude,
            'O' => Longitude,
            'S' => LocTimestamp,
            'A' => Altitude,
            'U' => Accuracy,
            'Q' => MinValue,
            'K' => MaxValue,
            'N' => CameraSerial,
            'V' => FirmwareVersion,
            'J' => NumFrames,
            'Z' => FrameRate,
            'G' => FramesPerIframe,
            'F' => FrameHeader,
            'g' => BackgroundFrame,
            'w' => BitsPerPixel,
            'f' => FrameSize,
            'c' => LastFfcTime,
            't' => TimeOn,
            'a' => FrameTempC,
            'b' => LastFfcTempC,
            _ => Unknown,
        }
    }
}
