use tiny_skia::{Pixmap, Color, Paint, Rect, PathBuilder, BlendMode, Transform, Stroke};
use crate::config::MonitorConfig;

fn push_rounded_rect(pb: &mut PathBuilder, rect: Rect, radius: f32) {
    let radius = radius.min(rect.width() / 2.0).min(rect.height() / 2.0);
    if radius <= 0.0 {
        pb.move_to(rect.x(), rect.y());
        pb.line_to(rect.right(), rect.y());
        pb.line_to(rect.right(), rect.bottom());
        pb.line_to(rect.x(), rect.bottom());
        pb.close();
        return;
    }

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

    let thickness = config.thickness.unwrap_or(16) as f32;
    let radius = config.radius.unwrap_or(24) as f32;
    
    let is_crt = config.preset.as_deref().unwrap_or("").to_lowercase() == "crt";

    if let Some(inner_rect) = Rect::from_xywh(
        thickness, 
        thickness, 
        width as f32 - 2.0 * thickness, 
        height as f32 - 2.0 * thickness
    ) {
        if inner_rect.width() > 0.0 && inner_rect.height() > 0.0 {
            
            // If CRT mode, draw a drop inner shadow and an outer highlight BEFORE carving the transparent hole, 
            // Wait, we can't draw *inside* the transparent hole after carving because we need the clear pixels for input passthrough.
            // But if we draw ON the bezel, the transparent hole is untouched!
            
            // A bezel highlight (drawn near the outer screen edge)
            if is_crt {
                let hw = config.highlight_width.unwrap_or(3.0);
                if hw > 0.0 {
                    if let Some(outer_rect) = Rect::from_xywh(hw/2.0, hw/2.0, width as f32 - hw, height as f32 - hw) {
                        let mut pb = PathBuilder::new();
                        push_rounded_rect(&mut pb, outer_rect, 0.0); // sharp outer corners
                        if let Some(path) = pb.finish() {
                            let mut stroke_paint = Paint::default();
                            stroke_paint.set_color(parse_hex_color(config.highlight_color.as_deref().unwrap_or("#FFFFFF22"), config.opacity.unwrap_or(1.0)));
                            stroke_paint.anti_alias = true;
                            let mut stroke = Stroke::default();
                            stroke.width = hw;
                            pixmap.stroke_path(&path, &stroke_paint, &stroke, Transform::identity(), None);
                        }
                    }
                }

                // Inner Shadow (drawn on the inner edge of the bezel)
                let sw = config.shadow_width.unwrap_or(12.0);
                if sw > 0.0 {
                    if let Some(shadow_rect) = Rect::from_xywh(
                        inner_rect.x() - sw / 2.0,
                        inner_rect.y() - sw / 2.0,
                        inner_rect.width() + sw,
                        inner_rect.height() + sw,
                    ) {
                        let mut pb = PathBuilder::new();
                        push_rounded_rect(&mut pb, shadow_rect, radius + (sw / 2.0));
                        if let Some(path) = pb.finish() {
                            let mut stroke_paint = Paint::default();
                            stroke_paint.set_color(parse_hex_color(config.shadow_color.as_deref().unwrap_or("#000000AA"), config.opacity.unwrap_or(1.0)));
                            stroke_paint.anti_alias = true;
                            let mut stroke = Stroke::default();
                            stroke.width = sw;
                            pixmap.stroke_path(&path, &stroke_paint, &stroke, Transform::identity(), None);
                        }
                    }
                }
            }

            // Finally, Erase the center completely transparent
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
    
    let mut data = pixmap.take();
    // Wayland's Argb8888 format is little-endian BGRA bytes. 
    for pixel in data.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
    data
}

fn parse_hex_color(hex: &str, alpha: f32) -> Color {
    let hex = hex.trim_start_matches('#');
    
    let r_str = hex.get(0..2).unwrap_or("00");
    let g_str = hex.get(2..4).unwrap_or("00");
    let b_str = hex.get(4..6).unwrap_or("00");
    
    // Support optional alpha channel in hex
    let a_str = hex.get(6..8).unwrap_or("FF");

    let r = u8::from_str_radix(r_str, 16).unwrap_or(0);
    let g = u8::from_str_radix(g_str, 16).unwrap_or(0);
    let b = u8::from_str_radix(b_str, 16).unwrap_or(0);
    let hex_a = u8::from_str_radix(a_str, 16).unwrap_or(255);
    
    Color::from_rgba8(r, g, b, (alpha * (hex_a as f32)) as u8)
}
