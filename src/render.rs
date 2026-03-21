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

    let t_base = config.thickness.unwrap_or(16) as f32;
    let t_top = config.thickness_top.map(|v| v as f32).unwrap_or(t_base);
    let t_bottom = config.thickness_bottom.map(|v| v as f32).unwrap_or(t_base);
    let t_left = config.thickness_left.map(|v| v as f32).unwrap_or(t_base);
    let t_right = config.thickness_right.map(|v| v as f32).unwrap_or(t_base);
    let radius = config.radius.unwrap_or(24) as f32;

    if let Some(inner_rect) = Rect::from_xywh(
        t_left, 
        t_top, 
        width as f32 - t_left - t_right, 
        height as f32 - t_top - t_bottom
    ) {
        if inner_rect.width() > 0.0 && inner_rect.height() > 0.0 {
            
            // Shadow and highlight effects BEFORE carving the transparent hole.
            // These are drawn ON the bezel.
            
            // A bezel highlight (drawn near the outer screen edge)
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

            // Inner Shadow (drawn on the inner edge of the bezel, fading outwards)
            let sw = config.shadow_width.unwrap_or(12.0);
            if sw > 0.0 {
                let shadow_c = parse_hex_color(config.shadow_color.as_deref().unwrap_or("#000000AA"), config.opacity.unwrap_or(1.0));
                let steps = sw.ceil() as usize;
                if steps > 0 {
                    let total_a = shadow_c.alpha() as f32 / 255.0;
                    let a_step = 1.0 - (1.0 - total_a).powf(1.0 / steps as f32);
                    
                    let step_color = tiny_skia::Color::from_rgba(
                        shadow_c.red() as f32 / 255.0,
                        shadow_c.green() as f32 / 255.0,
                        shadow_c.blue() as f32 / 255.0,
                        a_step,
                    ).unwrap_or_else(|| tiny_skia::Color::BLACK);

                    let mut stroke_paint = Paint::default();
                    stroke_paint.set_color(step_color);
                    stroke_paint.anti_alias = true;

                    for i in 0..steps {
                        let cur_sw = sw - (i as f32);
                        if cur_sw <= 0.0 { continue; }
                        
                        let mut pb = PathBuilder::new();
                        push_rounded_rect(&mut pb, inner_rect, radius);
                        if let Some(path) = pb.finish() {
                            let mut stroke = Stroke::default();
                            stroke.width = cur_sw * 2.0;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_draw_frame_basic() {
        let mut config = MonitorConfig {
            enabled: Some(true),
            thickness: Some(10),
            thickness_top: None,
            thickness_bottom: None,
            thickness_left: None,
            thickness_right: None,
            radius: Some(5),
            opacity: Some(1.0),
            color: Some("#FF0000".to_string()),
            preset: None,
            shadow_width: Some(5.0),
            shadow_color: Some("#000000AA".to_string()),
            highlight_width: Some(2.0),
            highlight_color: Some("#FFFFFF22".to_string()),
            layer: None,
        };

        let width = 100;
        let height = 100;
        let pixels = draw_frame(width, height, &config);
        
        assert_eq!(pixels.len(), (width * height * 4) as usize);
        
        // Test with 0 shadow/highlight
        config.shadow_width = Some(0.0);
        config.highlight_width = Some(0.0);
        let pixels_no_effects = draw_frame(width, height, &config);
        assert_eq!(pixels_no_effects.len(), (width * height * 4) as usize);
    }
}
