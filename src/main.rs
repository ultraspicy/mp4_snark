extern crate ffmpeg_next as ffmpeg;
extern crate image;

use ffmpeg::{media::Type, util::frame::video::Video};
use ffmpeg::software::scaling::flag::Flags;
use ffmpeg::util::format::pixel::Pixel;
use ffmpeg::codec::context::Context;
use image::jpeg::JpegEncoder;
use std::fs::File;
use std::io::BufWriter;
use image::{Rgb, ImageBuffer};

fn main() -> Result<(), ffmpeg::Error> {
    //Initialize the FFmpeg library
    ffmpeg::init().unwrap();

    let input_path = "input.mp4";

    // Open the video file
    let mut ictx = ffmpeg::format::input(&"input.mp4").unwrap();
     // Print some information about the file
    println!("format: {}", ictx.format().name());
    println!("duration: {}", ictx.duration());

    // Find the video stream in the input file
    let input = ictx
        .streams()
        .best(Type::Video)
        .ok_or(ffmpeg::Error::StreamNotFound)?;
    let video_stream_index = input.index();

    let context_decoder = Context::from_parameters(input.parameters())?;
    let mut decoder = context_decoder.decoder().video()?;

    println!("video_stream_index = {}", video_stream_index);

    let mut scaler = ffmpeg::software::scaling::context::Context::get(
        decoder.format(),
        decoder.width(),
        decoder.height(),
        Pixel::RGB24,
        decoder.width(),
        decoder.height(),
        Flags::BILINEAR,
    )?;

    let mut receive_and_process_decoded_frames =
        |decoder: &mut ffmpeg::decoder::Video| -> Result<(), ffmpeg::Error> {
            let mut decoded = Video::empty();
            let mut frame_index = 0;
            while decoder.receive_frame(&mut decoded).is_ok() {
                // Allocate a frame to store the converted frame
                let mut rgb_frame = Video::empty();
                scaler.run(&decoded, &mut rgb_frame)?;

                // Convert the frame data to an image buffer
                let img_buffer = ImageBuffer::<Rgb<u8>, Vec<u8>>::from_raw(
                    rgb_frame.width(),
                    rgb_frame.height(),
                    rgb_frame.data(0).to_vec(),
                ).ok_or(ffmpeg::Error::Bug)?;

                // Save the image buffer as a JPEG file
                let file_name = format!("frame_{}.jpg", frame_index);
                let file = match File::create(file_name) {
                    Ok(file) => file,
                    Err(error) => panic!("create file failed"),
                };
                let ref mut w = BufWriter::new(file);
                let mut encoder = JpegEncoder::new(w);
                encoder.encode_image(&img_buffer).unwrap();
                frame_index += 1;
            }
            Ok(())
        };
    
    for (stream, packet) in ictx.packets() {
        if stream.index() == video_stream_index {
            decoder.send_packet(&packet)?;
            receive_and_process_decoded_frames(&mut decoder)?;
        }
    }
    decoder.send_eof()?;
    receive_and_process_decoded_frames(&mut decoder)?;

    Ok(())
}