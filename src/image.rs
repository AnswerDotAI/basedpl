use crate::{data, display, execution::Context, keyed, Error, ErrorAt, ErrorKind, Value};
use ::image::{DynamicImage, ImageBuffer, ImageFormat};
use base64::{engine::general_purpose::STANDARD, Engine};
use std::io::Cursor;

/// `•image Y` returns a picture: the PNG or JPEG file that `Y` names, the bytes of one, or the numbers in `Y`. A picture is numbers from
/// 0 to 1 with axes for rows, columns and up to four channels, displayed as an image. `kind •image Y` encodes `Y` as `"png"` or `"jpeg"`
/// bytes.
pub(crate) fn image(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    if let Some(kind) = left {
        let format = match keyed::name(kind).as_deref() {
            Some("png") => ImageFormat::Png,
            Some("jpeg") => ImageFormat::Jpeg,
            _ => return Err(span.domain_error("•image encodes \"png\" or \"jpeg\"")),
        };
        return data::byte_vector(encoded(right, format, span)?).error_at(span, "image exceeds array limits");
    }
    let picture = match keyed::name(right) {
        Some(path) => decoded(&span.read(&path)?, span)?,
        None if right.shape().len() == 1 => decoded(&data::bytes(right, span)?, span)?,
        None => {
            pixels(right, span)?;
            right.clone()
        }
    };
    picture.with_renderer(display::renderer("image-renderer", render)).error_at(span, "invalid image")
}

fn render(_: Option<&Value>, picture: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let png = STANDARD.encode(encoded(picture, ImageFormat::Png, span)?);
    display::mime("image/png", keyed::text(&png)).error_at(span, "invalid image MIME bundle")
}

/// The picture in the encoded image `bytes`.
fn decoded(bytes: &[u8], span: &Context<'_>) -> Result<Value, Error> {
    let image = ::image::load_from_memory(bytes).map_err(|e| span.domain_error(format!("invalid image: {e}")))?;
    let (width, height, channels) = (image.width() as usize, image.height() as usize, usize::from(image.color().channel_count()));
    let data = match channels {
        1 => image.to_luma16().into_raw(),
        2 => image.to_luma_alpha16().into_raw(),
        3 => image.to_rgb16().into_raw(),
        _ => image.to_rgba16().into_raw(),
    };
    let shape = if channels == 1 { vec![height, width] } else { vec![height, width, channels] };
    Value::floats(shape, data.into_iter().map(|v| f64::from(v) / 65535.).collect()).error_at(span, "image exceeds array limits")
}

/// The picture `picture` encoded in `format`.
fn encoded(picture: &Value, format: ImageFormat, span: &Context<'_>) -> Result<Vec<u8>, Error> {
    let image = pixels(picture, span)?;
    // JPEG has no alpha channel.
    let image = match format {
        ImageFormat::Jpeg if image.color().has_color() => image.to_rgb8().into(),
        ImageFormat::Jpeg => image.to_luma8().into(),
        _ => image,
    };
    let mut bytes = Vec::new();
    image.write_to(&mut Cursor::new(&mut bytes), format).map_err(|e| span.domain_error(format!("image encoding failed: {e}")))?;
    Ok(bytes)
}

/// The image that the numbers in `picture` describe, with each value clamped to 0..1.
fn pixels(picture: &Value, span: &Context<'_>) -> Result<DynamicImage, Error> {
    let (height, width, channels) = match *picture.shape() {
        [height, width] => (height, width, 1),
        [height, width, channels @ 1..=4] => (height, width, channels),
        _ => return Err(span.error(ErrorKind::Rank, "a picture has axes for rows, columns and up to four channels")),
    };
    let values = picture.as_items().reals().ok_or_else(|| span.domain_error("a picture holds real numbers"))?;
    let data: Vec<u8> = values.iter().map(|v| (v.clamp(0., 1.) * 255.).round() as u8).collect();
    let (width, height) = (width as u32, height as u32);
    let image = match channels {
        1 => ImageBuffer::from_raw(width, height, data).map(DynamicImage::ImageLuma8),
        2 => ImageBuffer::from_raw(width, height, data).map(DynamicImage::ImageLumaA8),
        3 => ImageBuffer::from_raw(width, height, data).map(DynamicImage::ImageRgb8),
        _ => ImageBuffer::from_raw(width, height, data).map(DynamicImage::ImageRgba8),
    };
    Ok(image.expect("the shape gives the buffer's length"))
}
