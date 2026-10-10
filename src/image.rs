use crate::{data, display, element::read_as, execution::Context, keyed, Error, ErrorAt, ErrorKind, Value};
use ::image::{DynamicImage, ImageBuffer, ImageFormat};
use std::io::Cursor;

/// The rate an animation plays at without an `fps` or `delay` option.
const DEFAULT_FPS: f64 = 24.;

/// `X •image Y` returns a picture: the PNG or JPEG file that `Y` names, the bytes of one, or the numbers in `Y`. A picture is numbers
/// from 0 to 1 with axes for rows, columns and up to four channels, displayed as an image. An animation has frames along a first axis:
/// a rank-4 array, or any array of pictures with the option `fps`, the frame rate, or `delay`, the seconds each frame stays on screen.
pub(crate) fn image(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let options = data::Options::new("•image", left, None, &["fps", "delay"], span)?;
    let delays = timing(&options, "•image", span)?;
    let picture = match keyed::name(right) {
        Some(path) => decoded(&span.read(&path)?, span)?,
        None if right.shape().len() == 1 => decoded(&data::bytes(right, span)?, span)?,
        None => right.clone(),
    };
    frames(&picture, delays.clone(), span)?;
    let renderer = display::renderer("image-renderer", render);
    let renderer = match delays {
        Some(d) => crate::eval::before(Value::floats(vec![d.len()], span.numeric().width, d).error_at(span, "invalid delays")?, renderer, span.span)?,
        None => renderer,
    };
    picture.with_renderer(renderer).error_at(span, "invalid image")
}

/// `options •image⁻¹ Y` encodes the picture `Y` as bytes. The option `format` is `"png"`, the default, or `"jpeg"`. Plain text on
/// the left gives the format alone. With the option `fps` or `delay`, or with four axes, `Y` is an animation, which becomes an animated PNG.
pub(crate) fn encode(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let options = data::Options::new("•image⁻¹", left, Some("format"), &["format", "fps", "delay"], span)?;
    let format = match options.text("format", Some("png"), span)?.to_lowercase().as_str() {
        "png" => ImageFormat::Png,
        "jpeg" => ImageFormat::Jpeg,
        _ => return Err(span.domain_error("•image⁻¹ encodes \"png\" or \"jpeg\"")),
    };
    data::byte_vector(encoded(frames(right, timing(&options, "•image⁻¹", span)?, span)?, format, span)?).error_at(span, "image exceeds array limits")
}

/// The seconds each frame stays on screen, from the option `delay` of `name`, or from `fps`. Either makes the first axis of a picture
/// time. `delay` is one number for every frame, or one per frame, and sets the timing when both are given.
fn timing(options: &data::Options, name: &str, span: &Context<'_>) -> Result<Option<Vec<f64>>, Error> {
    let (key, delays) = match options.values.get("delay") {
        Some(d) => ("delay", (d.shape().len() <= 1).then(|| read_as::<f64>(d)).flatten().map(|d| d.into_owned())),
        None if options.values.contains_key("fps") => ("fps", Some(vec![1. / options.number("fps", DEFAULT_FPS, span)?])),
        None => return Ok(None),
    };
    match delays {
        Some(d) if !d.is_empty() && d.iter().all(|s| *s > 0. && s.is_finite()) => Ok(Some(d)),
        _ => Err(span.domain_error(format!("{name} {key} must be positive"))),
    }
}

/// Native builds send a PNG, animated for an animation. The browser build sends an animation the same way, and a still picture as
/// `image/x-rgba` bytes, with the width as a parameter, for its page to draw on a canvas. `delays` are the seconds per frame that
/// `•image`'s options set.
fn render(delays: Option<&Value>, picture: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let delays = delays.map(|d| read_as::<f64>(d).expect("•image keeps its delays as numbers").into_owned());
    let frames = frames(picture, delays, span)?;
    let (kind, bytes) = match &frames {
        Frames { images, delays: None } if cfg!(web) => (format!("image/x-rgba;width={}", images[0].width()), images[0].to_rgba8().into_raw()),
        _ => ("image/png".into(), encoded(frames, ImageFormat::Png, span)?),
    };
    let bytes = data::byte_vector(bytes).error_at(span, "image exceeds array limits")?;
    display::mime(&kind, bytes).error_at(span, "invalid image MIME bundle")
}

/// The picture in the encoded image `bytes`. An animated PNG with more than one frame gives its frames along a first axis, as a rank-4
/// array with a channel axis even for grey frames, so `•image⁻¹` encodes it back.
fn decoded(bytes: &[u8], span: &Context<'_>) -> Result<Value, Error> {
    use ::image::{codecs::png::PngDecoder, AnimationDecoder, ImageDecoder};
    let invalid = |e: ::image::ImageError| span.domain_error(format!("invalid image: {e}"));
    // An animated PNG's frames come composited to RGBA, and go back to the channels the file holds.
    let (images, channels) = match PngDecoder::new(Cursor::new(bytes)) {
        Ok(png) if png.is_apng().map_err(invalid)? => {
            let channels = png.color_type().channel_count();
            let frames = png.apng().map_err(invalid)?.into_frames().collect_frames().map_err(invalid)?;
            (frames.into_iter().map(|f| DynamicImage::ImageRgba8(f.into_buffer())).collect::<Vec<_>>(), channels)
        }
        _ => {
            let image = ::image::load_from_memory(bytes).map_err(invalid)?;
            let channels = image.color().channel_count();
            (vec![image], channels)
        }
    };
    let Some(first) = images.first() else { return Err(span.domain_error("invalid image: no frames")) };
    let (width, height, channels) = (first.width() as usize, first.height() as usize, usize::from(channels));
    let data = images.iter().flat_map(|image| match channels {
        1 => image.to_luma16().into_raw(),
        2 => image.to_luma_alpha16().into_raw(),
        3 => image.to_rgb16().into_raw(),
        _ => image.to_rgba16().into_raw(),
    });
    let shape = match images.len() { 1 if channels == 1 => vec![height, width], 1 => vec![height, width, channels], n => vec![n, height, width, channels] };
    Value::floats(shape, span.numeric().width, data.map(|v| f64::from(v) / 65535.).collect()).error_at(span, "image exceeds array limits")
}

/// `frames` encoded in `format`. Only PNG holds an animation.
fn encoded(frames: Frames, format: ImageFormat, span: &Context<'_>) -> Result<Vec<u8>, Error> {
    let failed = |e: &dyn std::fmt::Display| span.domain_error(format!("image encoding failed: {e}"));
    let mut bytes = Vec::new();
    let Frames { images, delays } = frames;
    let Some(delays) = delays else {
        // JPEG has no alpha channel.
        let image = match (format, images.into_iter().next().expect("a still picture is one frame")) {
            (ImageFormat::Jpeg, image) if image.color().has_color() => image.to_rgb8().into(),
            (ImageFormat::Jpeg, image) => image.to_luma8().into(),
            (_, image) => image,
        };
        image.write_to(&mut Cursor::new(&mut bytes), format).map_err(|e| failed(&e))?;
        return Ok(bytes);
    };
    if format != ImageFormat::Png { return Err(span.domain_error("only PNG holds an animation")); }
    let Some(first) = images.first() else { return Err(span.domain_error("an animation needs a frame")) };
    let mut encoder = png::Encoder::new(&mut bytes, first.width(), first.height());
    encoder.set_color(match first.color().channel_count() {
        1 => png::ColorType::Grayscale,
        2 => png::ColorType::GrayscaleAlpha,
        3 => png::ColorType::Rgb,
        _ => png::ColorType::Rgba,
    });
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_animated(images.len() as u32, 0).map_err(|e| failed(&e))?;
    let mut writer = encoder.write_header().map_err(|e| failed(&e))?;
    for (image, &seconds) in images.iter().zip(delays.iter().cycle()) {
        let (num, den) = ratio(seconds);
        writer.set_frame_delay(num, den).map_err(|e| failed(&e))?;
        writer.write_image_data(image.as_bytes()).map_err(|e| failed(&e))?;
    }
    writer.finish().map_err(|e| failed(&e))?;
    Ok(bytes)
}

/// `seconds` as an APNG frame delay, a ratio of 16-bit integers: exactly `1/n` for a whole rate `n`, otherwise in milliseconds, or in
/// whole seconds for a delay longer than 65 seconds.
fn ratio(seconds: f64) -> (u16, u16) {
    let max = f64::from(u16::MAX);
    let rate = (1. / seconds).round();
    if (1. ..=max).contains(&rate) && (rate * seconds - 1.).abs() < 1e-9 { return (1, rate as u16); }
    let ms = (seconds * 1000.).round();
    if ms <= max { (ms.max(1.) as u16, 1000) } else { (seconds.round().min(max) as u16, 1) }
}

/// A picture's frames, and the seconds each frame of an animation stays on screen: one number for every frame, or one per frame. A
/// still picture has one frame and no delays.
struct Frames { images: Vec<DynamicImage>, delays: Option<Vec<f64>> }

/// The frames that the numbers in `picture` describe, with each value clamped to 0..1. With `delays`, or with four axes, the first
/// axis is time.
fn frames(picture: &Value, delays: Option<Vec<f64>>, span: &Context<'_>) -> Result<Frames, Error> {
    let delays = delays.or_else(|| (picture.shape().len() == 4).then(|| vec![1. / DEFAULT_FPS]));
    let (count, frame) = match (&delays, picture.shape()) {
        (Some(_), [count, frame @ ..]) => (*count, frame),
        (None, frame) => (1, frame),
        (Some(_), []) => return Err(span.error(ErrorKind::Rank, "an animation has frames along its first axis")),
    };
    if delays.as_ref().is_some_and(|d| d.len() != 1 && d.len() != count) {
        return Err(span.error(ErrorKind::Length, "delay needs one number, or one for each frame"));
    }
    let (height, width, channels) = match *frame {
        [height, width] => (height, width, 1),
        [height, width, channels @ 1..=4] => (height, width, channels),
        _ => return Err(span.error(ErrorKind::Rank, "a picture has axes for rows, columns and up to four channels")),
    };
    let values = read_as::<f64>(picture).ok_or_else(|| span.domain_error("a picture holds real numbers"))?;
    let data: Vec<u8> = values.iter().map(|v| (v.clamp(0., 1.) * 255.).round() as u8).collect();
    let (size, width, height) = (height * width * channels, width as u32, height as u32);
    let images = (0..count)
        .map(|i| {
            let data = data[i * size..(i + 1) * size].to_vec();
            let image = match channels {
                1 => ImageBuffer::from_raw(width, height, data).map(DynamicImage::ImageLuma8),
                2 => ImageBuffer::from_raw(width, height, data).map(DynamicImage::ImageLumaA8),
                3 => ImageBuffer::from_raw(width, height, data).map(DynamicImage::ImageRgb8),
                _ => ImageBuffer::from_raw(width, height, data).map(DynamicImage::ImageRgba8),
            };
            image.expect("the shape gives the buffer's length")
        })
        .collect();
    Ok(Frames { images, delays })
}
