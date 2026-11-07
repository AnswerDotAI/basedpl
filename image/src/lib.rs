/// Decoding and encoding of image files: ImageIO on Apple platforms, and the `image` and `png` crates elsewhere.
#[cfg_attr(target_vendor = "apple", path = "imageio.rs")]
#[cfg_attr(not(target_vendor = "apple"), path = "codecs.rs")]
mod codec;
pub use codec::decode;

/// Encodes frames whose dimensions and channels match their samples.
pub fn encode(frames: &Raster<u8>, delays: Option<&[f64]>, format: Format) -> Result<Vec<u8>, String> {
    let size = [frames.count, frames.height, frames.width, frames.channels].into_iter().try_fold(1usize, usize::checked_mul);
    if !(1..=4).contains(&frames.channels) || size != Some(frames.data.len()) { return Err("invalid raster dimensions".into()); }
    codec::encode(frames, delays, format)
}

/// The image file formats that `•image⁻` writes.
#[derive(Clone, Copy, PartialEq)]
pub enum Format { Png, Jpeg }

/// The samples of `count` frames, each of `height` rows of `width` pixels with `channels` samples: grey, or red, green and blue, then alpha where `channels` is even. `data` holds the frames in turn, each row by row.
pub struct Raster<T> { pub count: usize, pub height: usize, pub width: usize, pub channels: usize, pub data: Vec<T> }

impl<T> Raster<T> {
    /// The samples of frame `i`.
    fn frame(&self, i: usize) -> &[T] { let size = self.height * self.width * self.channels; &self.data[i * size..(i + 1) * size] }
}

/// `data`'s pixels with `to` channels in place of `from`, as `rechannel_into` gives them.
pub fn rechannelled<T: Copy>(data: &[T], from: usize, to: usize, opaque: T) -> Vec<T> {
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

/// `seconds` as an APNG frame delay, a ratio of 16-bit integers: exactly `1/n` for a whole rate `n`, otherwise in milliseconds, or in whole seconds for a delay longer than 65 seconds.
fn ratio(seconds: f64) -> (u16, u16) {
    let max = f64::from(u16::MAX);
    let rate = (1. / seconds).round();
    if (1. ..=max).contains(&rate) && (rate * seconds - 1.).abs() < 1e-9 { return (1, rate as u16); }
    let ms = (seconds * 1000.).round();
    if ms <= max { (ms.max(1.) as u16, 1000) } else { (seconds.round().min(max) as u16, 1) }
}
