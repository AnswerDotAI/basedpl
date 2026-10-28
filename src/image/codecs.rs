//! `•image`'s decoding and encoding through the `image` and `png` crates, on platforms without ImageIO.
use super::{Format, Raster, ratio, rechannel_into};
use ::image::codecs::{jpeg::JpegEncoder, png::PngDecoder};
use ::image::{AnimationDecoder, ExtendedColorType, ImageDecoder, ImageEncoder, ImageError};
use std::io::Cursor;

/// The pixels of a PNG or JPEG file, widened to 16 bits.
pub(super) fn decode(bytes: &[u8]) -> Result<Raster<u16>, String> {
    let text = |e: ImageError| e.to_string();
    if let Ok(png) = PngDecoder::new(Cursor::new(bytes)) { if png.is_apng().map_err(text)? { return animation(png); } }
    let image = ::image::load_from_memory(bytes).map_err(text)?;
    let (width, height, channels) = (image.width() as usize, image.height() as usize, usize::from(image.color().channel_count()));
    let data = match channels {
        1 => image.into_luma16().into_raw(), 2 => image.into_luma_alpha16().into_raw(), 3 => image.into_rgb16().into_raw(), _ => image.into_rgba16().into_raw()
    };
    Ok(Raster { count: 1, height, width, channels, data })
}

/// The frames of the animated PNG that `png` reads. Each frame comes composited to RGBA. It's appended to the result with the channels the file holds.
fn animation(png: PngDecoder<Cursor<&[u8]>>) -> Result<Raster<u16>, String> {
    let text = |e: ImageError| e.to_string();
    let channels = usize::from(png.color_type().channel_count());
    let (width, height) = png.dimensions();
    let (mut data, mut count) = (Vec::new(), 0);
    for frame in png.apng().map_err(text)?.into_frames() {
        rechannel_into(&mut data, frame.map_err(text)?.buffer().as_raw(), 4, channels, u8::MAX, |v| u16::from(v) * 257);
        count += 1;
    }
    if count == 0 { return Err("no frames".into()); }
    Ok(Raster { count, height: height as usize, width: width as usize, channels, data })
}

/// `frames` as a PNG, animated with `delays`, or as a JPEG.
pub(super) fn encode(frames: &Raster<u8>, delays: Option<&[f64]>, format: Format) -> Result<Vec<u8>, String> {
    let (width, height) = (frames.width as u32, frames.height as u32);
    let mut bytes = Vec::new();
    if format == Format::Jpeg {
        let colour = if frames.channels == 1 { ExtendedColorType::L8 } else { ExtendedColorType::Rgb8 };
        JpegEncoder::new(&mut bytes).write_image(frames.frame(0), width, height, colour).map_err(|e| e.to_string())?;
        return Ok(bytes);
    }
    let mut encoder = png::Encoder::new(&mut bytes, width, height);
    encoder.set_color(match frames.channels {
        1 => png::ColorType::Grayscale, 2 => png::ColorType::GrayscaleAlpha, 3 => png::ColorType::Rgb, _ => png::ColorType::Rgba
    });
    encoder.set_depth(png::BitDepth::Eight);
    if delays.is_some() { encoder.set_animated(frames.count as u32, 0).map_err(|e| e.to_string())?; }
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    for i in 0..frames.count {
        if let Some(delays) = delays {
            let (num, den) = ratio(delays[i % delays.len()]);
            writer.set_frame_delay(num, den).map_err(|e| e.to_string())?;
        }
        writer.write_image_data(frames.frame(i)).map_err(|e| e.to_string())?;
    }
    writer.finish().map_err(|e| e.to_string())?;
    Ok(bytes)
}
