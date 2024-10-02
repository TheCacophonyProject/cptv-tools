use std::io;
use std::io::{ErrorKind, Read};

use js_sys::{Object, Reflect, Uint16Array, Uint8Array};
use log::Level;
#[allow(unused)]
use log::{info, trace, warn};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::ReadableStreamDefaultReader;

use codec::decode::CptvDecoder;

#[derive(Debug)]
struct WebReader {
    is_done: bool,
    stream: ReadableStreamDefaultReader,
    sink: Vec<u8>,
}

impl WebReader {
    async fn pump(&mut self) -> io::Result<usize> {
        // Fetch bytes from the stream to add to sink
        let result = wasm_bindgen_futures::JsFuture::from(self.stream.read()).await;
        match result {
            Ok(result) => {
                let is_done = Reflect::get(&result, &JsValue::from_str("done"))
                    .expect("Should have property 'done'")
                    .as_bool()
                    .unwrap();
                if is_done {
                    self.is_done = true;
                }
                if let Ok(value) = Reflect::get(&result, &JsValue::from_str("value")) {
                    if !value.is_undefined() {
                        let arr = value.dyn_into::<Uint8Array>().unwrap();
                        let bytes_read = arr.length();
                        self.sink.append(&mut arr.to_vec());
                        return Ok(bytes_read as usize);
                    }
                }
                Ok(0)
            }
            Err(_e) => Err(io::Error::new(ErrorKind::UnexpectedEof, "Stream error")),
        }
    }
}

impl Read for WebReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if !self.sink.is_empty() {
            // Copy as many bytes as we can from sink
            let num_bytes = buf.len().min(self.sink.len());
            buf[0..num_bytes].copy_from_slice(&self.sink[0..num_bytes]);
            self.sink.drain(0..num_bytes);
            Ok(num_bytes)
        } else {
            Err(io::Error::new(
                ErrorKind::WouldBlock,
                "Waiting for more bytes from stream",
            ))
        }
    }
}

#[wasm_bindgen]
pub struct CptvDecoderContext {
    inner: CptvDecoder<WebReader>,
}

#[wasm_bindgen]
impl CptvDecoderContext {
    fn reader_mut(&mut self) -> &mut WebReader {
        self.inner.inner_reader().get_mut()
    }

    async fn fetch_bytes(&mut self) -> io::Result<usize> {
        let inner_reader = self.reader_mut();
        let mut num_read_bytes = 0usize;
        while inner_reader.sink.len() < 100_000 && !inner_reader.is_done {
            let result = inner_reader.pump().await;
            match result {
                Ok(b) => num_read_bytes += b,
                Err(e) => return Err(e),
            }
        }
        Ok(num_read_bytes)
    }

    #[wasm_bindgen(js_name = newWithReadableStream)]
    pub fn new_with_readable_stream(stream: ReadableStreamDefaultReader) -> CptvDecoderContext {
        init_console();
        CptvDecoderContext {
            inner: CptvDecoder::new_with_read(WebReader {
                sink: Vec::new(),
                stream,
                is_done: false,
            }),
        }
    }

    #[wasm_bindgen(js_name = getHeader)]
    pub async fn get_header(&mut self) -> JsValue {
        // Make sure there are enough bytes in the sink to make forward progress
        let got_bytes = self.fetch_bytes().await;
        if got_bytes.is_err() {
            return JsValue::from_str("Stream ended unexpectedly");
        }
        match self.inner.get_header() {
            Ok(header) => serde_wasm_bindgen::to_value(&header).unwrap(),
            Err(_) => JsValue::from_str("Unable to parse header"),
        }
    }

    #[wasm_bindgen(js_name = nextFrame)]
    pub async fn next_frame(&mut self) -> JsValue {
        // Make sure there are enough bytes in the sink to make forward progress
        let got_bytes = self.fetch_bytes().await;
        if got_bytes.is_err() {
            return JsValue::from_str("Stream ended unexpectedly");
        }
        let frame = self.inner.next_frame();
        match frame {
            Ok(frame) => {
                let object = Object::new();
                Reflect::set(
                    &object,
                    &JsValue::from_str("imageData"),
                    &JsValue::from(unsafe { Uint16Array::view(frame.image_data.data()) }),
                )
                .unwrap();
                Reflect::set(
                    &object,
                    &JsValue::from_str("timeOnMs"),
                    &JsValue::from(frame.time_on),
                )
                .unwrap();
                Reflect::set(
                    &object,
                    &JsValue::from_str("lastFfcTimeMs"),
                    &JsValue::from(frame.last_ffc_time),
                )
                .unwrap();
                Reflect::set(
                    &object,
                    &JsValue::from_str("lastFfcTempC"),
                    &JsValue::from(frame.last_ffc_temp_c),
                )
                .unwrap();
                Reflect::set(
                    &object,
                    &JsValue::from_str("frameTempC"),
                    &JsValue::from(frame.frame_temp_c),
                )
                .unwrap();
                Reflect::set(
                    &object,
                    &JsValue::from_str("isBackgroundFrame"),
                    &JsValue::from(frame.is_background_frame),
                )
                .unwrap();
                JsValue::from(object)
            }
            Err(_e) => JsValue::NULL,
        }
    }

    #[wasm_bindgen(js_name = nextFrameOwned)]
    pub async fn next_frame_owned(&mut self) -> JsValue {
        // Make sure there are enough bytes in the sink to make forward progress
        let got_bytes = self.fetch_bytes().await;
        if got_bytes.is_err() {
            return JsValue::from_str("Stream ended unexpectedly");
        }
        let frame = self.inner.next_frame();
        match frame {
            Ok(frame) => {
                let object = Object::new();
                let image_data = Uint16Array::new_with_length(frame.image_data.data().len() as u32);
                image_data.copy_from(frame.image_data.data());
                Reflect::set(
                    &object,
                    &JsValue::from_str("imageData"),
                    &JsValue::from(image_data),
                )
                .unwrap();
                Reflect::set(
                    &object,
                    &JsValue::from_str("timeOnMs"),
                    &JsValue::from(frame.time_on),
                )
                .unwrap();
                Reflect::set(
                    &object,
                    &JsValue::from_str("lastFfcTimeMs"),
                    &JsValue::from(frame.last_ffc_time),
                )
                .unwrap();
                Reflect::set(
                    &object,
                    &JsValue::from_str("lastFfcTempC"),
                    &JsValue::from(frame.last_ffc_temp_c),
                )
                .unwrap();
                Reflect::set(
                    &object,
                    &JsValue::from_str("frameTempC"),
                    &JsValue::from(frame.frame_temp_c),
                )
                .unwrap();
                Reflect::set(
                    &object,
                    &JsValue::from_str("isBackgroundFrame"),
                    &JsValue::from(frame.is_background_frame),
                )
                .unwrap();
                JsValue::from(object)
            }
            Err(_e) => JsValue::NULL,
        }
    }
}

fn init_console() {
    console_error_panic_hook::set_once();
    console_log::init_with_level(Level::Info).unwrap_or_else(|_| ());
}
