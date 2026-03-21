use tiny_skia::{Pixmap, Color, Paint, Rect, PathBuilder, BlendMode, Transform};
use crate::config::MonitorConfig;

fn push_rounded_rect(pb: &mut PathBuilder, rect: Rect, radius: f32) {
    let radius = radius.min(rect.width() / 2.0).min(rect.height() / 2.0);
    let x = rect.x();
    let y = rect.y();
    let w = rect.width();
    let h = rect.height();
    
    let kr = radius * 0.55228475; // kappa * radius

    pb.move_to(x + radius, y);
    pb.line_to(x + w - radius, y);
    pb.cubic_to(x + w - radius + kr, y, x + w, y + radius - kr, x + w, y + radius);
    pb.line_to(x + w, y + h - radius);
    pb.cubic_to(x + w, y + h - radius + kr, x + w - radius + kr, y + h, x + w - radius, y + h);
    pb.line_to(x + radius, y + h);
    pb.cubic_to(x + radius - kr, y + h, x, y + h - radius + kr, x, y + h - radius);
    pb.line_to(x, y + radius);
    pb.cubic_to(x, y + radius - kr, x + radius - kr, y, x + radius, y);
    pb.close();
}

pub fn draw_frame(width: u32, height: u32, config: &MonitorConfig) -> Vec<u8> {
    let mut pixmap = Pixmap::new(width, height).unwrap_or_else(|| Pixmap::new(1, 1).unwrap());
    
    let hex = config.color.as_deref().unwrap_or("#000000");
    let color = parse_hex_color(hex, config.opacity.unwrap_or(1.0));
    
    let mut paint = Paint::default();
    paint.set_color(color);
    paint.anti_alias = true;

    // 1. Fill entire background
    if let Some(full_rect) = Rect::from_xywh(0.0, 0.0, width as f32, height as f32) {
        pixmap.fill_rect(full_rect, &paint, Transform::identity(), None);
    }

    // 2. Erase the center
    let thickness = config.thickness.unwrap_or(16) as f32;
    let radius = config.radius.unwrap_or(24) as f32;
    
    if let Some(inner_rect) = Rect::from_xywh(
        thickness, 
        thickness, 
        width as f32 - 2.0 * thickness, 
        height as f32 - 2.0 * thickness
    ) {
        if inner_rect.width() > 0.0 && inner_rect.height() > 0.0 {
            let mut pb = PathBuilder::new();
            push_rounded_rect(&mut pb, inner_rect, radius);
            if let Some(path) = pb.finish() {
                let mut clear_paint = Paint::default();
                clear_paint.blend_mode = BlendMode::Clear;
                clear_paint.anti_alias = true;
                pixmap.fill_path(&path, &clear_paint, tiny_skia::FillRule::Winding, Transform::identity(), None);
            }
        }
    }
    
    let mut data = pixmap.take(); // returns Vec<u8> of standard RGBA 
    for pixel in data.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
    data
}

fn parse_hex_color(hex: &str, alpha: f32) -> Color {
    let hex = hex.trim_start_matches('#');
    let r = u8::from_str_radix(hex.get(0..2).unwrap_or("00"), 16).unwrap_or(0);
    let g = u8::from_str_radix(hex.get(2..4).unwrap_or("00"), 16).unwrap_or(0);
    let b = u8::from_str_radix(hex.get(4..6).unwrap_or("00"), 16).unwrap_or(0);
    Color::from_rgba8(r, g, b, (alpha * 255.0) as u8)
}
