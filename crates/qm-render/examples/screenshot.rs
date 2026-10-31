//! Renders the preview headlessly to a PNG with a tiny software rasteriser,
//! so the output can be inspected without a display or GPU.
//!
//! ```sh
//! cargo run -p qm-render --example screenshot -- out.png [light|dark] [width] [height] [pixels_per_point] [scroll_line]
//! cargo run -p qm-render --release --example screenshot -- --bench
//! ```

mod shared;

use std::collections::HashMap;
use std::time::Instant;

use egui::epaint::{ClippedPrimitive, Primitive};
use egui::{
    vec2, Color32, Context, ImageData, Mesh, Pos2, RawInput, Rect, TextureId, ViewportId,
    ViewportInfo,
};
use qm_format::Document;
use qm_render::{PreviewOptions, Theme};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("--bench") {
        bench();
        return;
    }
    let path = args
        .first()
        .cloned()
        .unwrap_or_else(|| "preview.png".into());
    let theme = if args.get(1).map(String::as_str) == Some("dark") {
        Theme::dark()
    } else {
        Theme::light()
    };
    let width: f32 = args.get(2).and_then(|a| a.parse().ok()).unwrap_or(860.0);
    let height: f32 = args.get(3).and_then(|a| a.parse().ok()).unwrap_or(3200.0);
    let pixels_per_point: f32 = args.get(4).and_then(|a| a.parse().ok()).unwrap_or(1.0);
    let scroll_to_line = args.get(5).and_then(|a| a.parse().ok());
    let document = document_to_show();
    let image = render(
        &document,
        &theme,
        vec2(width, height),
        pixels_per_point,
        scroll_to_line,
    );
    image.write_png(&path).expect("write png");
    println!("wrote {path} ({}x{})", image.width, image.height);
}

/// The lecture example once the parser can read it, else the hand-built sample.
fn document_to_show() -> Document {
    let source_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../docs/examples/lecture.qm"
    );
    match std::fs::read_to_string(source_path) {
        Ok(source) if std::env::var_os("QM_SAMPLE").is_none() => qm_format::parse_document(&source),
        _ => shared::sample_document(),
    }
}

fn raw_input(size: egui::Vec2, pixels_per_point: f32) -> RawInput {
    let info = ViewportInfo {
        native_pixels_per_point: Some(pixels_per_point),
        ..Default::default()
    };
    RawInput {
        screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)),
        viewports: std::iter::once((ViewportId::ROOT, info)).collect(),
        ..Default::default()
    }
}

fn new_context() -> Context {
    let ctx = Context::default();
    let mut fonts = egui::FontDefinitions::default();
    qm_render::install_fonts(&mut fonts);
    ctx.set_fonts(fonts);
    ctx
}

fn render(
    document: &Document,
    theme: &Theme,
    size: egui::Vec2,
    pixels_per_point: f32,
    scroll_to_line: Option<usize>,
) -> Canvas {
    let ctx = new_context();
    let mut textures: HashMap<TextureId, Texture> = HashMap::new();
    let mut last = None;
    for frame in 0..4 {
        let options = PreviewOptions {
            scroll_to_line: scroll_to_line.filter(|_| frame == 1),
        };
        let output = ctx.run_ui(raw_input(size, pixels_per_point), |ui| {
            let _ = qm_render::show_document(ui, document, theme, &options);
        });
        for (id, delta) in &output.textures_delta.set {
            let ImageData::Color(image) = &delta.image;
            let texture = textures.entry(*id).or_insert_with(|| Texture {
                width: 0,
                height: 0,
                pixels: Vec::new(),
            });
            texture.apply(image.size, &image.pixels, delta.pos);
        }
        last = Some(output);
    }
    let output = last.expect("ran at least one frame");
    let primitives = ctx.tessellate(output.shapes, output.pixels_per_point);
    let mut canvas = Canvas::new(
        (size.x * pixels_per_point) as usize,
        (size.y * pixels_per_point) as usize,
        theme.surface,
    );
    for ClippedPrimitive {
        clip_rect,
        primitive,
    } in &primitives
    {
        if let Primitive::Mesh(mesh) = primitive {
            canvas.draw_mesh(mesh, *clip_rect, pixels_per_point, &textures);
        }
    }
    canvas
}

/// Frame times for a ~1000-line note: first (cold) frame and warm frames.
fn bench() {
    let document = shared::long_document(1000);
    let theme = Theme::light();
    let ctx = new_context();
    let input = || raw_input(vec2(1000.0, 900.0), 1.0);
    let run = || {
        let start = Instant::now();
        let _ = ctx.run_ui(input(), |ui| {
            let _ = qm_render::show_document(ui, &document, &theme, &PreviewOptions::default());
        });
        start.elapsed()
    };
    // Fonts are applied at the start of the first pass, so it lays out everything.
    let cold = run();
    println!("blocks: {}", document.blocks.len());
    println!("cold frame (fonts + full layout): {cold:?}");
    let warm: Vec<_> = (0..30).map(|_| run()).collect();
    println!(
        "warm frame (avg of 30): {:?}",
        warm.iter().sum::<std::time::Duration>() / warm.len() as u32
    );
}

struct Texture {
    width: usize,
    height: usize,
    pixels: Vec<Color32>,
}

impl Texture {
    fn apply(&mut self, size: [usize; 2], pixels: &[Color32], pos: Option<[usize; 2]>) {
        let Some([x0, y0]) = pos else {
            self.width = size[0];
            self.height = size[1];
            self.pixels = pixels.to_vec();
            return;
        };
        for y in 0..size[1] {
            for x in 0..size[0] {
                if x0 + x < self.width && y0 + y < self.height {
                    self.pixels[(y0 + y) * self.width + x0 + x] = pixels[y * size[0] + x];
                }
            }
        }
    }

    fn sample(&self, u: f32, v: f32) -> [f32; 4] {
        if self.width == 0 {
            return [1.0; 4];
        }
        let x = ((u * self.width as f32) as usize).min(self.width - 1);
        let y = ((v * self.height as f32) as usize).min(self.height - 1);
        to_floats(self.pixels[y * self.width + x])
    }
}

fn to_floats(c: Color32) -> [f32; 4] {
    let [r, g, b, a] = c.to_array();
    [
        r as f32 / 255.0,
        g as f32 / 255.0,
        b as f32 / 255.0,
        a as f32 / 255.0,
    ]
}

struct Canvas {
    width: usize,
    height: usize,
    /// Premultiplied, gamma-space RGBA (as egui's own backends blend).
    pixels: Vec<[f32; 4]>,
}

impl Canvas {
    fn new(width: usize, height: usize, background: Color32) -> Canvas {
        Canvas {
            width,
            height,
            pixels: vec![to_floats(background); width * height],
        }
    }

    fn draw_mesh(
        &mut self,
        mesh: &Mesh,
        clip: Rect,
        ppp: f32,
        textures: &HashMap<TextureId, Texture>,
    ) {
        let texture = textures.get(&mesh.texture_id);
        for triangle in mesh.indices.chunks_exact(3) {
            let v = [triangle[0], triangle[1], triangle[2]].map(|i| &mesh.vertices[i as usize]);
            let p = v.map(|v| [v.pos.x * ppp, v.pos.y * ppp]);
            let area = edge(p[0], p[1], p[2]);
            if area.abs() < 1e-9 {
                continue;
            }
            let min_x = p
                .iter()
                .map(|q| q[0])
                .fold(f32::INFINITY, f32::min)
                .max(clip.min.x * ppp)
                .max(0.0) as usize;
            let max_x = p
                .iter()
                .map(|q| q[0])
                .fold(f32::NEG_INFINITY, f32::max)
                .min(clip.max.x * ppp)
                .min(self.width as f32)
                .ceil() as usize;
            let min_y = p
                .iter()
                .map(|q| q[1])
                .fold(f32::INFINITY, f32::min)
                .max(clip.min.y * ppp)
                .max(0.0) as usize;
            let max_y = p
                .iter()
                .map(|q| q[1])
                .fold(f32::NEG_INFINITY, f32::max)
                .min(clip.max.y * ppp)
                .min(self.height as f32)
                .ceil() as usize;
            let colors = v.map(|v| to_floats(v.color));
            for y in min_y..max_y {
                for x in min_x..max_x {
                    let q = [x as f32 + 0.5, y as f32 + 0.5];
                    let w = [
                        edge(p[1], p[2], q) / area,
                        edge(p[2], p[0], q) / area,
                        edge(p[0], p[1], q) / area,
                    ];
                    if w.iter().any(|&w| w < 0.0) {
                        continue;
                    }
                    let u = w[0] * v[0].uv.x + w[1] * v[1].uv.x + w[2] * v[2].uv.x;
                    let t = w[0] * v[0].uv.y + w[1] * v[1].uv.y + w[2] * v[2].uv.y;
                    let texel = texture.map_or([1.0; 4], |tex| tex.sample(u, t));
                    let mut source = [0.0; 4];
                    for c in 0..4 {
                        source[c] =
                            (w[0] * colors[0][c] + w[1] * colors[1][c] + w[2] * colors[2][c])
                                * texel[c];
                    }
                    let destination = &mut self.pixels[y * self.width + x];
                    for c in 0..4 {
                        destination[c] = source[c] + destination[c] * (1.0 - source[3]);
                    }
                }
            }
        }
    }

    fn write_png(&self, path: &str) -> std::io::Result<()> {
        let mut raw = Vec::with_capacity((self.width * 3 + 1) * self.height);
        for row in self.pixels.chunks(self.width) {
            raw.push(0); // no filter
            for pixel in row {
                raw.extend(
                    pixel[..3]
                        .iter()
                        .map(|c| (c.clamp(0.0, 1.0) * 255.0).round() as u8),
                );
            }
        }
        let mut png = b"\x89PNG\r\n\x1a\n".to_vec();
        let mut header = Vec::new();
        header.extend((self.width as u32).to_be_bytes());
        header.extend((self.height as u32).to_be_bytes());
        header.extend([8, 2, 0, 0, 0]); // 8-bit RGB
        write_chunk(&mut png, b"IHDR", &header);
        write_chunk(&mut png, b"IDAT", &zlib_stored(&raw));
        write_chunk(&mut png, b"IEND", &[]);
        std::fs::write(path, png)
    }
}

fn edge(a: [f32; 2], b: [f32; 2], c: [f32; 2]) -> f32 {
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
}

fn write_chunk(png: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    png.extend((data.len() as u32).to_be_bytes());
    let start = png.len();
    png.extend(kind);
    png.extend(data);
    let crc = crc32(&png[start..]);
    png.extend(crc.to_be_bytes());
}

/// Uncompressed ("stored") zlib stream.
fn zlib_stored(data: &[u8]) -> Vec<u8> {
    let mut out = vec![0x78, 0x01];
    let blocks: Vec<&[u8]> = data.chunks(65_535).collect();
    for (i, block) in blocks.iter().enumerate() {
        out.push(u8::from(i + 1 == blocks.len()));
        let len = block.len() as u16;
        out.extend(len.to_le_bytes());
        out.extend((!len).to_le_bytes());
        out.extend(*block);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &byte in data {
        a = (a + u32::from(byte)) % 65_521;
        b = (b + a) % 65_521;
    }
    out.extend(((b << 16) | a).to_be_bytes());
    out
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &byte in data {
        crc ^= u32::from(byte);
        for _ in 0..8 {
            crc = if crc & 1 == 1 {
                (crc >> 1) ^ 0xEDB8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}
