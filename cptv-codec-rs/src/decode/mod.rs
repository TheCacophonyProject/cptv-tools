use flate2::bufread::MultiGzDecoder;
use nom::Needed;
use std::fs::File;
use std::io;
use std::io::{BufReader, Cursor, Error, ErrorKind, Read};
use std::ops::{Deref, DerefMut};
use std::path::Path;

pub use crate::common::cptv_frame::CptvFrame;
pub use crate::common::cptv_header::CptvHeader;
use crate::common::{HEIGHT, WIDTH};

struct DoubleBuffer {
    buffer_a: Vec<u8>,
    buffer_b: Vec<u8>,
    swapped: bool,
}

impl DoubleBuffer {
    pub fn with_capacity(size: usize) -> DoubleBuffer {
        DoubleBuffer {
            buffer_a: Vec::with_capacity(size),
            buffer_b: Vec::with_capacity(size),
            swapped: false,
        }
    }

    #[inline]
    fn front_mut(&mut self) -> &mut Vec<u8> {
        if self.swapped {
            &mut self.buffer_b
        } else {
            &mut self.buffer_a
        }
    }

    #[inline]
    fn front(&self) -> &Vec<u8> {
        if self.swapped {
            &self.buffer_b
        } else {
            &self.buffer_a
        }
    }

    #[inline]
    fn back_mut(&mut self) -> &mut Vec<u8> {
        if self.swapped {
            &mut self.buffer_a
        } else {
            &mut self.buffer_b
        }
    }
    fn consume(&mut self, num_bytes: usize) {
        if self.swapped {
            self.buffer_a.extend_from_slice(&self.buffer_b[num_bytes..])
        } else {
            self.buffer_b.extend_from_slice(&self.buffer_a[num_bytes..])
        }
        self.swapped = !self.swapped;
        self.back_mut().clear();
    }
}

impl Deref for DoubleBuffer {
    type Target = Vec<u8>;

    #[inline]
    fn deref(&self) -> &Vec<u8> {
        self.front()
    }
}

impl DerefMut for DoubleBuffer {
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.front_mut()
    }
}

// Should be more than enough for 1 frames worth of uncompressed data.
const MAX_NEEDED_CAPACITY: usize = 1024 * 64;

#[cfg(feature = "std")]
pub struct CptvDecoder<R: Read> {
    reader: MultiGzDecoder<BufReader<R>>,
    buffer: DoubleBuffer,
    header: Option<CptvHeader>,
    prev_frame: Option<CptvFrame>,
    sequence: Vec<usize>,
}

impl<R: Read> Iterator for CptvDecoder<R> {
    type Item = CptvFrame;

    fn next(&mut self) -> Option<Self::Item> {
        match self.next_frame_owned() {
            Ok(frame) => Some(frame),
            Err(_) => None,
        }
    }
}

#[cfg(feature = "std")]
impl<R: Read> CptvDecoder<R> {
    /// Construct a `CptvDecoder` reading from a given file path.
    pub fn from_path(path: &Path) -> io::Result<CptvDecoder<File>> {
        let f = File::open(path)?;
        Ok(CptvDecoder::<File>::new_with_read(f))
    }

    /// Construct a `CptvDecoder` reading from an owned Vec
    pub fn from_bytes(file_bytes: Vec<u8>) -> io::Result<CptvDecoder<Cursor<Vec<u8>>>> {
        Ok(CptvDecoder::new_with_read(Cursor::new(file_bytes)))
    }

    /// Construct a `CptvDecoder` with any type that implements the `Read` trait.
    pub fn from(reader: R) -> io::Result<CptvDecoder<R>> {
        Ok(CptvDecoder::new_with_read(reader))
    }

    /// If the file header has not yet been decoded, decodes, stores and returns a copy of it.
    /// If it *has* already been decoded, returns a copy of the stored header, so calling this
    /// more than once is safe.
    pub fn get_header(&mut self) -> io::Result<CptvHeader> {
        if self.header.is_none() {
            let mut buffer = [0u8; 1024]; // Read 1024 bytes at a time until we can decode the header.
            let cptv_header: CptvHeader;
            loop {
                let initial_len = self.buffer.len();
                match CptvHeader::from_bytes(&self.buffer) {
                    Ok((remaining, header)) => {
                        cptv_header = header;
                        let used = initial_len - remaining.len();
                        self.buffer.consume(used);
                        break;
                    }
                    Err(e) => match e {
                        nom::Err::Incomplete(need_more_bytes) => match need_more_bytes {
                            Needed::Size(size) => {
                                while self.buffer.len() <= initial_len + size.get() {
                                    match self.read_into_buffer(&mut buffer) {
                                        Ok(_) => (),
                                        Err(e) => return Err(e),
                                    }
                                }
                            }
                            Needed::Unknown => match self.read_into_buffer(&mut buffer) {
                                Ok(_) => (),
                                Err(e) => return Err(e),
                            },
                        },
                        nom::Err::Failure(_e) | nom::Err::Error(_e) => {
                            return Err(Error::new(
                                ErrorKind::Other,
                                "Unexpected input parsing header; not a valid cptv file?",
                            ));
                        }
                    },
                }
            }
            self.header = Some(cptv_header);
        }
        Ok(self.header.as_ref().unwrap().clone())
    }

    /// Decodes the next frame if any, and returns a reference to the latest decoded frame.
    /// If the file header has not yet been decoded, also decodes and stores the Cptv2Header.
    pub fn next_frame(&mut self) -> io::Result<&CptvFrame> {
        let header = self.get_header();
        if !header.is_ok() {
            return Err(header.err().unwrap());
        } else {
            // Get each frame.  The decoder will need to hold onto the previous frame in order
            // to decode the next.
            let mut buffer = [0u8; 1024]; // Read 1024 bytes at a time until we can decode the frame.
            let cptv_frame: CptvFrame;
            loop {
                let initial_len = self.buffer.len();
                match CptvFrame::from_bytes(&self.buffer, &self.prev_frame, &self.sequence) {
                    Ok((remaining, frame)) => {
                        cptv_frame = frame;
                        self.prev_frame = Some(cptv_frame);
                        let used = initial_len - remaining.len();
                        self.buffer.consume(used);
                        break;
                    }
                    Err(e) => match e {
                        nom::Err::Incomplete(need_more_bytes) => match need_more_bytes {
                            Needed::Size(size) => {
                                while self.buffer.len() <= initial_len + size.get() {
                                    match self.read_into_buffer(&mut buffer) {
                                        Ok(_) => (),
                                        Err(e) => return Err(e),
                                    }
                                }
                            }
                            Needed::Unknown => match self.read_into_buffer(&mut buffer) {
                                Ok(_) => (),
                                Err(e) => return Err(e),
                            },
                        },
                        nom::Err::Failure(_e) | nom::Err::Error(_e) => {
                            return Err(Error::new(
                                ErrorKind::Other,
                                "Unexpected input, CPTV file may be corrupt?",
                            ));
                        }
                    },
                }
            }
            Ok(self.prev_frame.as_ref().unwrap())
        }
    }

    /// Decodes the next frame if any, and returns an owned clone of it.
    /// If the file header has not yet been decoded, also decodes and stores the Cptv2Header.
    pub fn next_frame_owned(&mut self) -> io::Result<CptvFrame> {
        self.next_frame().map(|r| r.clone())
    }

    pub fn new_with_read(read: R) -> CptvDecoder<R> {
        //let reader = Rc::new(Box::new(read));
        CptvDecoder {
            reader: MultiGzDecoder::new(BufReader::new(read)),
            //ext_reader: reader,
            buffer: DoubleBuffer::with_capacity(MAX_NEEDED_CAPACITY),
            header: None,
            prev_frame: None,
            sequence: (0..WIDTH)
                .chain((0..WIDTH).rev())
                .cycle()
                .take(WIDTH * HEIGHT)
                .enumerate()
                .map(|(index, i)| ((index / WIDTH) * WIDTH) + i)
                .skip(1)
                .collect(),
        }
    }

    pub fn inner_reader(&mut self) -> &mut BufReader<R> {
        self.reader.get_mut()
    }

    fn read_into_buffer(&mut self, buffer: &mut [u8; 1024]) -> Result<(), Error> {
        match self.reader.read(buffer) {
            Ok(bytes_read) => {
                if bytes_read > 0 {
                    self.buffer.extend_from_slice(&buffer[0..bytes_read]);
                    Ok(())
                } else {
                    return Err(Error::new(ErrorKind::Other, "Reached end of input"));
                }
            }
            Err(e) => {
                match e.kind() {
                    ErrorKind::Interrupted => {
                        // Let the loop continue and retry
                        Ok(())
                    }
                    _ => return Err(e),
                }
            }
        }
    }
}
