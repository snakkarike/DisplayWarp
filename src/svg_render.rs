/// Decode a PNG file (bytes) into RGBA pixels, width, and height.
pub fn png_to_rgba(png_data: &[u8]) -> (Vec<u8>, u32, u32) {
    let img = image::load_from_memory_with_format(png_data, image::ImageFormat::Png)
        .expect("failed to decode PNG")
        .into_rgba8();
    let (w, h) = img.dimensions();
    (img.into_raw(), w, h)
}
