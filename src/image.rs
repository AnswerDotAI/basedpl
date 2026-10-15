use crate::{array::Kind, data, display, element::read_as, execution::Context, keyed, Error, ErrorAt, ErrorKind, Value};

/// Decoding and encoding of image files: ImageIO on Apple platforms, and the `image` and `png` crates elsewhere.
#[cfg_attr(target_vendor = "apple", path = "image/imageio.rs")]
#[cfg_attr(not(target_vendor = "apple"), path = "image/codecs.rs")]
mod codec;

/// The rate an animation plays at without an `fps` or `delay` option.
const DEFAULT_FPS: f64 = 24.;

/// The image file formats that `•image⁻¹` writes.
#[derive(Clone, Copy, PartialEq)]
enum Format { Png, Jpeg }

/// The samples of `count` frames, each of `height` rows of `width` pixels with `channels` samples: grey, or red, green and blue, then alpha where `channels` is even. `data` holds the frames in turn, each row by row.
struct Raster<T> { count: usize, height: usize, width: usize, channels: usize, data: Vec<T> }

impl<T> Raster<T> {
    /// The samples of frame `i`.
    fn frame(&self, i: usize) -> &[T] { let size = self.height * self.width * self.channels; &self.data[i * size..(i + 1) * size] }
}

/// `X •image Y` returns a picture: the image file that `Y` names, the bytes of one, or the numbers in `Y`. Files are PNG or JPEG. Apple platforms also read the other formats that ImageIO reads. A picture is numbers from 0 to 1 with axes for rows, columns and up to four channels, displayed as an image. An animation has frames along a first axis: a rank-4 array, or any array of pictures with the option `fps`, the frame rate, or `delay`, the seconds each frame stays on screen.
pub(crate) fn image(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let options = data::Options::new("•image", left, None, &["fps", "delay"], span)?;
    let delays = timing(&options, "•image", span)?;
    let picture = match keyed::name(right) {
        Some(path) => decoded(&span.read(&path)?, span)?, None if right.shape().len() == 1 => decoded(&data::bytes(right, span)?, span)?, None => right.clone()
    };
    layout(&picture, delays.clone(), span)?;
    let renderer = display::renderer("image-renderer", render);
    let renderer = match delays {
        Some(d) => crate::eval::before(Value::floats(vec![d.len()], span.numeric().width, d).error_at(span, "invalid delays")?, renderer, span.span)?,
        None => renderer,
    };
    picture.with_renderer(renderer).error_at(span, "invalid image")
}

/// `options •image⁻¹ Y` encodes the picture `Y` as bytes. The option `format` is `"png"`, the default, or `"jpeg"`. Plain text on the left gives the format alone. With the option `fps` or `delay`, or with four axes, `Y` is an animation, which becomes an animated PNG.
pub(crate) fn encode(left: Option<&Value>, right: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let options = data::Options::new("•image⁻¹", left, Some("format"), &["format", "fps", "delay"], span)?;
    let format = match options.text("format", Some("png"), span)?.to_lowercase().as_str() {
        "png" => Format::Png, "jpeg" => Format::Jpeg, _ => return Err(span.domain_error("•image⁻¹ encodes \"png\" or \"jpeg\""))
    };
    let (frames, delays) = frames(right, timing(&options, "•image⁻¹", span)?, span)?;
    data::byte_vector(encoded(frames, delays.as_deref(), format, span)?).error_at(span, "image exceeds array limits")
}

/// The seconds each frame stays on screen, from the option `delay` of `name`, or from `fps`. Either makes the first axis of a picture time. `delay` is one number for every frame, or one per frame, and sets the timing when both are given.
fn timing(options: &data::Options, name: &str, span: &Context<'_>) -> Result<Option<Vec<f64>>, Error> {
    let (key, delays) = match options.values.get("delay") {
        Some(d) => ("delay", (d.shape().len() <= 1).then(|| read_as::<f64>(d)).flatten().map(|d| d.into_owned())),
        None => match options.number("fps", span)? { Some(fps) => ("fps", Some(vec![1. / fps])), None => return Ok(None) },
    };
    match delays {
        Some(d) if !d.is_empty() && d.iter().all(|s| *s > 0. && s.is_finite()) => Ok(Some(d)),
        _ => Err(span.domain_error(format!("{name} {key} must be positive"))),
    }
}

/// Native builds send a PNG, animated for an animation. The browser build sends an animation the same way, and a still picture as `image/x-rgba` bytes, with the width as a parameter, for its page to draw on a canvas. `delays` are the seconds per frame that `•image`'s options set.
fn render(delays: Option<&Value>, picture: &Value, span: &Context<'_>) -> Result<Value, Error> {
    let delays = delays.map(|d| read_as::<f64>(d).expect("•image keeps its delays as numbers").into_owned());
    let (frames, delays) = frames(picture, delays, span)?;
    let (kind, bytes) = match delays {
        None if cfg!(web) => (format!("image/x-rgba;width={}", frames.width), rechannelled(&frames.data, frames.channels, 4, u8::MAX)),
        _ => ("image/png".into(), encoded(frames, delays.as_deref(), Format::Png, span)?),
    };
    let bytes = data::byte_vector(bytes).error_at(span, "image exceeds array limits")?;
    display::mime(&kind, bytes).error_at(span, "invalid image MIME bundle")
}

/// The picture in the encoded image `bytes`. An animated PNG with more than one frame gives its frames along a first axis, as a rank-4 array with a channel axis even for grey frames, so `•image⁻¹` encodes it back.
fn decoded(bytes: &[u8], span: &Context<'_>) -> Result<Value, Error> {
    let Raster { count, height, width, channels, data } = codec::decode(bytes).map_err(|e| span.domain_error(format!("invalid image: {e}")))?;
    let shape = match count { 1 if channels == 1 => vec![height, width], 1 => vec![height, width, channels], n => vec![n, height, width, channels] };
    Value::floats(shape, span.numeric().width, data.into_iter().map(|v| f64::from(v) / 65535.)).error_at(span, "image exceeds array limits")
}

/// `frames` encoded in `format`, as an animation with `delays`. Only PNG holds an animation. JPEG drops alpha.
fn encoded(mut frames: Raster<u8>, delays: Option<&[f64]>, format: Format, span: &Context<'_>) -> Result<Vec<u8>, Error> {
    if delays.is_some() && format != Format::Png { return Err(span.domain_error("only PNG holds an animation")); }
    if frames.count == 0 { return Err(span.domain_error("an animation needs a frame")); }
    if format == Format::Jpeg && frames.channels % 2 == 0 {
        frames.data = rechannelled(&frames.data, frames.channels, frames.channels - 1, 0);
        frames.channels -= 1;
    }
    codec::encode(&frames, delays, format).map_err(|e| span.domain_error(format!("image encoding failed: {e}")))
}

/// `data`'s pixels with `to` channels in place of `from`, as `rechannel_into` gives them.
fn rechannelled<T: Copy>(data: &[T], from: usize, to: usize, opaque: T) -> Vec<T> {
    let mut out = Vec::new();
    rechannel_into(&mut out, data, from, to, opaque, |x| x);
    out
}

/// Appends `data`'s pixels to `out` through `f`, with `to` channels in place of `from`. Grey repeats into red, green and blue, and red stands for grey. Alpha is dropped, or added as `opaque`.
fn rechannel_into<T: Copy, U>(out: &mut Vec<U>, data: &[T], from: usize, to: usize, opaque: T, f: impl Fn(T) -> U) {
    let colours = |channels: usize| if channels < 3 { 1 } else { 3 };
    out.reserve(data.len() / from * to);
    for pixel in data.chunks_exact(from) {
        out.extend((0..colours(to)).map(|k| f(pixel[if colours(from) == 1 { 0 } else { k }])));
        if to % 2 == 0 { out.push(f(if from % 2 == 0 { pixel[from - 1] } else { opaque })); }
    }
}

/// The frame count, height, width and channels that `picture` describes, and the seconds each frame of an animation stays on screen: one number for every frame, or one per frame. With `delays`, or with four axes, the first axis is time. It reads the picture's storage kind instead of its samples. A complex picture is read in full, because its imaginary parts must all be zero.
fn layout(picture: &Value, delays: Option<Vec<f64>>, span: &Context<'_>) -> Result<([usize; 4], Option<Vec<f64>>), Error> {
    let delays = delays.or_else(|| (picture.shape().len() == 4).then(|| vec![1. / DEFAULT_FPS]));
    let (count, frame) = match (&delays, picture.shape()) {
        (Some(_), [count, frame @ ..]) => (*count, frame), (None, frame) => (1, frame),
        (Some(_), []) => return Err(span.error(ErrorKind::Rank, "an animation has frames along its first axis"))
    };
    if delays.as_ref().is_some_and(|d| d.len() != 1 && d.len() != count) {
        return Err(span.error(ErrorKind::Length, "delay needs one number, or one for each frame"));
    }
    let (height, width, channels) = match *frame {
        [height, width] => (height, width, 1), [height, width, channels @ 1..=4] => (height, width, channels),
        _ => return Err(span.error(ErrorKind::Rank, "a picture has axes for rows, columns and up to four channels"))
    };
    let real = match picture.compact_kind() {
        Some(Kind::Complex(_)) => read_as::<f64>(picture).is_some(), Some(Kind::Character) | None => false, Some(_) => true
    };
    if !real { return Err(span.domain_error("a picture holds real numbers")); }
    Ok(([count, height, width, channels], delays))
}

/// The frames that the numbers in `picture` describe, as 8-bit samples of values clamped to 0..1, with the delays that `layout` gives.
fn frames(picture: &Value, delays: Option<Vec<f64>>, span: &Context<'_>) -> Result<(Raster<u8>, Option<Vec<f64>>), Error> {
    let ([count, height, width, channels], delays) = layout(picture, delays, span)?;
    let values = read_as::<f64>(picture).expect("`layout` checked that the picture holds real numbers");
    let data = values.iter().map(|v| (v.clamp(0., 1.) * 255.).round() as u8).collect();
    Ok((Raster { count, height, width, channels, data }, delays))
}

/// `seconds` as an APNG frame delay, a ratio of 16-bit integers: exactly `1/n` for a whole rate `n`, otherwise in milliseconds, or in whole seconds for a delay longer than 65 seconds.
fn ratio(seconds: f64) -> (u16, u16) {
    let max = f64::from(u16::MAX);
    let rate = (1. / seconds).round();
    if (1. ..=max).contains(&rate) && (rate * seconds - 1.).abs() < 1e-9 { return (1, rate as u16); }
    let ms = (seconds * 1000.).round();
    if ms <= max { (ms.max(1.) as u16, 1000) } else { (seconds.round().min(max) as u16, 1) }
}
