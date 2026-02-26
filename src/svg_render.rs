use resvg::tiny_skia;
use resvg::usvg;

/// Render an SVG (bytes) to RGBA pixels at the given width × height.
pub fn svg_to_rgba(svg_data: &[u8], width: u32, height: u32) -> Vec<u8> {
    let options = usvg::Options::default();
    let tree = usvg::Tree::from_data(svg_data, &options).expect("failed to parse SVG");

    let svg_size = tree.size();
    let sx = width as f32 / svg_size.width();
    let sy = height as f32 / svg_size.height();

    let mut pixmap = tiny_skia::Pixmap::new(width, height).unwrap();
    resvg::render(
        &tree,
        tiny_skia::Transform::from_scale(sx, sy),
        &mut pixmap.as_mut(),
    );

    // tiny_skia uses premultiplied RGBA; convert to straight RGBA.
    let mut data = pixmap.take();
    for chunk in data.chunks_exact_mut(4) {
        let a = chunk[3] as f32 / 255.0;
        if a > 0.0 {
            chunk[0] = (chunk[0] as f32 / a).min(255.0) as u8;
            chunk[1] = (chunk[1] as f32 / a).min(255.0) as u8;
            chunk[2] = (chunk[2] as f32 / a).min(255.0) as u8;
        }
    }
    data
}
