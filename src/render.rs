use crate::config::{
    MonitorConfig, MonitorFrameSidesConfig, MonitorGradientConfig, MonitorPatternConfig,
    PatternConfig,
};
use log::warn;
use std::{
    collections::HashMap,
    fs,
    path::{Path as FsPath, PathBuf},
    sync::{Mutex, OnceLock},
    time::SystemTime,
};
use tiny_skia::{
    BlendMode, Color, FillRule, FilterQuality, GradientStop, LinearGradient, Paint, Path,
    PathBuilder, Pixmap, PixmapPaint, Point, Rect, SpreadMode, Stroke, Transform,
};

#[derive(Clone)]
struct SvgCacheEntry {
    modified: Option<SystemTime>,
    result: Result<usvg::Tree, SvgCacheFailure>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum SvgCacheFailure {
    Missing,
    Read(String),
    Parse(String),
}

fn svg_cache() -> &'static Mutex<HashMap<PathBuf, SvgCacheEntry>> {
    static SVG_CACHE: OnceLock<Mutex<HashMap<PathBuf, SvgCacheEntry>>> = OnceLock::new();
    SVG_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn svg_modified_time(path: &FsPath) -> Option<SystemTime> {
    fs::metadata(path)
        .ok()
        .and_then(|metadata| metadata.modified().ok())
}

fn load_svg_tree(path: &FsPath) -> Result<usvg::Tree, SvgCacheFailure> {
    let modified = svg_modified_time(path);

    if let Some(entry) = svg_cache().lock().unwrap().get(path).cloned() {
        if entry.modified == modified {
            return entry.result;
        }
    }

    let result = if !path.exists() {
        warn!(
            "SVG pattern source not found: {}. Make sure it is in ~/.config/rustbar/svg or provide an absolute path.",
            path.display()
        );
        Err(SvgCacheFailure::Missing)
    } else {
        match fs::read(path) {
            Ok(svg_data) => {
                let mut options = usvg::Options::default();
                if let Some(parent) = path.parent() {
                    options.resources_dir = Some(parent.to_path_buf());
                }

                match usvg::Tree::from_data(&svg_data, &options) {
                    Ok(tree) => Ok(tree),
                    Err(error) => {
                        let message = error.to_string();
                        warn!("Failed to parse SVG {}: {}", path.display(), message);
                        Err(SvgCacheFailure::Parse(message))
                    }
                }
            }
            Err(error) => {
                let message = error.to_string();
                warn!("Failed to read SVG file {}: {}", path.display(), message);
                Err(SvgCacheFailure::Read(message))
            }
        }
    };

    svg_cache().lock().unwrap().insert(
        path.to_path_buf(),
        SvgCacheEntry {
            modified,
            result: result.clone(),
        },
    );

    result
}

#[cfg(test)]
fn clear_svg_cache() {
    svg_cache().lock().unwrap().clear();
}

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
    pb.cubic_to(
        x + w - radius + kr,
        y,
        x + w,
        y + radius - kr,
        x + w,
        y + radius,
    );
    pb.line_to(x + w, y + h - radius);
    pb.cubic_to(
        x + w,
        y + h - radius + kr,
        x + w - radius + kr,
        y + h,
        x + w - radius,
        y + h,
    );
    pb.line_to(x + radius, y + h);
    pb.cubic_to(
        x + radius - kr,
        y + h,
        x,
        y + h - radius + kr,
        x,
        y + h - radius,
    );
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

fn draw_inner_shadow(
    pixmap: &mut Pixmap,
    inner_rect: Rect,
    radius: f32,
    config: &MonitorInnerShadowConfig,
) {
    if !config.enabled.unwrap_or(false) {
        return;
    }
    let size = config.size.unwrap_or(0.0);
    if size <= 0.0 {
        return;
    }

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
            step_alpha.clamp(0.0, 1.0),
        )
        .unwrap_or(Color::BLACK);

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
    mask_pixmap.fill_path(
        &path,
        &mask_paint,
        FillRule::Winding,
        Transform::identity(),
        None,
    );

    // Use alpha-only mask to composite
    let mask = tiny_skia::Mask::from_pixmap(mask_pixmap.as_ref(), tiny_skia::MaskType::Alpha);
    pixmap.draw_pixmap(
        0,
        0,
        shadow_pixmap.as_ref(),
        &tiny_skia::PixmapPaint::default(),
        Transform::identity(),
        Some(&mask),
    );
}

fn draw_pattern(
    pixmap: &mut Pixmap,
    outer_rect: Rect,
    inner_rect: Rect,
    radius: f32,
    config: &MonitorPatternConfig,
) {
    if !config.enabled.unwrap_or(false) {
        return;
    }
    let source = match &config.source {
        Some(s) => s,
        None => return,
    };

    let svg_path = PatternConfig::resolve_svg_path(source);
    let tree = match load_svg_tree(&svg_path) {
        Ok(tree) => tree,
        Err(_) => return,
    };

    let transform_config = config.transform.clone().unwrap_or_default();
    let repeat_config = config.repeat.clone().unwrap_or_default();
    let sides_config = config.sides.clone().unwrap_or_default();
    let corners_config = config.corners.clone().unwrap_or_default();
    let layout_config = config.layout.clone().unwrap_or_default();

    let t_top = inner_rect.top() - outer_rect.top();
    let t_bottom = outer_rect.bottom() - inner_rect.bottom();
    let t_left = inner_rect.left() - outer_rect.left();
    let t_right = outer_rect.right() - inner_rect.right();

    let motif_size = tree.size();
    let base_extent = [
        (sides_config.top, t_top),
        (sides_config.bottom, t_bottom),
        (sides_config.left, t_left),
        (sides_config.right, t_right),
    ]
    .into_iter()
    .filter_map(|(enabled, thickness)| {
        if enabled && thickness > 0.0 {
            Some(thickness)
        } else {
            None
        }
    })
    .fold(f32::INFINITY, f32::min);
    let base_extent = if base_extent.is_finite() {
        base_extent
    } else {
        24.0
    };
    let target_extent = (base_extent * transform_config.scale.max(0.01)).max(1.0);
    let intrinsic_max = motif_size.width().max(motif_size.height()).max(1.0);
    let motif_scale = target_extent / intrinsic_max;
    let motif_w = motif_size.width() * motif_scale;
    let motif_h = motif_size.height() * motif_scale;

    if motif_w <= 0.0 || motif_h <= 0.0 {
        warn!(
            "SVG motif has zero size after scaling: {}x{}",
            motif_w, motif_h
        );
        return;
    }

    // Render the SVG slightly oversized, then downsample when stamping to keep edges smooth.
    let oversample = 2.0;
    let mut motif_pixmap = match Pixmap::new(
        (motif_w * oversample).ceil() as u32,
        (motif_h * oversample).ceil() as u32,
    ) {
        Some(p) => p,
        None => return,
    };

    let motif_transform = Transform::from_scale(
        motif_pixmap.width() as f32 / motif_size.width(),
        motif_pixmap.height() as f32 / motif_size.height(),
    );

    resvg::render(&tree, motif_transform, &mut motif_pixmap.as_mut());

    let tint_color = config
        .color
        .as_ref()
        .map(|tint_hex| parse_hex_color(tint_hex, config.opacity.unwrap_or(1.0)));
    let target_rect = Rect::from_xywh(0.0, 0.0, pixmap.width() as f32, pixmap.height() as f32);

    let seed = config.seed.unwrap_or(0);
    let mut rng = SimpleRng::new(seed);

    let draw_at = |p: &mut Pixmap, x: f32, y: f32, angle: f32, rng: &mut SimpleRng| {
        let j_rotation = if repeat_config.jitter_rotation != 0.0 {
            rng.next_f32_range(
                -repeat_config.jitter_rotation,
                repeat_config.jitter_rotation,
            )
        } else {
            0.0
        };

        let j_scale = if repeat_config.jitter_scale != 0.0 {
            1.0 + rng.next_f32_range(-repeat_config.jitter_scale, repeat_config.jitter_scale)
        } else {
            1.0
        };

        let scale_x = if transform_config.flip_x {
            -j_scale
        } else {
            j_scale
        };
        let scale_y = if transform_config.flip_y {
            -j_scale
        } else {
            j_scale
        };
        let mut ts = Transform::from_translate(
            -(motif_pixmap.width() as f32) / 2.0,
            -(motif_pixmap.height() as f32) / 2.0,
        );
        ts = ts.post_scale(scale_x / oversample, scale_y / oversample);
        ts = ts.post_rotate(transform_config.rotation + angle + j_rotation);
        ts = ts.post_translate(x, y);

        let blend_mode = match config.blend.as_deref() {
            Some("multiply") => BlendMode::Multiply,
            Some("screen") => BlendMode::Screen,
            Some("overlay") => BlendMode::Overlay,
            Some("darken") => BlendMode::Darken,
            Some("lighten") => BlendMode::Lighten,
            _ => BlendMode::SourceOver,
        };

        let Some(mut instance_pixmap) = Pixmap::new(p.width(), p.height()) else {
            return;
        };

        let mut base_paint = PixmapPaint::default();
        base_paint.opacity = 1.0;
        base_paint.quality = FilterQuality::Bicubic;
        instance_pixmap.draw_pixmap(0, 0, motif_pixmap.as_ref(), &base_paint, ts, None);

        if let Some(tint) = tint_color {
            let Some(target_rect) = target_rect else {
                return;
            };
            let mask =
                tiny_skia::Mask::from_pixmap(instance_pixmap.as_ref(), tiny_skia::MaskType::Alpha);
            let mut paint = Paint::default();
            paint.set_color(tint);
            paint.blend_mode = blend_mode;
            paint.anti_alias = true;
            p.fill_rect(target_rect, &paint, Transform::identity(), Some(&mask));
        } else {
            let mut pp = PixmapPaint::default();
            pp.opacity = config.opacity.unwrap_or(1.0);
            pp.blend_mode = blend_mode;
            pp.quality = FilterQuality::Bicubic;
            p.draw_pixmap(
                0,
                0,
                instance_pixmap.as_ref(),
                &pp,
                Transform::identity(),
                None,
            );
        }
    };

    let spacing = repeat_config.spacing.max(1.0);
    let offset = repeat_config.offset;
    let corner_padding = corners_config.corner_padding;
    let mode = config.mode.as_deref().unwrap_or("repeat");
    let edge_angle = |angle: f32| {
        if layout_config.follow_edge_direction {
            angle
        } else {
            0.0
        }
    };
    let diagonal_mode = matches!(mode, "diagonal" | "diagonal_repeat" | "diagonal_lines");
    let random_mode = matches!(mode, "random" | "scatter");
    let directional_mode = matches!(mode, "horizontal" | "vertical" | "checker" | "diamond");
    let overlap_padding = layout_config.overlap_padding.max(0.0);
    let safe_spacing = if layout_config.prevent_overlap {
        spacing.max(motif_w.max(motif_h) + overlap_padding)
    } else {
        spacing
    };
    let safe_row_step = if layout_config.prevent_overlap {
        motif_h.max(1.0) + overlap_padding
    } else {
        safe_spacing
    };
    let safe_col_step = if layout_config.prevent_overlap {
        motif_w.max(1.0) + overlap_padding
    } else {
        safe_spacing
    };

    let band_ranges = [
        (
            sides_config.top,
            0.0,
            if corners_config.avoid_corners {
                t_left + corner_padding
            } else {
                0.0
            },
            t_top,
            if corners_config.avoid_corners {
                pixmap.width() as f32 - t_right - corner_padding
            } else {
                pixmap.width() as f32
            },
            0.0,
        ),
        (
            sides_config.bottom,
            pixmap.height() as f32 - t_bottom,
            if corners_config.avoid_corners {
                t_left + corner_padding
            } else {
                0.0
            },
            pixmap.height() as f32,
            if corners_config.avoid_corners {
                pixmap.width() as f32 - t_right - corner_padding
            } else {
                pixmap.width() as f32
            },
            180.0,
        ),
        (
            sides_config.left,
            0.0,
            0.0,
            pixmap.height() as f32,
            t_left,
            -90.0,
        ),
        (
            sides_config.right,
            0.0,
            pixmap.width() as f32 - t_right,
            pixmap.height() as f32,
            pixmap.width() as f32,
            90.0,
        ),
    ];

    for (enabled, y_min, x_min, y_max, x_max, side_angle) in band_ranges {
        if !enabled {
            continue;
        }

        let width = (x_max - x_min).max(0.0);
        let height = (y_max - y_min).max(0.0);
        if width <= 0.0 || height <= 0.0 {
            continue;
        }

        let center_x = (x_min + x_max) / 2.0;
        let center_y = (y_min + y_max) / 2.0;

        if mode == "single" {
            draw_at(pixmap, center_x, center_y, edge_angle(side_angle), &mut rng);
            continue;
        }

        if random_mode {
            let count = ((width.max(height)) / safe_spacing).ceil().max(1.0) as usize;
            for _ in 0..count {
                let x = rng.next_f32_range(x_min, x_max.max(x_min + 1.0));
                let y = rng.next_f32_range(y_min, y_max.max(y_min + 1.0));
                draw_at(pixmap, x, y, edge_angle(side_angle), &mut rng);
            }
            continue;
        }

        if diagonal_mode {
            let angle = layout_config.angle;
            let theta = angle.to_radians();
            let dir_x = theta.cos();
            let dir_y = theta.sin();
            let normal_x = -dir_y;
            let normal_y = dir_x;
            let draw_angle = if layout_config.follow_edge_direction {
                angle
            } else {
                0.0
            };
            let line_footprint = motif_w * normal_x.abs() + motif_h * normal_y.abs();
            let line_step = if layout_config.prevent_overlap {
                safe_spacing.max(line_footprint + overlap_padding)
            } else {
                safe_spacing
            }
            .max(1.0);
            let along_padding = motif_w.max(motif_h) + overlap_padding;
            let corners = [
                (x_min, y_min),
                (x_max, y_min),
                (x_min, y_max),
                (x_max, y_max),
            ];
            let (mut min_along, mut max_along) = (f32::INFINITY, f32::NEG_INFINITY);
            let (mut min_normal, mut max_normal) = (f32::INFINITY, f32::NEG_INFINITY);
            for (x, y) in corners {
                let along = x * dir_x + y * dir_y;
                let normal = x * normal_x + y * normal_y;
                min_along = min_along.min(along);
                max_along = max_along.max(along);
                min_normal = min_normal.min(normal);
                max_normal = max_normal.max(normal);
            }

            let start_along = (((min_along - along_padding - offset) / safe_spacing).floor()
                * safe_spacing)
                + offset;
            let end_along = (((max_along + along_padding - offset) / safe_spacing).ceil()
                * safe_spacing)
                + offset;
            let start_normal = ((min_normal - line_step) / line_step).floor() * line_step;
            let end_normal = ((max_normal + line_step) / line_step).ceil() * line_step;
            let draw_margin = along_padding;

            let mut normal = start_normal;
            while normal <= end_normal {
                let mut along = start_along;
                while along <= end_along {
                    let x = dir_x * along + normal_x * normal;
                    let y = dir_y * along + normal_y * normal;
                    if x >= x_min - draw_margin
                        && x <= x_max + draw_margin
                        && y >= y_min - draw_margin
                        && y <= y_max + draw_margin
                    {
                        draw_at(pixmap, x, y, draw_angle, &mut rng);
                    }
                    let jitter = if repeat_config.jitter_spacing != 0.0 {
                        rng.next_f32_range(
                            -repeat_config.jitter_spacing,
                            repeat_config.jitter_spacing,
                        )
                    } else {
                        0.0
                    };
                    along += (safe_spacing + jitter).max(1.0);
                }
                normal += line_step;
            }
            continue;
        }

        if directional_mode {
            let angle = match mode {
                "horizontal" => 0.0,
                "vertical" => 90.0,
                "diamond" => layout_config.angle,
                "checker" => edge_angle(side_angle),
                _ => edge_angle(side_angle),
            };
            let row_step = match mode {
                "vertical" => safe_row_step,
                "horizontal" => safe_row_step,
                "diamond" => (safe_row_step * 0.9).max(1.0),
                "checker" => safe_row_step,
                _ => safe_row_step,
            };
            let col_step = match mode {
                "vertical" => safe_col_step,
                "horizontal" => safe_col_step,
                "diamond" => (safe_col_step * 0.9).max(1.0),
                "checker" => safe_col_step,
                _ => safe_col_step,
            };

            let mut row_index = 0usize;
            let mut y = y_min + row_step / 2.0 + offset;
            while y < y_max {
                let row_offset = match mode {
                    "checker" => {
                        if row_index % 2 == 0 {
                            0.0
                        } else {
                            col_step * 0.5
                        }
                    }
                    "diamond" => {
                        if row_index % 2 == 0 {
                            col_step * 0.25
                        } else {
                            col_step * 0.75
                        }
                    }
                    _ => 0.0,
                };
                let mut x = x_min + col_step / 2.0 + offset + row_offset;
                while x < x_max {
                    draw_at(pixmap, x, y, angle, &mut rng);
                    x += col_step;
                }
                y += row_step;
                row_index += 1;
            }
            continue;
        }

        let line_horizontal = width >= height;
        if line_horizontal {
            let y = center_y;
            let mut x = x_min + offset;
            while x < x_max {
                draw_at(pixmap, x, y, edge_angle(side_angle), &mut rng);
                let jitter = if repeat_config.jitter_spacing != 0.0 {
                    rng.next_f32_range(-repeat_config.jitter_spacing, repeat_config.jitter_spacing)
                } else {
                    0.0
                };
                x += (safe_spacing + jitter).max(1.0);
            }
        } else {
            let x = center_x;
            let mut y = y_min + offset;
            while y < y_max {
                draw_at(pixmap, x, y, edge_angle(side_angle), &mut rng);
                let jitter = if repeat_config.jitter_spacing != 0.0 {
                    rng.next_f32_range(-repeat_config.jitter_spacing, repeat_config.jitter_spacing)
                } else {
                    0.0
                };
                y += (safe_spacing + jitter).max(1.0);
            }
        }
    }

    if layout_config.clip_to_frame {
        if let Some(mut mask_pixmap) = Pixmap::new(pixmap.width(), pixmap.height()) {
            let mut pb = PathBuilder::new();
            push_rounded_rect(&mut pb, outer_rect, 0.0);
            if let Some(full_path) = pb.finish() {
                let mut paint = Paint::default();
                paint.set_color(Color::WHITE);
                mask_pixmap.fill_path(
                    &full_path,
                    &paint,
                    FillRule::Winding,
                    Transform::identity(),
                    None,
                );
            }

            if let Some(inner_path) = get_inner_path(inner_rect, radius) {
                let mut clear_paint = Paint::default();
                clear_paint.blend_mode = BlendMode::Clear;
                mask_pixmap.fill_path(
                    &inner_path,
                    &clear_paint,
                    FillRule::Winding,
                    Transform::identity(),
                    None,
                );
            }

            let source = pixmap.clone();
            pixmap.fill(Color::TRANSPARENT);
            let mask =
                tiny_skia::Mask::from_pixmap(mask_pixmap.as_ref(), tiny_skia::MaskType::Alpha);
            pixmap.draw_pixmap(
                0,
                0,
                source.as_ref(),
                &tiny_skia::PixmapPaint::default(),
                Transform::identity(),
                Some(&mask),
            );
        }
    }
}

struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self {
            state: seed.wrapping_add(1),
        }
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

struct FrameGeometry {
    outer_rect: Rect,
    inner_rect: Rect,
    radius: f32,
}

fn gradient_points(rect: Rect, gradient: &MonitorGradientConfig) -> (Point, Point) {
    match gradient
        .mode
        .as_deref()
        .unwrap_or("vertical")
        .to_lowercase()
        .as_str()
    {
        "horizontal" => (
            Point::from_xy(rect.left(), rect.top() + rect.height() / 2.0),
            Point::from_xy(rect.right(), rect.top() + rect.height() / 2.0),
        ),
        "diagonal_down" => (
            Point::from_xy(rect.left(), rect.top()),
            Point::from_xy(rect.right(), rect.bottom()),
        ),
        "diagonal_up" => (
            Point::from_xy(rect.left(), rect.bottom()),
            Point::from_xy(rect.right(), rect.top()),
        ),
        _ => (
            Point::from_xy(rect.left() + rect.width() / 2.0, rect.top()),
            Point::from_xy(rect.left() + rect.width() / 2.0, rect.bottom()),
        ),
    }
}

fn fill_paint_for_frame(rect: Rect, config: &MonitorConfig) -> Paint<'static> {
    let mut paint = Paint::default();
    paint.anti_alias = true;

    if let Some(gradient) = &config.gradient {
        if gradient.enabled.unwrap_or(false) {
            let (start, end) = gradient_points(rect, gradient);
            let stops = vec![
                GradientStop::new(0.0, parse_hex_color(
                    gradient.start_color.as_deref().unwrap_or("#000000"),
                    config.opacity.unwrap_or(1.0),
                )),
                GradientStop::new(1.0, parse_hex_color(
                    gradient.end_color.as_deref().unwrap_or("#202020"),
                    config.opacity.unwrap_or(1.0),
                )),
            ];
            if let Some(shader) = LinearGradient::new(
                start,
                end,
                stops,
                SpreadMode::Pad,
                Transform::identity(),
            ) {
                paint.shader = shader;
                return paint;
            }
        }
    }

    paint.set_color(parse_hex_color(
        config.color.as_deref().unwrap_or("#000000"),
        config.opacity.unwrap_or(1.0),
    ));
    paint
}

fn compute_frame_geometry(width: u32, height: u32, config: &MonitorConfig) -> Option<FrameGeometry> {
    let t_base = config.thickness.unwrap_or(16) as f32;
    let frame_sides = config
        .frame_sides
        .clone()
        .unwrap_or(MonitorFrameSidesConfig {
            top: Some(true),
            bottom: Some(true),
            left: Some(true),
            right: Some(true),
        });
    let t_top = if frame_sides.top.unwrap_or(true) {
        config.thickness_top.map(|v| v as f32).unwrap_or(t_base)
    } else {
        0.0
    };
    let t_bottom = if frame_sides.bottom.unwrap_or(true) {
        config.thickness_bottom.map(|v| v as f32).unwrap_or(t_base)
    } else {
        0.0
    };
    let t_left = if frame_sides.left.unwrap_or(true) {
        config.thickness_left.map(|v| v as f32).unwrap_or(t_base)
    } else {
        0.0
    };
    let t_right = if frame_sides.right.unwrap_or(true) {
        config.thickness_right.map(|v| v as f32).unwrap_or(t_base)
    } else {
        0.0
    };

    let m_base = config.margin.unwrap_or(0) as f32;
    let m_top = config.margin_top.map(|v| v as f32).unwrap_or(m_base);
    let m_bottom = config.margin_bottom.map(|v| v as f32).unwrap_or(m_base);
    let m_left = config.margin_left.map(|v| v as f32).unwrap_or(m_base);
    let m_right = config.margin_right.map(|v| v as f32).unwrap_or(m_base);

    let offset_x = config.offset_x.unwrap_or(0) as f32;
    let offset_y = config.offset_y.unwrap_or(0) as f32;
    let radius = config.radius.unwrap_or(24) as f32;

    let outer_rect = Rect::from_xywh(
        m_left + offset_x,
        m_top + offset_y,
        width as f32 - m_left - m_right,
        height as f32 - m_top - m_bottom,
    )?;

    let inner_rect = Rect::from_xywh(
        outer_rect.left() + t_left,
        outer_rect.top() + t_top,
        outer_rect.width() - t_left - t_right,
        outer_rect.height() - t_top - t_bottom,
    )?;

    if outer_rect.width() <= 0.0
        || outer_rect.height() <= 0.0
        || inner_rect.width() <= 0.0
        || inner_rect.height() <= 0.0
    {
        return None;
    }

    Some(FrameGeometry {
        outer_rect,
        inner_rect,
        radius,
    })
}

pub fn draw_frame(width: u32, height: u32, config: &MonitorConfig) -> Vec<u8> {
    let mut pixmap = Pixmap::new(width, height).unwrap_or_else(|| Pixmap::new(1, 1).unwrap());

    if let Some(geometry) = compute_frame_geometry(width, height, config) {
        let paint = fill_paint_for_frame(geometry.outer_rect, config);
        pixmap.fill_rect(geometry.outer_rect, &paint, Transform::identity(), None);

        let hw = config.highlight_width.unwrap_or(3.0);
        if hw > 0.0 {
            if let Some(highlight_rect) = Rect::from_xywh(
                geometry.outer_rect.left() + hw / 2.0,
                geometry.outer_rect.top() + hw / 2.0,
                geometry.outer_rect.width() - hw,
                geometry.outer_rect.height() - hw,
            ) {
                let mut pb = PathBuilder::new();
                push_rounded_rect(&mut pb, highlight_rect, 0.0);
                if let Some(path) = pb.finish() {
                    let mut stroke_paint = Paint::default();
                    stroke_paint.set_color(parse_hex_color(
                        config.highlight_color.as_deref().unwrap_or("#FFFFFF22"),
                        config.opacity.unwrap_or(1.0),
                    ));
                    stroke_paint.anti_alias = true;
                    let mut stroke = Stroke::default();
                    stroke.width = hw;
                    pixmap.stroke_path(
                        &path,
                        &stroke_paint,
                        &stroke,
                        Transform::identity(),
                        None,
                    );
                }
            }
        }

        let sw = config.shadow_width.unwrap_or(12.0);
        if sw > 0.0 {
            let shadow_c = parse_hex_color(
                config.shadow_color.as_deref().unwrap_or("#000000AA"),
                config.opacity.unwrap_or(1.0),
            );
            let steps = sw.ceil() as usize;
            if steps > 0 {
                let total_a = shadow_c.alpha();
                let a_step = 1.0 - (1.0 - total_a).powf(1.0 / steps as f32);

                let step_color = tiny_skia::Color::from_rgba(
                    shadow_c.red(),
                    shadow_c.green(),
                    shadow_c.blue(),
                    a_step,
                )
                .unwrap_or_else(|| tiny_skia::Color::BLACK);

                let mut stroke_paint = Paint::default();
                stroke_paint.set_color(step_color);
                stroke_paint.anti_alias = true;

                for i in 0..steps {
                    let cur_sw = sw - (i as f32);
                    if cur_sw <= 0.0 {
                        continue;
                    }

                    let mut pb = PathBuilder::new();
                    push_rounded_rect(&mut pb, geometry.inner_rect, geometry.radius);
                    if let Some(path) = pb.finish() {
                        let mut stroke = Stroke::default();
                        stroke.width = cur_sw * 2.0;
                        pixmap.stroke_path(
                            &path,
                            &stroke_paint,
                            &stroke,
                            Transform::identity(),
                            None,
                        );
                    }
                }
            }
        }

        if let Some(path) = get_inner_path(geometry.inner_rect, geometry.radius) {
            let mut clear_paint = Paint::default();
            clear_paint.blend_mode = BlendMode::Clear;
            clear_paint.anti_alias = true;
            pixmap.fill_path(
                &path,
                &clear_paint,
                FillRule::Winding,
                Transform::identity(),
                None,
            );
        }

        if let Some(ish_config) = &config.inner_shadow {
            draw_inner_shadow(&mut pixmap, geometry.inner_rect, geometry.radius, ish_config);
        }

        if let Some(p_config) = &config.pattern {
            if let Some(mut pattern_pixmap) = Pixmap::new(width, height) {
                draw_pattern(
                    &mut pattern_pixmap,
                    geometry.outer_rect,
                    geometry.inner_rect,
                    geometry.radius,
                    p_config,
                );
                pixmap.draw_pixmap(
                    0,
                    0,
                    pattern_pixmap.as_ref(),
                    &tiny_skia::PixmapPaint::default(),
                    Transform::identity(),
                    None,
                );
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
    fn test_parse_hex_color_multiplies_global_opacity_and_hex_alpha() {
        let color = parse_hex_color("#33669980", 0.5);

        assert_eq!(color.red(), 0x33 as f32 / 255.0);
        assert_eq!(color.green(), 0x66 as f32 / 255.0);
        assert_eq!(color.blue(), 0x99 as f32 / 255.0);
        assert!((color.alpha() - (64.0 / 255.0)).abs() < 0.01);
    }

    #[test]
    fn test_draw_frame_opacity_respects_hex_alpha() {
        let config = MonitorConfig {
            enabled: Some(true),
            thickness: Some(12),
            thickness_top: None,
            thickness_bottom: None,
            thickness_left: None,
            thickness_right: None,
            frame_sides: None,
            margin: None,
            margin_top: None,
            margin_bottom: None,
            margin_left: None,
            margin_right: None,
            offset_x: None,
            offset_y: None,
            radius: Some(0),
            opacity: Some(0.5),
            color: Some("#ff000080".to_string()),
            preset: None,
            shadow_width: Some(0.0),
            shadow_color: Some("#00000000".to_string()),
            highlight_width: Some(0.0),
            highlight_color: Some("#00000000".to_string()),
            inner_shadow: None,
            gradient: None,
            pattern: None,
            layer: None,
        };

        let pixels = draw_frame(64, 64, &config);
        let idx = |x: usize, y: usize| (y * 64 + x) * 4;
        let pixel = &pixels[idx(4, 4)..idx(4, 4) + 4];

        assert_eq!(pixel[0], 0);
        assert_eq!(pixel[1], 0);
        assert!(pixel[2] >= 63 && pixel[2] <= 65, "expected premultiplied red near 64, got {}", pixel[2]);
        assert!(pixel[3] >= 63 && pixel[3] <= 65, "expected alpha near 64, got {}", pixel[3]);
    }

    #[test]
    fn test_draw_frame_basic() {
        let mut config = MonitorConfig {
            enabled: Some(true),
            thickness: Some(10),
            thickness_top: None,
            thickness_bottom: None,
            thickness_left: None,
            thickness_right: None,
            frame_sides: None,
            margin: None,
            margin_top: None,
            margin_bottom: None,
            margin_left: None,
            margin_right: None,
            offset_x: None,
            offset_y: None,
            radius: Some(5),
            opacity: Some(1.0),
            color: Some("#FF0000".to_string()),
            preset: None,
            shadow_width: Some(5.0),
            shadow_color: Some("#000000AA".to_string()),
            highlight_width: Some(2.0),
            highlight_color: Some("#FFFFFF22".to_string()),
            inner_shadow: None,
            gradient: None,
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
    fn test_draw_frame_respects_margins_and_offsets() {
        let config = MonitorConfig {
            enabled: Some(true),
            thickness: Some(8),
            thickness_top: None,
            thickness_bottom: None,
            thickness_left: None,
            thickness_right: None,
            frame_sides: None,
            margin: Some(10),
            margin_top: None,
            margin_bottom: None,
            margin_left: None,
            margin_right: None,
            offset_x: Some(6),
            offset_y: Some(4),
            radius: Some(4),
            opacity: Some(1.0),
            color: Some("#FF0000".to_string()),
            preset: None,
            shadow_width: Some(0.0),
            shadow_color: Some("#00000000".to_string()),
            highlight_width: Some(0.0),
            highlight_color: Some("#00000000".to_string()),
            inner_shadow: None,
            gradient: None,
            pattern: None,
            layer: None,
        };

        let pixels = draw_frame(64, 64, &config);
        let idx = |x: usize, y: usize| (y * 64 + x) * 4;

        assert_eq!(&pixels[idx(0, 0)..idx(0, 0) + 4], &[0, 0, 0, 0]);
        assert_eq!(&pixels[idx(18, 16)..idx(18, 16) + 4], &[0, 0, 255, 255]);
    }

    #[test]
    fn test_draw_frame_gradient_fill_varies_across_axis() {
        let config = MonitorConfig {
            enabled: Some(true),
            thickness: Some(12),
            thickness_top: None,
            thickness_bottom: None,
            thickness_left: None,
            thickness_right: None,
            frame_sides: None,
            margin: None,
            margin_top: None,
            margin_bottom: None,
            margin_left: None,
            margin_right: None,
            offset_x: None,
            offset_y: None,
            radius: Some(4),
            opacity: Some(1.0),
            color: Some("#000000".to_string()),
            preset: None,
            shadow_width: Some(0.0),
            shadow_color: Some("#00000000".to_string()),
            highlight_width: Some(0.0),
            highlight_color: Some("#00000000".to_string()),
            inner_shadow: None,
            gradient: Some(MonitorGradientConfig {
                enabled: Some(true),
                mode: Some("vertical".to_string()),
                start_color: Some("#FF0000".to_string()),
                end_color: Some("#0000FF".to_string()),
            }),
            pattern: None,
            layer: None,
        };

        let pixels = draw_frame(64, 64, &config);
        let idx = |x: usize, y: usize| (y * 64 + x) * 4;
        let top = &pixels[idx(6, 6)..idx(6, 6) + 4];
        let bottom = &pixels[idx(6, 57)..idx(6, 57) + 4];

        assert_ne!(top, bottom);
        assert!(top[2] > top[0]);
        assert!(bottom[0] > bottom[2]);
    }

    #[test]
    fn test_draw_frame_explicit_side_toggle_disables_side() {
        let config = MonitorConfig {
            enabled: Some(true),
            thickness: Some(8),
            thickness_top: None,
            thickness_bottom: None,
            thickness_left: None,
            thickness_right: None,
            frame_sides: Some(MonitorFrameSidesConfig {
                top: Some(false),
                bottom: Some(true),
                left: Some(true),
                right: Some(true),
            }),
            margin: None,
            margin_top: None,
            margin_bottom: None,
            margin_left: None,
            margin_right: None,
            offset_x: None,
            offset_y: None,
            radius: Some(4),
            opacity: Some(1.0),
            color: Some("#FF0000".to_string()),
            preset: None,
            shadow_width: Some(0.0),
            shadow_color: Some("#00000000".to_string()),
            highlight_width: Some(0.0),
            highlight_color: Some("#00000000".to_string()),
            inner_shadow: None,
            gradient: None,
            pattern: None,
            layer: None,
        };

        let pixels = draw_frame(64, 64, &config);
        let idx = |x: usize, y: usize| (y * 64 + x) * 4;

        assert_eq!(&pixels[idx(16, 2)..idx(16, 2) + 4], &[0, 0, 0, 0]);
        assert_eq!(&pixels[idx(2, 16)..idx(2, 16) + 4], &[0, 0, 255, 255]);
    }

    #[test]
    fn test_rounded_inner_cutout_produces_anti_aliased_edge_pixels() {
        let config = MonitorConfig {
            enabled: Some(true),
            thickness: Some(12),
            thickness_top: None,
            thickness_bottom: None,
            thickness_left: None,
            thickness_right: None,
            frame_sides: None,
            margin: None,
            margin_top: None,
            margin_bottom: None,
            margin_left: None,
            margin_right: None,
            offset_x: None,
            offset_y: None,
            radius: Some(12),
            opacity: Some(1.0),
            color: Some("#ff0000".to_string()),
            preset: None,
            shadow_width: Some(0.0),
            shadow_color: Some("#00000000".to_string()),
            highlight_width: Some(0.0),
            highlight_color: Some("#00000000".to_string()),
            inner_shadow: None,
            gradient: None,
            pattern: None,
            layer: None,
        };

        let pixels = draw_frame(64, 64, &config);
        let idx = |x: usize, y: usize| (y * 64 + x) * 4;

        assert_eq!(pixels[idx(24, 24) + 3], 0);
        assert_eq!(pixels[idx(4, 4) + 3], 255);

        let mut saw_partial_alpha = false;
        for y in 14..18 {
            for x in 14..18 {
                let alpha = pixels[idx(x, y) + 3];
                if alpha > 0 && alpha < 255 {
                    saw_partial_alpha = true;
                    break;
                }
            }
            if saw_partial_alpha {
                break;
            }
        }

        assert!(saw_partial_alpha, "expected partially covered pixels around the rounded inner cutout edge");
    }

    #[test]
    fn test_pattern_clip_to_frame_keeps_inner_cutout_clear() {
        use std::env::temp_dir;
        use std::fs;

        use crate::config::{
            MonitorPatternConfig, PatternCornersConfig, PatternLayoutConfig, PatternRepeatConfig,
            PatternSidesConfig, PatternTransformConfig,
        };

        let svg_path = temp_dir().join("rustbar-pattern-clip-inner.svg");
        fs::write(
            &svg_path,
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20">
<rect width="20" height="20" fill="#ffffff"/>
</svg>"##,
        )
        .unwrap();

        let base_pattern = MonitorPatternConfig {
            enabled: Some(true),
            source: Some(svg_path.display().to_string()),
            mode: Some("single".to_string()),
            opacity: Some(1.0),
            color: Some("#ffffff".to_string()),
            blend: Some("normal".to_string()),
            seed: Some(1),
            transform: Some(PatternTransformConfig {
                scale: 2.0,
                rotation: 0.0,
                flip_x: false,
                flip_y: false,
            }),
            repeat: Some(PatternRepeatConfig {
                spacing: 24.0,
                offset: 0.0,
                jitter_spacing: 0.0,
                jitter_rotation: 0.0,
                jitter_scale: 0.0,
            }),
            sides: Some(PatternSidesConfig {
                top: true,
                bottom: false,
                left: false,
                right: false,
            }),
            corners: Some(PatternCornersConfig {
                avoid_corners: false,
                corner_padding: 0.0,
            }),
            layout: Some(PatternLayoutConfig {
                follow_edge_direction: true,
                clip_to_frame: true,
                prevent_overlap: false,
                overlap_padding: 0.0,
                angle: 45.0,
            }),
        };

        let config = MonitorConfig {
            enabled: Some(true),
            thickness: Some(8),
            thickness_top: None,
            thickness_bottom: None,
            thickness_left: None,
            thickness_right: None,
            frame_sides: None,
            margin: None,
            margin_top: None,
            margin_bottom: None,
            margin_left: None,
            margin_right: None,
            offset_x: None,
            offset_y: None,
            radius: Some(0),
            opacity: Some(1.0),
            color: Some("#000000".to_string()),
            preset: None,
            shadow_width: Some(0.0),
            shadow_color: Some("#00000000".to_string()),
            highlight_width: Some(0.0),
            highlight_color: Some("#00000000".to_string()),
            inner_shadow: None,
            gradient: None,
            pattern: Some(base_pattern.clone()),
            layer: None,
        };

        let clipped = draw_frame(64, 64, &config);

        let mut unclipped_config = config.clone();
        if let Some(pattern) = &mut unclipped_config.pattern {
            if let Some(layout) = &mut pattern.layout {
                layout.clip_to_frame = false;
            }
        }
        let unclipped = draw_frame(64, 64, &unclipped_config);

        let idx = |x: usize, y: usize| (y * 64 + x) * 4;
        assert_eq!(&clipped[idx(32, 10)..idx(32, 10) + 4], &[0, 0, 0, 0]);
        assert_ne!(&unclipped[idx(32, 10)..idx(32, 10) + 4], &[0, 0, 0, 0]);
    }

    #[test]
    fn test_pattern_clip_to_frame_respects_outer_margins() {
        use std::env::temp_dir;
        use std::fs;

        use crate::config::{
            MonitorPatternConfig, PatternCornersConfig, PatternLayoutConfig, PatternRepeatConfig,
            PatternSidesConfig, PatternTransformConfig,
        };

        let svg_path = temp_dir().join("rustbar-pattern-clip-outer.svg");
        fs::write(
            &svg_path,
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20">
<rect width="20" height="20" fill="#ffffff"/>
</svg>"##,
        )
        .unwrap();

        let pattern = MonitorPatternConfig {
            enabled: Some(true),
            source: Some(svg_path.display().to_string()),
            mode: Some("single".to_string()),
            opacity: Some(1.0),
            color: Some("#ffffff".to_string()),
            blend: Some("normal".to_string()),
            seed: Some(2),
            transform: Some(PatternTransformConfig {
                scale: 2.0,
                rotation: 0.0,
                flip_x: false,
                flip_y: false,
            }),
            repeat: Some(PatternRepeatConfig {
                spacing: 24.0,
                offset: 0.0,
                jitter_spacing: 0.0,
                jitter_rotation: 0.0,
                jitter_scale: 0.0,
            }),
            sides: Some(PatternSidesConfig {
                top: false,
                bottom: false,
                left: true,
                right: false,
            }),
            corners: Some(PatternCornersConfig {
                avoid_corners: false,
                corner_padding: 0.0,
            }),
            layout: Some(PatternLayoutConfig {
                follow_edge_direction: true,
                clip_to_frame: true,
                prevent_overlap: false,
                overlap_padding: 0.0,
                angle: 45.0,
            }),
        };

        let config = MonitorConfig {
            enabled: Some(true),
            thickness: Some(8),
            thickness_top: None,
            thickness_bottom: None,
            thickness_left: None,
            thickness_right: None,
            frame_sides: None,
            margin: Some(10),
            margin_top: None,
            margin_bottom: None,
            margin_left: None,
            margin_right: None,
            offset_x: None,
            offset_y: None,
            radius: Some(0),
            opacity: Some(1.0),
            color: Some("#000000".to_string()),
            preset: None,
            shadow_width: Some(0.0),
            shadow_color: Some("#00000000".to_string()),
            highlight_width: Some(0.0),
            highlight_color: Some("#00000000".to_string()),
            inner_shadow: None,
            gradient: None,
            pattern: Some(pattern),
            layer: None,
        };

        let clipped = draw_frame(64, 64, &config);

        let mut unclipped_config = config.clone();
        if let Some(pattern) = &mut unclipped_config.pattern {
            if let Some(layout) = &mut pattern.layout {
                layout.clip_to_frame = false;
            }
        }
        let unclipped = draw_frame(64, 64, &unclipped_config);

        let idx = |x: usize, y: usize| (y * 64 + x) * 4;
        assert_eq!(&clipped[idx(4, 32)..idx(4, 32) + 4], &[0, 0, 0, 0]);
        assert_ne!(&unclipped[idx(4, 32)..idx(4, 32) + 4], &[0, 0, 0, 0]);
    }

    #[test]
    fn test_svg_cache_invalidates_when_asset_disappears() {
        use std::{env::temp_dir, fs};

        let svg_path = temp_dir().join("rustbar-svg-cache-test.svg");
        fs::write(
            &svg_path,
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 4 4">
<rect width="4" height="4" fill="#ffffff"/>
</svg>"##,
        )
        .unwrap();

        clear_svg_cache();
        assert!(load_svg_tree(&svg_path).is_ok());

        fs::remove_file(&svg_path).unwrap();
        assert!(matches!(
            load_svg_tree(&svg_path),
            Err(SvgCacheFailure::Missing)
        ));
    }

    #[test]
    fn test_draw_pattern_png() {
        use std::env::temp_dir;
        use std::fs;

        use crate::config::{
            MonitorPatternConfig, PatternCornersConfig, PatternLayoutConfig, PatternRepeatConfig,
            PatternSidesConfig, PatternTransformConfig,
        };
        let svg_path = temp_dir().join("rustbar-pattern-test.svg");
        fs::write(
            &svg_path,
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20">
<circle cx="10" cy="10" r="7" fill="#ffffff"/>
</svg>"##,
        )
        .unwrap();

        let config = MonitorConfig {
            enabled: Some(true),
            thickness: Some(32),
            thickness_top: Some(72),
            thickness_bottom: Some(72),
            thickness_left: Some(48),
            thickness_right: Some(48),
            frame_sides: None,
            margin: None,
            margin_top: None,
            margin_bottom: None,
            margin_left: None,
            margin_right: None,
            offset_x: None,
            offset_y: None,
            radius: Some(32),
            opacity: Some(0.9),
            color: Some("#1a1b26".to_string()),
            preset: None,
            shadow_width: Some(0.0),
            shadow_color: Some("#00000000".to_string()),
            highlight_width: Some(0.0),
            highlight_color: Some("#00000000".to_string()),
            inner_shadow: None,
            gradient: None,
            pattern: Some(MonitorPatternConfig {
                enabled: Some(true),
                source: Some(svg_path.display().to_string()),
                mode: Some("horizontal".to_string()),
                opacity: Some(0.4),
                color: Some("#f7768e".to_string()),
                blend: Some("normal".to_string()),
                seed: Some(42),
                transform: Some(PatternTransformConfig {
                    scale: 0.8,
                    rotation: 0.0,
                    flip_x: false,
                    flip_y: false,
                }),
                repeat: Some(PatternRepeatConfig {
                    spacing: 34.0,
                    offset: 0.0,
                    jitter_spacing: 0.0,
                    jitter_rotation: 0.0,
                    jitter_scale: 0.0,
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
                    prevent_overlap: true,
                    overlap_padding: 4.0,
                    angle: 45.0,
                }),
            }),
            layer: None,
        };

        let width = 800;
        let height = 600;
        let horizontal_pixels = draw_frame(width, height, &config);
        assert_eq!(horizontal_pixels.len(), (width * height * 4) as usize);

        let mut no_pattern = config.clone();
        no_pattern.pattern = None;
        let baseline = draw_frame(width, height, &no_pattern);
        assert_ne!(horizontal_pixels, baseline);

        let mut diagonal = config.clone();
        if let Some(pattern) = &mut diagonal.pattern {
            pattern.mode = Some("diagonal".to_string());
        }
        let diagonal_pixels = draw_frame(width, height, &diagonal);
        assert_ne!(diagonal_pixels, baseline);
        assert_ne!(diagonal_pixels, horizontal_pixels);

        let mut steep_diagonal = diagonal.clone();
        if let Some(pattern) = &mut steep_diagonal.pattern {
            if let Some(layout) = &mut pattern.layout {
                layout.angle = 65.0;
            }
        }
        let steep_diagonal_pixels = draw_frame(width, height, &steep_diagonal);
        assert_ne!(steep_diagonal_pixels, baseline);
        assert_ne!(steep_diagonal_pixels, diagonal_pixels);

        let _ = fs::remove_file(svg_path);
    }
}
