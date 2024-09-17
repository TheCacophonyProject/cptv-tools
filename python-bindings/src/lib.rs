use std::fs::File;
use std::path::Path;

use numpy::ndarray::Array;
use numpy::{IntoPyArray, PyArray2, PyArrayMethods};
use pyo3::prelude::*;

use codec::decode::CptvDecoder;

#[pyclass]
struct CptvReader {
    inner: CptvDecoder<File>,
}

#[pyclass(frozen)]
struct CptvHeader {
    #[pyo3(get)]
    version: u8,
    #[pyo3(get)]
    device_name: String,
    #[pyo3(get)]
    motion_config: Option<String>,
    #[pyo3(get)]
    device_id: u32,
    #[pyo3(get)]
    timestamp: u64,
    #[pyo3(get)]
    x_resolution: u32,
    #[pyo3(get)]
    y_resolution: u32,
    #[pyo3(get)]
    latitude: Option<f32>,
    #[pyo3(get)]
    longitude: Option<f32>,
    #[pyo3(get)]
    altitude: Option<f32>,
    #[pyo3(get)]
    preview_secs: Option<u8>,
    #[pyo3(get)]
    loc_timestamp: Option<u64>,
    #[pyo3(get)]
    fps: u8,
    #[pyo3(get)]
    model: Option<String>,
    #[pyo3(get)]
    brand: Option<String>,
    #[pyo3(get)]
    firmware: Option<String>,
    #[pyo3(get)]
    camera_serial: Option<u32>,
    #[pyo3(get)]
    compression: u8,
    #[pyo3(get)]
    total_frames: Option<u16>,
    #[pyo3(get)]
    max_value: Option<u16>,
    #[pyo3(get)]
    min_value: Option<u16>,
    #[pyo3(get)]
    frame_dim: (u32, u32),
}

#[pyclass(frozen)]
struct CptvFrame {
    #[pyo3(get)]
    time_on: u32,
    #[pyo3(get)]
    last_ffc_time: u32,
    #[pyo3(get)]
    temp_c: f32,
    #[pyo3(get)]
    last_ffc_temp_c: f32,
    #[pyo3(get)]
    background_frame: bool,
    #[pyo3(get)]
    pix: Py<PyArray2<u16>>,
}

#[pymethods]
impl CptvReader {
    #[new]
    pub fn new(path: String) -> CptvReader {
        CptvReader {
            inner: CptvDecoder::<File>::from_path(Path::new(&path)).unwrap(),
        }
    }

    pub fn get_header(&mut self) -> Option<CptvHeader> {
        if let Ok(header) = self.inner.get_header() {
            Some(CptvHeader {
                version: header.version,
                device_name: header.device_name.as_string(),
                device_id: header.device_id.unwrap_or_default(),
                timestamp: header.timestamp,
                motion_config: header.motion_config.map(|s| s.as_string()),
                x_resolution: header.width,
                y_resolution: header.height,
                latitude: header.latitude,
                longitude: header.longitude,
                altitude: header.altitude,
                preview_secs: header.preview_secs,
                loc_timestamp: header.loc_timestamp,
                fps: header.fps,
                model: header.model.map(|s| s.as_string()),
                brand: header.brand.map(|s| s.as_string()),
                firmware: header.firmware_version.map(|s| s.as_string()),
                camera_serial: header.serial_number,
                compression: header.compression,
                total_frames: header.total_frame_count,
                max_value: header.max_value,
                min_value: header.min_value,
                frame_dim: (header.width, header.height),
            })
        } else {
            None
        }
    }

    pub fn next_frame<'py>(&mut self, py: Python<'py>) -> Option<CptvFrame> {
        if let Ok(frame_ref) = self.inner.next_frame() {
            let chunk =
                Array::from_shape_vec((120, 160), frame_ref.image_data.data().to_vec()).unwrap();
            Some(CptvFrame {
                time_on: frame_ref.time_on,
                last_ffc_time: frame_ref.last_ffc_time,
                temp_c: frame_ref.frame_temp_c,
                last_ffc_temp_c: frame_ref.last_ffc_temp_c,
                background_frame: frame_ref.is_background_frame,
                pix: Bound::unbind(chunk.into_pyarray_bound(py)),
            })
        } else {
            None
        }
    }
}

/// A Python module implemented in Rust.
#[pymodule]
fn cptv_rs_python_bindings(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<CptvReader>()?;
    Ok(())
}
