#![feature(test)]

extern crate test;
use std::fs::File;
use std::path::Path;
use std::{fs, io};

#[test]
fn decode_cptv1_file() -> io::Result<()> {
    let file = File::open(&Path::new("./tests/fixtures/20180603-090916.cptv"))?;
    let mut decoder = CptvDecoder::from(file)?;
    let header = decoder.get_header()?;

    assert_eq!(header.width, 160);
    assert_eq!(header.height, 120);

    // Iterate over references to the current decoded frame
    let mut num_frames = 0;
    loop {
        let frame = decoder.next_frame();
        if let Ok(_frame) = frame {
            num_frames += 1;
        } else {
            break;
        }
    }
    assert_eq!(num_frames, 309);
    println!("Num frames {}", num_frames);

    Ok(())
}

#[test]
fn decode_cptv2_file() -> io::Result<()> {
    // NOTE: Decode a basic test cptv file, and check it has the expected
    //  header, number of frames, and maybe that the sha1 hash of the image data
    //  is what we expected.
    let file = File::open(&Path::new("./tests/fixtures/20201221-748923.cptv"))?;
    let mut decoder = CptvDecoder::from(file)?;
    let header = decoder.get_header()?;

    assert_eq!(header.width, 160);
    assert_eq!(header.height, 120);
    assert!(header.brand.is_some());
    assert_eq!(header.brand.unwrap().as_string(), "flir");

    // Iterate over references to the current decoded frame
    let mut num_frames = 0;
    loop {
        let frame = decoder.next_frame();
        if let Ok(_frame) = frame {
            num_frames += 1;
        } else {
            break;
        }
    }
    assert_eq!(num_frames, 378);
    println!("Num frames {}", num_frames);

    Ok(())
}

#[test]
fn decode_cptv2_file_iterator_count() -> io::Result<()> {
    let file = File::open(&Path::new("./tests/fixtures/20201221-748923.cptv"))?;
    let decoder = CptvDecoder::from(file)?;
    let num_frames = decoder.count();
    assert_eq!(num_frames, 378);
    println!("Num frames {}", num_frames);

    Ok(())
}

#[test]
fn decode_cptv2_file_iterator() -> io::Result<()> {
    let file = File::open(&Path::new("./tests/fixtures/20201221-748923.cptv"))?;
    let decoder = CptvDecoder::from(file)?;
    let mut last_time_on = 0;
    let mut num_frames = 0;
    for frame in decoder {
        assert!(frame.time_on > last_time_on);
        last_time_on = frame.time_on;
        num_frames += 1;
    }
    assert_eq!(num_frames, 378);
    println!("Num frames {}", num_frames);

    Ok(())
}

use codec::decode::CptvDecoder;
use test::Bencher;

#[bench]
fn decode_cptv2_file_benchmark(b: &mut Bencher) {
    let file = fs::read(Path::new("./tests/fixtures/20201221-748923.cptv")).unwrap();
    b.iter(|| {
        let mut decoder = CptvDecoder::from(&file[..]).unwrap();
        let mut num_frames = 0;
        loop {
            let frame = decoder.next_frame();
            if let Ok(_frame) = frame {
                num_frames += 1;
            } else {
                break;
            }
        }
        assert_eq!(num_frames, 378);
    });
}

#[test]
fn decode_cptv2_low_power_file() -> io::Result<()> {
    // A file created by our minimalistic streaming encoder in low power mode for DOC AI Cam.
    let file = File::open(&Path::new("./tests/fixtures/20240507-074536.cptv"))?;
    let decoder = CptvDecoder::from(file)?;
    let mut last_time_on = 0;
    let mut num_frames = 0;
    for frame in decoder {
        assert!(frame.time_on > last_time_on);
        println!("Frame #{}, {:#?}", num_frames + 1, frame);
        last_time_on = frame.time_on;
        num_frames += 1;
    }
    assert_eq!(num_frames, 45);
    println!("Num frames {}", num_frames);
    Ok(())
}
