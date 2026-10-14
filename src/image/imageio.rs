//! `•image`'s decoding and encoding through ImageIO. Decoding accepts every format that ImageIO reads. Reads return the sample
//! values that ImageIO stores for each image, with no colour conversion and with alpha not premultiplied. Drawing converts colours
//! and premultiplies alpha. The module draws only an opaque image whose samples it can't read directly, such as CMYK, into 8-bit
//! RGB. Encoding uses device grey or device RGB. In a PNG, ImageIO writes no colour chunk for device grey. It tags device RGB as
//! sRGB.
use super::{ratio, rechannel_into, rechannelled, Format, Raster};
use std::ffi::{c_char, c_void, CStr};
use std::ptr::{null, NonNull};

/// A borrowed Core Foundation object.
type Ref = *const c_void;

/// A rectangle in CoreGraphics' coordinates: its origin, then its size.
#[repr(C)]
struct Rect { x: f64, y: f64, width: f64, height: f64 }

#[link(name = "CoreFoundation", kind = "framework")]
extern "C" {
    fn CFRelease(object: Ref);
    fn CFDataCreate(allocator: Ref, bytes: *const u8, length: isize) -> Ref;
    fn CFDataCreateMutable(allocator: Ref, capacity: isize) -> Ref;
    fn CFDataGetBytePtr(data: Ref) -> *const u8;
    fn CFDataGetLength(data: Ref) -> isize;
    fn CFStringCreateWithCString(allocator: Ref, text: *const c_char, encoding: u32) -> Ref;
}

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGImageGetWidth(image: Ref) -> usize;
    fn CGImageGetHeight(image: Ref) -> usize;
    fn CGImageGetBitsPerComponent(image: Ref) -> usize;
    fn CGImageGetBitsPerPixel(image: Ref) -> usize;
    fn CGImageGetBytesPerRow(image: Ref) -> usize;
    fn CGImageGetBitmapInfo(image: Ref) -> u32;
    fn CGImageGetAlphaInfo(image: Ref) -> u32;
    fn CGImageGetColorSpace(image: Ref) -> Ref;
    fn CGImageGetDataProvider(image: Ref) -> Ref;
    fn CGImageCreate(width: usize, height: usize, bits_per_component: usize, bits_per_pixel: usize, bytes_per_row: usize, space: Ref, info: u32, provider: Ref, decode: *const f64, interpolate: bool, intent: i32) -> Ref;
    fn CGDataProviderCopyData(provider: Ref) -> Ref;
    fn CGDataProviderCreateWithCFData(data: Ref) -> Ref;
    fn CGColorSpaceGetModel(space: Ref) -> i32;
    fn CGColorSpaceGetBaseColorSpace(space: Ref) -> Ref;
    fn CGColorSpaceGetNumberOfComponents(space: Ref) -> usize;
    fn CGColorSpaceGetColorTableCount(space: Ref) -> usize;
    fn CGColorSpaceGetColorTable(space: Ref, table: *mut u8);
    fn CGColorSpaceCreateDeviceGray() -> Ref;
    fn CGColorSpaceCreateDeviceRGB() -> Ref;
    fn CGBitmapContextCreate(data: *mut c_void, width: usize, height: usize, bits_per_component: usize, bytes_per_row: usize, space: Ref, info: u32) -> Ref;
    fn CGContextDrawImage(context: Ref, rect: Rect, image: Ref);
}

#[link(name = "ImageIO", kind = "framework")]
extern "C" {
    fn CGImageSourceCreateWithData(data: Ref, options: Ref) -> Ref;
    fn CGImageSourceGetCount(source: Ref) -> usize;
    fn CGImageSourceCreateImageAtIndex(source: Ref, index: usize, options: Ref) -> Ref;
    fn CGImageDestinationCreateWithData(data: Ref, kind: Ref, count: usize, options: Ref) -> Ref;
    fn CGImageDestinationAddImage(destination: Ref, image: Ref, properties: Ref);
    fn CGImageDestinationFinalize(destination: Ref) -> bool;
}

// `CGColorSpaceModel` values.
const MONOCHROME: i32 = 0;
const RGB: i32 = 1;
const INDEXED: i32 = 5;
// `CGImageAlphaInfo` values.
const ALPHA_NONE: u32 = 0;
const ALPHA_LAST: u32 = 3;
const ALPHA_FIRST: u32 = 4;
const SKIP_LAST: u32 = 5;
const SKIP_FIRST: u32 = 6;
// `CGBitmapInfo` flags and byte orders.
const FLOAT: u32 = 1 << 8;
const ORDER: u32 = 7 << 12;
const ORDER_16_LITTLE: u32 = 1 << 12;
const ORDER_16_BIG: u32 = 3 << 12;
const ORDER_32_BIG: u32 = 4 << 12;
// `kCFStringEncodingUTF8`.
const UTF8: u32 = 0x0800_0100;

/// An owned Core Foundation object, released when dropped.
struct Cf(NonNull<c_void>);

impl Cf {
    /// Takes what a `Create` or `Copy` function returns, or gives `None` for the null that means it failed.
    fn new(object: Ref) -> Option<Self> { NonNull::new(object.cast_mut()).map(Self) }
    fn get(&self) -> Ref { self.0.as_ptr() }
}

impl Drop for Cf {
    fn drop(&mut self) { unsafe { CFRelease(self.get()) } }
}

/// The pixels of an image file in any format that ImageIO reads, widened to 16 bits. An animation's frames come composited. An animated
/// PNG's frames keep the channels that the file's header gives.
pub(super) fn decode(bytes: &[u8]) -> Result<Raster<u16>, String> {
    let source = Cf::new(unsafe { CGImageSourceCreateWithData(cf_data(bytes).get(), null()) }).ok_or("unreadable data")?;
    let count = unsafe { CGImageSourceGetCount(source.get()) };
    let frame = |i| Cf::new(unsafe { CGImageSourceCreateImageAtIndex(source.get(), i, null()) }).as_ref().and_then(|image| stored(image).or_else(|| drawn(image))).ok_or("unsupported pixel layout");
    if count == 0 { return Err("no frames".into()) }
    let mut raster = frame(0)?;
    let channels = if count > 1 { png_channels(bytes).unwrap_or(raster.channels) } else { raster.channels };
    if raster.channels != channels { raster.data = rechannelled(&raster.data, raster.channels, channels, u16::MAX) }
    raster.data.reserve((count - 1) * raster.data.len());
    for i in 1..count {
        let next = frame(i)?;
        if (next.height, next.width) != (raster.height, raster.width) { return Err("frames differ in size".into()); }
        if next.channels == channels { raster.data.extend_from_slice(&next.data) } else { rechannel_into(&mut raster.data, &next.data, next.channels, channels, u16::MAX, |x| x) }
    }
    Ok(Raster { count, channels, ..raster })
}

/// The samples that ImageIO stores for `image`, widened to 16 bits: grey or RGB, then alpha where it has alpha. A palette image's
/// indices go through its colour table. `None` for other colour models and layouts, such as a CMYK JPEG's.
fn stored(image: &Cf) -> Option<Raster<u16>> {
    let i = image.get();
    let (width, height, depth, bits, row) = unsafe { (CGImageGetWidth(i), CGImageGetHeight(i), CGImageGetBitsPerComponent(i), CGImageGetBitsPerPixel(i), CGImageGetBytesPerRow(i)) };
    let (info, layout, space) = unsafe { (CGImageGetBitmapInfo(i), CGImageGetAlphaInfo(i), CGImageGetColorSpace(i)) };
    let palette = match unsafe { CGColorSpaceGetModel(space) } {
        MONOCHROME | RGB => None,
        INDEXED if depth == 8 => Some(palette(space)?),
        _ => return None,
    };
    let colours = palette.as_ref().map_or_else(|| unsafe { CGColorSpaceGetNumberOfComponents(space) }, |p| p.0);
    let little = match (depth, info & ORDER) {
        (8, 0 | ORDER_32_BIG) | (16, 0 | ORDER_16_BIG) => false,
        (16, ORDER_16_LITTLE) => true,
        _ => return None,
    };
    // Each pixel's components: a palette index or the colours, and an alpha or a skipped component first or last.
    let indices = if palette.is_some() { 1 } else { colours };
    let step = indices + usize::from(layout != ALPHA_NONE);
    let (first, alpha) = match layout {
        ALPHA_NONE | SKIP_LAST => (0, None),
        SKIP_FIRST => (1, None),
        ALPHA_LAST => (0, Some(indices)),
        ALPHA_FIRST => (1, Some(0)),
        _ => return None,
    };
    if !matches!(colours, 1 | 3) || info & FLOAT != 0 || bits != step * depth { return None; }
    let provided = Cf::new(unsafe { CGDataProviderCopyData(CGImageGetDataProvider(i)) })?;
    let samples = cf_slice(&provided);
    let size = depth / 8;
    let pixels = width * step * size;
    if height > 0 && samples.len() < (height - 1) * row + pixels { return None; }
    let sample = |at: &[u8]| match (size, little) {
        (1, _) => u16::from(at[0]) * 257,
        (_, true) => u16::from_le_bytes([at[0], at[1]]),
        _ => u16::from_be_bytes([at[0], at[1]]),
    };
    let channels = colours + usize::from(alpha.is_some());
    let mut data = Vec::with_capacity(height * width * channels);
    for y in 0..height {
        for pixel in samples[y * row..][..pixels].chunks_exact(step * size) {
            match &palette {
                Some((n, table)) => data.extend(table.get(usize::from(pixel[first]) * n..)?.get(..*n)?.iter().map(|&v| u16::from(v) * 257)),
                None => data.extend((first..first + colours).map(|k| sample(&pixel[k * size..]))),
            }
            if let Some(a) = alpha { data.push(sample(&pixel[a * size..])); }
        }
    }
    Some(Raster { count: 1, height, width, channels, data })
}

/// `image` drawn into 8-bit RGB, for an opaque image in a colour model that `stored` doesn't read, such as a CMYK JPEG's. Drawing
/// converts its colours. An image with alpha gives `None`, because drawing would premultiply its alpha.
fn drawn(image: &Cf) -> Option<Raster<u16>> {
    let i = image.get();
    if !matches!(unsafe { CGImageGetAlphaInfo(i) }, ALPHA_NONE | SKIP_LAST | SKIP_FIRST) { return None; }
    let (width, height) = unsafe { (CGImageGetWidth(i), CGImageGetHeight(i)) };
    let mut pixels = vec![0u8; width * height * 4];
    let space = Cf::new(unsafe { CGColorSpaceCreateDeviceRGB() })?;
    let context = Cf::new(unsafe { CGBitmapContextCreate(pixels.as_mut_ptr().cast(), width, height, 8, width * 4, space.get(), SKIP_LAST) })?;
    unsafe { CGContextDrawImage(context.get(), Rect { x: 0., y: 0., width: width as f64, height: height as f64 }, i) };
    drop(context);
    let data = pixels.chunks_exact(4).flat_map(|p| p[..3].iter().map(|&v| u16::from(v) * 257)).collect();
    Some(Raster { count: 1, height, width, channels: 3, data })
}

/// The colour table of the indexed colour space `space`: the components of each entry, 1 for grey or 3 for RGB, and the entries.
fn palette(space: Ref) -> Option<(usize, Vec<u8>)> {
    let base = unsafe { CGColorSpaceGetBaseColorSpace(space) };
    if !matches!(unsafe { CGColorSpaceGetModel(base) }, MONOCHROME | RGB) { return None; }
    let n = unsafe { CGColorSpaceGetNumberOfComponents(base) };
    let mut table = vec![0; n * unsafe { CGColorSpaceGetColorTableCount(space) }];
    unsafe { CGColorSpaceGetColorTable(space, table.as_mut_ptr()) };
    Some((n, table))
}

/// The channels that a PNG's header gives its pixels: those of its colour type, with alpha where a `tRNS` chunk before the image data
/// adds transparency. `None` for other formats. ImageIO composites an RGB or palette animation's frames into RGBA.
fn png_channels(bytes: &[u8]) -> Option<usize> {
    if !bytes.starts_with(b"\x89PNG\r\n\x1a\n") { return None; }
    let channels = match bytes.get(25)? { 0 => 1, 2 | 3 => 3, 4 => 2, 6 => 4, _ => return None };
    let mut at = 8;
    while let Some(head) = bytes.get(at..at + 8) {
        match &head[4..] {
            b"IDAT" => break,
            b"tRNS" if channels % 2 == 1 => return Some(channels + 1),
            _ => at += 12 + u32::from_be_bytes(head[..4].try_into().ok()?) as usize,
        }
    }
    Some(channels)
}

/// `frames` as a PNG, animated with `delays`, or as a JPEG.
pub(super) fn encode(frames: &Raster<u8>, delays: Option<&[f64]>, format: Format) -> Result<Vec<u8>, String> {
    let out = Cf::new(unsafe { CFDataCreateMutable(null(), 0) }).expect("CFDataCreateMutable");
    let destination = Cf::new(unsafe { CGImageDestinationCreateWithData(out.get(), cf_string(uti(format)).get(), frames.count, null()) }).ok_or("ImageIO has no encoder")?;
    let space = Cf::new(unsafe { if frames.channels < 3 { CGColorSpaceCreateDeviceGray() } else { CGColorSpaceCreateDeviceRGB() } }).expect("device colour space");
    let layout = if frames.channels % 2 == 0 { ALPHA_LAST } else { ALPHA_NONE };
    let c = frames.channels;
    for i in 0..frames.count {
        let provider = Cf::new(unsafe { CGDataProviderCreateWithCFData(cf_data(frames.frame(i)).get()) }).expect("CGDataProviderCreateWithCFData");
        let image = Cf::new(unsafe { CGImageCreate(frames.width, frames.height, 8, 8 * c, frames.width * c, space.get(), layout, provider.get(), null(), false, 0) })
            .ok_or("an empty picture")?;
        unsafe { CGImageDestinationAddImage(destination.get(), image.get(), null()) };
    }
    if !unsafe { CGImageDestinationFinalize(destination.get()) } { return Err("ImageIO couldn't write it".into()); }
    drop(destination);
    let mut bytes = cf_slice(&out).to_vec();
    if let Some(delays) = delays { exact_delays(&mut bytes, delays) }
    Ok(bytes)
}

/// Rewrites the delay in each `fcTL` chunk of the animated PNG `png` as [`ratio`] gives it, with the chunk's checksum. ImageIO only
/// stores whole milliseconds. At 24 frames a second, each frame would last 41 ms instead of 1/24 s.
fn exact_delays(png: &mut [u8], delays: &[f64]) {
    let (mut at, mut frame) = (8, 0);
    while let Some(head) = png.get(at..at + 8) {
        let length = u32::from_be_bytes([head[0], head[1], head[2], head[3]]) as usize;
        if head[4..] == *b"fcTL" {
            let (num, den) = ratio(delays[frame % delays.len()]);
            png[at + 28..at + 32].copy_from_slice(&[num.to_be_bytes(), den.to_be_bytes()].concat());
            let mut crc = flate2::Crc::new();
            crc.update(&png[at + 4..at + 8 + length]);
            png[at + 8 + length..at + 12 + length].copy_from_slice(&crc.sum().to_be_bytes());
            frame += 1;
        }
        at += 12 + length;
    }
}

/// The uniform type identifier of `format`.
fn uti(format: Format) -> &'static CStr {
    match format {
        Format::Png => c"public.png",
        Format::Jpeg => c"public.jpeg",
    }
}

/// A `CFData` holding a copy of `bytes`.
fn cf_data(bytes: &[u8]) -> Cf {
    Cf::new(unsafe { CFDataCreate(null(), bytes.as_ptr(), bytes.len() as isize) }).expect("CFDataCreate")
}

/// The bytes that the `CFData` `data` holds.
fn cf_slice(data: &Cf) -> &[u8] {
    let length = unsafe { CFDataGetLength(data.get()) } as usize;
    if length == 0 { return &[]; }
    unsafe { std::slice::from_raw_parts(CFDataGetBytePtr(data.get()), length) }
}

fn cf_string(text: &CStr) -> Cf {
    Cf::new(unsafe { CFStringCreateWithCString(null(), text.as_ptr(), UTF8) }).expect("CFStringCreateWithCString")
}
