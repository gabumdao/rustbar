use tiny_skia::{Pixmap, Color, Paint, Rect, PathBuilder, BlendMode, Transform, Stroke, FillRule, Path};
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

fn get_inner_path(rect: Rect, radius: f32) -> Option<Path> {
    let mut pb = PathBuilder::new();
    push_rounded_rect(&mut pb, rect, radius);
    pb.finish()
}

use crate::config::{MonitorInnerShadowConfig, MonitorTextureConfig};

fn draw_inner_shadow(pixmap: &mut Pixmap, inner_rect: Rect, radius: f32, config: &MonitorInnerShadowConfig) {
    if !config.enabled.unwrap_or(false) { return; }
    let size = config.size.unwrap_or(0.0);
    if size <= 0.0 { return; }
    
    let path = match get_inner_path(inner_rect, radius) {
        Some(p) => p,
        None => return,
    };

    let color_hex = config.color.as_deref().unwrap_or("#000000");
    let base_color = parse_hex_color(color_hex, 1.0);
    // Number of steps for smoothness
    let steps = (size.ceil() * 2.0).clamp(16.0, 128.0) as usize;
    let base_opacity = config.opacity.unwrap_or(0.5);
    let softness = config.softness.unwrap_or(1.0).clamp(0.01, 2.0);
    let falloff_style = config.falloff.as_deref().unwrap_or("ease_out");

    // Create a temporary pixmap to draw the shadow, then composite using the inner path as mask
    let mut shadow_pixmap = match Pixmap::new(pixmap.width(), pixmap.height()) {
        Some(p) => p,
        None => return,
    };

    let mut paint = Paint::default();
    paint.anti_alias = true;

    for i in 0..steps {
        let t = (i as f32 + 0.5) / steps as f32; // 0 to 1
        
        // Width of this stroke step
        let w = size * t;
        
        // Opacity based on falloff/softness
        // We'll use a simple ease-out-like curve if requested
        let factor = match falloff_style {
            "ease_out" => (1.0 - t).powf(1.0 / softness),
            "ease_in" => 1.0 - t.powf(1.0 / softness),
            "linear" => 1.0 - t,
            _ => (1.0 - t).powf(1.0 / softness), // default to ease_out
        };
        
        // Since we are stacking strokes, we need to calculate the incremental alpha
        // But it's easier to think of each stroke as covering [0, w]
        // If we draw from t=1 down to 0, each stroke is wider.
        // Actually, let's use the property that a stroke of width 2*w centered on path
        // covers [0, w] inside and [0, w] outside.
        // We only care about the inside because we'll clip later.
        
        // To get a smooth gradient by stacking:
        // Alpha of this step's contribution.
        // If we want total alpha A(t) = base_opacity * factor(t)
        // And we're stacking N steps, then each step i adds some alpha.
        // Actually, more robust: draw each step i with alpha that makes it reach target.
        // Or just draw N strokes with small alpha and sum them up.
        
        let step_alpha = (base_opacity * factor) / (steps as f32 / 4.0);
        let step_color = Color::from_rgba(
            base_color.red(),
            base_color.green(),
            base_color.blue(),
            step_alpha.clamp(0.0, 1.0)
        ).unwrap_or(Color::BLACK);
        
        paint.set_color(step_color);
        let mut stroke = Stroke::default();
        stroke.width = w * 2.0;
        shadow_pixmap.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
    }

    // Now composite shadow_pixmap onto main pixmap, BUT ONLY inside the inner path
    let mut mask_pixmap = match Pixmap::new(pixmap.width(), pixmap.height()) {
        Some(p) => p,
        None => return,
    };
    let mut mask_paint = Paint::default();
    mask_paint.set_color(Color::WHITE);
    mask_paint.anti_alias = true;
    mask_pixmap.fill_path(&path, &mask_paint, FillRule::Winding, Transform::identity(), None);

    // Use alpha-only mask to composite
    let mask = tiny_skia::Mask::from_pixmap(mask_pixmap.as_ref(), tiny_skia::MaskType::Alpha);
    pixmap.draw_pixmap(0, 0, shadow_pixmap.as_ref(), &tiny_skia::PixmapPaint::default(), Transform::identity(), Some(&mask));
}

fn apply_texture(pixmap: &mut Pixmap, config: &MonitorTextureConfig) {
    if !config.enabled.unwrap_or(false) { return; }
    let strength = config.strength.unwrap_or(0.12).clamp(0.0, 1.0);
    if strength <= 0.0 { return; }
    
    let texture_type = config.texture_type.as_deref().unwrap_or("fine_noise");
    let scale = config.scale.unwrap_or(1.0).max(0.01);
    let contrast = config.contrast.unwrap_or(0.25);
    let seed = config.seed.unwrap_or(42);

    let width = pixmap.width();
    let height = pixmap.height();
    let pixels = pixmap.pixels_mut();

    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) as usize;
            
            // Map coordinates based on scale and type
            let (tx, ty) = match texture_type {
                "fine_noise" => (x as f32 / scale, y as f32 / scale),
                "grain" => (x as f32 / (scale * 2.0), y as f32 / (scale * 2.0)),
                "directional" => (x as f32 / (scale * 5.0), y as f32 / scale),
                "mottled" => (x as f32 / (scale * 8.0), y as f32 / (scale * 8.0)),
                _ => (x as f32 / scale, y as f32 / scale),
            };

            // Stable noise generation
            let n = squirrel_noise(tx.floor() as i32, ty.floor() as i32, seed);
            let mut val = (n as f32 / u32::MAX as f32) * 2.0 - 1.0; // -1.0 to 1.0
            
            // Apply contrast
            if contrast != 1.0 {
                val = val * contrast;
            }

            let p = &mut pixels[idx];
            let r = p.red() as f32 / 255.0;
            let g = p.green() as f32 / 255.0;
            let b = p.blue() as f32 / 255.0;
            let a = p.alpha() as f32 / 255.0;

            // Simple additive/subtractive blending for material variation
            // We modify the brightness. val is -1 to 1.
            let factor = 1.0 + (val * strength);
            
            let new_r = (r * factor).clamp(0.0, 1.0);
            let new_g = (g * factor).clamp(0.0, 1.0);
            let new_b = (b * factor).clamp(0.0, 1.0);

            if let Some(new_color) = Color::from_rgba(new_r, new_g, new_b, a) {
                *p = new_color.premultiply().to_color_u8();
            }
        }
    }
}

fn squirrel_noise(x: i32, y: i32, seed: u64) -> u32 {
    let mut n = (seed as u32).wrapping_add(x as u32).wrapping_mul(0xdeadbeef);
    n = n ^ (n >> 13);
    n = n.wrapping_add(y as u32).wrapping_mul(0x85ebca6b);
    n = n ^ (n >> 16);
    n = n.wrapping_mul(0xc2b2ae35);
    n = n ^ (n >> 13);
    n
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

    // 2. Apply texture before other effects
    if let Some(texture_config) = &config.texture {
        apply_texture(&mut pixmap, texture_config);
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
            if let Some(path) = get_inner_path(inner_rect, radius) {
                let mut clear_paint = Paint::default();
                clear_paint.blend_mode = BlendMode::Clear;
                clear_paint.anti_alias = true;
                pixmap.fill_path(&path, &clear_paint, FillRule::Winding, Transform::identity(), None);
            }

            // --- Inner Shadow ---
            if let Some(ish_config) = &config.inner_shadow {
                draw_inner_shadow(&mut pixmap, inner_rect, radius, ish_config);
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
            inner_shadow: None,
            texture: None,
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

        // Test with inner shadow
        let mut config_inner = config.clone();
        config_inner.inner_shadow = Some(MonitorInnerShadowConfig {
            enabled: Some(true),
            size: Some(10.0),
            opacity: Some(0.5),
            color: Some("#000000".to_string()),
            softness: Some(1.0),
            falloff: Some("ease_out".to_string()),
        });
        let pixels_inner = draw_frame(width, height, &config_inner);
        assert_eq!(pixels_inner.len(), (width * height * 4) as usize);
        assert_ne!(pixels_no_effects, pixels_inner);
    }

    #[test]
    fn test_draw_frame_texture() {
        let config = MonitorConfig {
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
            shadow_width: Some(0.0),
            shadow_color: None,
            highlight_width: Some(0.0),
            highlight_color: None,
            inner_shadow: None,
            texture: Some(MonitorTextureConfig {
                enabled: Some(true),
                texture_type: Some("fine_noise".to_string()),
                strength: Some(1.0), // Max strength for obvious difference
                scale: Some(1.0),
                contrast: Some(1.0),
                seed: Some(42),
            }),
            layer: None,
        };

        let width = 50;
        let height = 50;
        let pixels_textured = draw_frame(width, height, &config);
        
        let mut config_no_texture = config.clone();
        config_no_texture.texture = None;
        let pixels_plain = draw_frame(width, height, &config_no_texture);
        
        assert_ne!(pixels_textured, pixels_plain);
    }
}
