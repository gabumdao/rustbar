use tiny_skia::{Pixmap, Color, Paint, Rect, PathBuilder, BlendMode, Transform, Stroke, FillRule, Path, PixmapPaint};
use crate::config::{MonitorConfig, PatternConfig, MonitorPatternConfig};
use log::warn;

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

use crate::config::MonitorInnerShadowConfig;

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

fn draw_pattern(
    pixmap: &mut Pixmap,
    inner_rect: Rect,
    _radius: f32,
    config: &MonitorPatternConfig,
) {
    if !config.enabled.unwrap_or(false) { return; }
    let source = match &config.source {
        Some(s) => s,
        None => return,
    };

    let svg_path = PatternConfig::resolve_svg_path(source);
    if !svg_path.exists() {
        warn!("SVG pattern source not found: {}. Make sure it is in ~/.config/rustbar/svg or provide an absolute path.", svg_path.display());
        return;
    }

    let svg_data = match std::fs::read(&svg_path) {
        Ok(d) => d,
        Err(e) => {
            warn!("Failed to read SVG file {}: {}", svg_path.display(), e);
            return;
        }
    };

    let mut options = usvg::Options::default();
    // Default to the SVG directory for resources
    if let Some(parent) = svg_path.parent() {
        options.resources_dir = Some(parent.to_path_buf());
    }

    let tree = match usvg::Tree::from_data(&svg_data, &options) {
        Ok(t) => t,
        Err(e) => {
            warn!("Failed to parse SVG {}: {}", svg_path.display(), e);
            return;
        }
    };

    let transform_config = config.transform.clone().unwrap_or_default();
    let repeat_config = config.repeat.clone().unwrap_or_default();
    let sides_config = config.sides.clone().unwrap_or_default();
    let corners_config = config.corners.clone().unwrap_or_default();
    let _layout_config = config.layout.clone().unwrap_or_default();

    let motif_size = tree.size();
    let base_scale = transform_config.scale;
    let motif_w = motif_size.width() * base_scale;
    let motif_h = motif_size.height() * base_scale;

    if motif_w <= 0.0 || motif_h <= 0.0 {
        warn!("SVG motif has zero size after scaling: {}x{}", motif_w, motif_h);
        return;
    }

    // We'll render the SVG once into a pixmap and reuse it
    let mut motif_pixmap = match Pixmap::new(motif_w.ceil() as u32, motif_h.ceil() as u32) {
        Some(p) => p,
        None => return,
    };

    let motif_transform = Transform::from_scale(
        motif_w / motif_size.width(),
        motif_h / motif_size.height()
    );

    resvg::render(&tree, motif_transform, &mut motif_pixmap.as_mut());

    // Apply tint color if specified
    if let Some(tint_hex) = &config.color {
        let tint = parse_hex_color(tint_hex, 1.0);
        let mut paint = Paint::default();
        paint.set_color(tint);
        paint.blend_mode = BlendMode::SourceIn;
        if let Some(r) = Rect::from_xywh(0.0, 0.0, motif_w, motif_h) {
            motif_pixmap.fill_rect(r, &paint, Transform::identity(), None);
        }
    }

    let seed = config.seed.unwrap_or(0);
    let mut rng = SimpleRng::new(seed);

    let draw_at = |p: &mut Pixmap, x: f32, y: f32, angle: f32, rng: &mut SimpleRng| {
        let j_rotation = if repeat_config.jitter_rotation != 0.0 {
            rng.next_f32_range(-repeat_config.jitter_rotation, repeat_config.jitter_rotation)
        } else { 0.0 };
        
        let j_scale = if repeat_config.jitter_scale != 0.0 {
            1.0 + rng.next_f32_range(-repeat_config.jitter_scale, repeat_config.jitter_scale)
        } else { 1.0 };

        let mut ts = Transform::identity();
        ts = ts.post_translate(-motif_w / 2.0, -motif_h / 2.0);
        ts = ts.post_scale(
            if transform_config.flip_x { -j_scale } else { j_scale },
            if transform_config.flip_y { -j_scale } else { j_scale }
        );
        ts = ts.post_rotate(transform_config.rotation + angle + j_rotation);
        ts = ts.post_translate(x, y);

        let mut pp = PixmapPaint::default();
        pp.opacity = config.opacity.unwrap_or(1.0);
        pp.blend_mode = match config.blend.as_deref() {
            Some("multiply") => BlendMode::Multiply,
            Some("screen") => BlendMode::Screen,
            Some("overlay") => BlendMode::Overlay,
            Some("darken") => BlendMode::Darken,
            Some("lighten") => BlendMode::Lighten,
            _ => BlendMode::SourceOver,
        };

        p.draw_pixmap(0, 0, motif_pixmap.as_ref(), &pp, ts, None);
    };

    let t_top = (inner_rect.top()) as f32;
    let t_bottom = (pixmap.height() as f32 - inner_rect.bottom()) as f32;
    let t_left = (inner_rect.left()) as f32;
    let t_right = (pixmap.width() as f32 - inner_rect.right()) as f32;

    let spacing = repeat_config.spacing.max(1.0);
    let offset = repeat_config.offset;
    let corner_padding = corners_config.corner_padding;

    // Top edge
    if sides_config.top {
        let start_x = if corners_config.avoid_corners { t_left + corner_padding } else { 0.0 };
        let end_x = if corners_config.avoid_corners { pixmap.width() as f32 - t_right - corner_padding } else { pixmap.width() as f32 };
        let y = t_top / 2.0;
        
        let mut x = start_x + offset;
        while x < end_x {
            draw_at(pixmap, x, y, 0.0, &mut rng);
            let jitter = if repeat_config.jitter_spacing != 0.0 {
                rng.next_f32_range(-repeat_config.jitter_spacing, repeat_config.jitter_spacing)
            } else { 0.0 };
            x += spacing + jitter;
        }
    }

    // Bottom edge
    if sides_config.bottom {
        let start_x = if corners_config.avoid_corners { t_left + corner_padding } else { 0.0 };
        let end_x = if corners_config.avoid_corners { pixmap.width() as f32 - t_right - corner_padding } else { pixmap.width() as f32 };
        let y = pixmap.height() as f32 - (t_bottom / 2.0);
        
        let mut x = start_x + offset;
        while x < end_x {
            draw_at(pixmap, x, y, 180.0, &mut rng);
            let jitter = if repeat_config.jitter_spacing != 0.0 {
                rng.next_f32_range(-repeat_config.jitter_spacing, repeat_config.jitter_spacing)
            } else { 0.0 };
            x += spacing + jitter;
        }
    }

    // Left edge
    if sides_config.left {
        let start_y = if corners_config.avoid_corners { t_top + corner_padding } else { 0.0 };
        let end_y = if corners_config.avoid_corners { pixmap.height() as f32 - t_bottom - corner_padding } else { pixmap.height() as f32 };
        let x = t_left / 2.0;
        
        let mut y = start_y + offset;
        while y < end_y {
            draw_at(pixmap, x, y, -90.0, &mut rng);
            let jitter = if repeat_config.jitter_spacing != 0.0 {
                rng.next_f32_range(-repeat_config.jitter_spacing, repeat_config.jitter_spacing)
            } else { 0.0 };
            y += spacing + jitter;
        }
    }

    // Right edge
    if sides_config.right {
        let start_y = if corners_config.avoid_corners { t_top + corner_padding } else { 0.0 };
        let end_y = if corners_config.avoid_corners { pixmap.height() as f32 - t_bottom - corner_padding } else { pixmap.height() as f32 };
        let x = pixmap.width() as f32 - (t_right / 2.0);
        
        let mut y = start_y + offset;
        while y < end_y {
            draw_at(pixmap, x, y, 90.0, &mut rng);
            let jitter = if repeat_config.jitter_spacing != 0.0 {
                rng.next_f32_range(-repeat_config.jitter_spacing, repeat_config.jitter_spacing)
            } else { 0.0 };
            y += spacing + jitter;
        }
    }
}

struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self { state: seed.wrapping_add(1) }
    }

    fn next_u32(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1);
        (self.state >> 32) as u32
    }

    fn next_f32(&mut self) -> f32 {
        (self.next_u32() as f32) / (u32::MAX as f32)
    }

    fn next_f32_range(&mut self, min: f32, max: f32) -> f32 {
        min + (max - min) * self.next_f32()
    }
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

                // --- SVG Pattern ---
            if let Some(p_config) = &config.pattern {
                // We use a mask to ensure it only draws on the bezel
                if let Some(mut pattern_pixmap) = Pixmap::new(width, height) {
                    draw_pattern(&mut pattern_pixmap, inner_rect, radius, p_config);

                    if let Some(mut mask_pixmap) = Pixmap::new(width, height) {
                        // Create bezel mask: (Full - InnerCutout)
                        let mut pb = PathBuilder::new();
                        if let Some(full_rect) = Rect::from_xywh(0.0, 0.0, width as f32, height as f32) {
                            push_rounded_rect(&mut pb, full_rect, 0.0);
                            if let Some(full_path) = pb.finish() {
                                let mut paint = Paint::default();
                                paint.set_color(Color::WHITE);
                                mask_pixmap.fill_path(&full_path, &paint, FillRule::Winding, Transform::identity(), None);
                            }
                        }

                        if let Some(inner_path) = get_inner_path(inner_rect, radius) {
                            let mut clear_paint = Paint::default();
                            clear_paint.blend_mode = BlendMode::Clear;
                            mask_pixmap.fill_path(&inner_path, &clear_paint, FillRule::Winding, Transform::identity(), None);
                        }

                        let mask = tiny_skia::Mask::from_pixmap(mask_pixmap.as_ref(), tiny_skia::MaskType::Alpha);
                        pixmap.draw_pixmap(0, 0, pattern_pixmap.as_ref(), &tiny_skia::PixmapPaint::default(), Transform::identity(), Some(&mask));
                    }
                }
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
            pattern: None,
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
    fn test_draw_pattern_png() {
        use crate::config::{MonitorPatternConfig, PatternTransformConfig, PatternRepeatConfig, PatternSidesConfig, PatternCornersConfig, PatternLayoutConfig};
        let config = MonitorConfig {
            enabled: Some(true),
            thickness: Some(24),
            thickness_top: None,
            thickness_bottom: None,
            thickness_left: None,
            thickness_right: None,
            radius: Some(32),
            opacity: Some(0.9),
            color: Some("#1a1b26".to_string()),
            preset: None,
            shadow_width: Some(0.0),
            shadow_color: Some("#00000000".to_string()),
            highlight_width: Some(0.0),
            highlight_color: Some("#00000000".to_string()),
            inner_shadow: None,
            pattern: Some(MonitorPatternConfig {
                enabled: Some(true),
                source: Some("cross-svgrepo-com.svg".to_string()),
                mode: Some("repeat".to_string()),
                opacity: Some(0.4),
                color: Some("#f7768e".to_string()),
                blend: Some("normal".to_string()),
                seed: Some(42),
                transform: Some(PatternTransformConfig {
                    scale: 0.8,
                    rotation: 45.0,
                    flip_x: false,
                    flip_y: false,
                }),
                repeat: Some(PatternRepeatConfig {
                    spacing: 40.0,
                    offset: 10.0,
                    jitter_spacing: 5.0,
                    jitter_rotation: 15.0,
                    jitter_scale: 0.1,
                }),
                sides: Some(PatternSidesConfig {
                    top: true,
                    bottom: true,
                    left: true,
                    right: true,
                }),
                corners: Some(PatternCornersConfig {
                    avoid_corners: true,
                    corner_padding: 20.0,
                }),
                layout: Some(PatternLayoutConfig {
                    follow_edge_direction: true,
                    clip_to_frame: true,
                }),
            }),
            layer: None,
        };

        let width = 800;
        let height = 600;
        let pixels = draw_frame(width, height, &config);
        
        let mut pixmap = tiny_skia::Pixmap::new(width, height).unwrap();
        pixmap.data_mut().copy_from_slice(&pixels);
        for pixel in pixmap.data_mut().chunks_mut(4) {
            pixel.swap(0, 2);
        }
        pixmap.save_png("test_output.png").unwrap();
    }
}
