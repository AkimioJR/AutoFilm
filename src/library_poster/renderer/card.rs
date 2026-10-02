use image::{DynamicImage, Rgba, RgbaImage};
use imageproc::geometric_transformations::{Border, Interpolation, rotate_about_center};

use super::{
    Fonts, Result,
    utils::{
        adjust_brightness, apply_rounded_corners, cover, dominant_color, draw_titles_wrapped,
        optimized_blur, overlay_with_shadow, tint,
    },
};
use crate::library_poster::RenderConfig;

/// 生成右侧 iMessage 式多卡片堆叠、左侧标题的卡片风格。
pub fn render(
    images: &[DynamicImage],
    title: &str,
    subtitle: &str,
    fonts: &Fonts,
    config: &RenderConfig,
    dimensions: (u32, u32),
) -> Result<RgbaImage> {
    let source = images.first().ok_or(super::Error::MissingImage)?;
    let (width, height) = dimensions;
    let width_f = width as f32;
    let height_f = height as f32;
    let theme = dominant_color(source);
    let mut canvas = cover(source, width, height);
    canvas = optimized_blur(&canvas, config.blur_radius.max(0.0) * height_f / 1080.0);
    tint(
        &mut canvas,
        adjust_brightness(theme, 0.78),
        config.color_strength.clamp(0.0, 1.0),
    );

    let card_height = (height_f * 0.76) as u32;
    let card_height_f = card_height as f32;
    let card_width = (card_height_f * 0.70) as u32;
    let card_width_f = card_width as f32;
    let target_center_x = width_f * 0.75;
    let bottom_edge_y = height_f * 0.900;
    let pivot_step = width_f * 0.008;

    // 根据实际素材数量使用 7/5/3/1 层，避免重复图片填满卡片堆。
    let layers = layers_for_count(images.len(), pivot_step);
    let rendered_layers = layers
        .iter()
        .copied()
        .map(|layer| {
            let layer_width = (card_width_f * layer.scale) as u32;
            let layer_height = (card_height_f * layer.scale) as u32;
            let card = prepare_card(
                &images[layer.image_index],
                layer_width,
                layer_height,
                layer.darkness,
                layer.opacity,
            );
            RenderedLayer {
                card: rotate_around_bottom_anchor(&card, layer.angle),
                offset: layer.offset,
            }
        })
        .collect::<Vec<_>>();
    let visual_center = visible_center_x(&rendered_layers);

    for (depth, layer) in rendered_layers.iter().enumerate() {
        let x = (target_center_x + layer.offset - layer.card.anchor_x - visual_center) as i64;
        let y = (bottom_edge_y - layer.card.opaque_bottom) as i64;
        if depth + 1 == rendered_layers.len() {
            overlay_with_shadow(
                &mut canvas,
                &layer.card.image,
                x,
                y,
                (height_f * 0.008) as i64,
                (height_f * 0.010) as i64,
                height_f * 0.010,
                138,
            );
        } else {
            image::imageops::overlay(&mut canvas, &layer.card.image, x, y);
        }
    }

    draw_titles_wrapped(
        &mut canvas,
        title,
        subtitle,
        fonts,
        (width_f * 0.25) as i32,
        (height_f * 0.37) as i32,
        height_f * 0.14,
        height_f * 0.055,
        Rgba([255, 255, 255, 235]),
        (width_f * 0.40) as i32,
    );
    Ok(canvas)
}

#[derive(Clone, Copy)]
struct Layer {
    image_index: usize,
    offset: f32,
    angle: f32,
    scale: f32,
    darkness: f32,
    opacity: u8,
}

fn layers_for_count(image_count: usize, pivot_step: f32) -> Vec<Layer> {
    let visible_count = match image_count {
        7.. => 7,
        5.. => 5,
        3.. => 3,
        _ => 1,
    };
    let side_count = (visible_count - 1) / 2;
    let side_count_f = side_count.max(1) as f32;
    let mut layers = Vec::with_capacity(visible_count);

    // 先从最外侧绘制到内侧，最后绘制中央主卡。
    for distance in (1..=side_count).rev() {
        let distance_f = distance as f32;
        let progress = distance_f / side_count_f;
        let scale = 1.0 - progress * 0.055;
        let darkness = progress * 0.30;
        let opacity = (255.0 - progress * 36.0) as u8;
        let angle = progress * 11.5;
        let offset = pivot_step * distance_f;
        let left_index = distance * 2 - 1;
        let right_index = distance * 2;

        layers.push(Layer {
            image_index: left_index,
            offset: -offset,
            angle: -angle,
            scale,
            darkness,
            opacity,
        });
        layers.push(Layer {
            image_index: right_index,
            offset,
            angle,
            scale,
            darkness,
            opacity,
        });
    }
    layers.push(Layer {
        image_index: 0,
        offset: 0.0,
        angle: 0.0,
        scale: 1.0,
        darkness: 0.0,
        opacity: 255,
    });
    layers
}

fn prepare_card(
    source: &DynamicImage,
    width: u32,
    height: u32,
    darkness: f32,
    opacity: u8,
) -> RgbaImage {
    let mut card = cover(source, width, height);
    if darkness > 0.0 {
        tint(&mut card, Rgba([18, 18, 22, 255]), darkness);
    }
    apply_rounded_corners(&mut card, (width as f32 * 0.10) as u32);
    if opacity < 255 {
        for pixel in card.pixels_mut() {
            pixel[3] = ((u16::from(pixel[3]) * u16::from(opacity)) / 255) as u8;
        }
    }
    card
}

struct AnchoredCard {
    image: RgbaImage,
    anchor_x: f32,
    opaque_bottom: f32,
}

struct RenderedLayer {
    card: AnchoredCard,
    offset: f32,
}

#[derive(Clone, Copy)]
struct AlphaBounds {
    left: u32,
    right: u32,
}

/// 使用所有卡片的真实可见边界计算视觉中心，避免透明旋转画布影响定位。
fn visible_center_x(layers: &[RenderedLayer]) -> f32 {
    let bounds = layers.iter().filter_map(|layer| {
        alpha_bounds(&layer.card.image).map(|bounds| {
            (
                layer.offset - layer.card.anchor_x + bounds.left as f32,
                layer.offset - layer.card.anchor_x + bounds.right as f32,
            )
        })
    });
    let (left, right) = bounds.fold(
        (f32::INFINITY, f32::NEG_INFINITY),
        |(left, right), (layer_left, layer_right)| (left.min(layer_left), right.max(layer_right)),
    );
    if left.is_finite() && right.is_finite() {
        (left + right) / 2.0
    } else {
        0.0
    }
}

fn alpha_bounds(image: &RgbaImage) -> Option<AlphaBounds> {
    let mut left = image.width();
    let mut right = 0;
    let mut found = false;
    for (x, _, pixel) in image.enumerate_pixels() {
        if pixel[3] <= 8 {
            continue;
        }
        left = left.min(x);
        right = right.max(x);
        found = true;
    }
    found.then_some(AlphaBounds { left, right })
}

/// 以卡片底部中央为共同轴心旋转，让扇形顶部展开而底部保持收束。
fn rotate_around_bottom_anchor(card: &RgbaImage, angle: f32) -> AnchoredCard {
    if angle == 0.0 {
        return AnchoredCard {
            image: card.clone(),
            anchor_x: card.width() as f32 / 2.0,
            opaque_bottom: card.height().saturating_sub(1) as f32,
        };
    }

    let pivot_from_top = card.height() as f32 * 0.96;
    let half_width = card.width() as f32 / 2.0;
    let top_radius = half_width.hypot(pivot_from_top);
    let bottom_radius = half_width.hypot(card.height() as f32 - pivot_from_top);
    let radius = top_radius.max(bottom_radius);
    let canvas_size = (radius.ceil() as u32 * 2).max(1);
    let center = canvas_size as f32 / 2.0;
    let mut pivot_canvas = RgbaImage::from_pixel(canvas_size, canvas_size, Rgba([0, 0, 0, 0]));
    image::imageops::overlay(
        &mut pivot_canvas,
        card,
        (center - half_width) as i64,
        (center - pivot_from_top) as i64,
    );
    let image = rotate_about_center(
        &pivot_canvas,
        angle.to_radians(),
        Interpolation::Bicubic,
        Border::Constant(Rgba([0, 0, 0, 0])),
    );
    let opaque_bottom = image
        .enumerate_pixels()
        .filter(|(_, _, pixel)| pixel[3] > 8)
        .map(|(_, y, _)| y)
        .max()
        .unwrap_or(0) as f32;
    AnchoredCard {
        image,
        anchor_x: center,
        opaque_bottom,
    }
}

#[cfg(test)]
mod tests {
    use image::{DynamicImage, Rgb, RgbImage};

    use super::*;

    #[test]
    fn prepares_rounded_translucent_back_card() {
        let source = DynamicImage::ImageRgb8(RgbImage::from_pixel(400, 600, Rgb([80, 160, 220])));

        let card = prepare_card(&source, 200, 300, 0.25, 200);

        assert_eq!(card.dimensions(), (200, 300));
        assert_eq!(card.get_pixel(0, 0)[3], 0);
        assert_eq!(card.get_pixel(100, 150)[3], 200);
    }

    #[test]
    fn rotating_card_tracks_real_opaque_bottom() {
        let card = RgbaImage::from_pixel(200, 300, Rgba([80, 160, 220, 255]));

        let rotated = rotate_around_bottom_anchor(&card, 8.0);

        assert!(rotated.image.width() > card.width());
        assert!(rotated.image.height() > card.height());
        assert_eq!(rotated.image.get_pixel(0, 0)[3], 0);
        assert_eq!(rotated.anchor_x, rotated.image.width() as f32 / 2.0);
        assert!(rotated.opaque_bottom > rotated.image.height() as f32 / 2.0);
    }

    #[test]
    fn card_layers_fall_back_to_odd_counts() {
        for (available, expected) in [
            (1, 1),
            (2, 1),
            (3, 3),
            (4, 3),
            (5, 5),
            (6, 5),
            (7, 7),
            (10, 7),
        ] {
            let layers = layers_for_count(available, 40.0);

            assert_eq!(layers.len(), expected);
            assert!(layers.iter().all(|layer| layer.image_index < available));
        }
    }

    #[test]
    fn visible_center_ignores_transparent_rotation_padding() {
        let left = AnchoredCard {
            image: padded_card(40, 60, 10, 8),
            anchor_x: 30.0,
            opaque_bottom: 67.0,
        };
        let right = AnchoredCard {
            image: padded_card(40, 60, 14, 8),
            anchor_x: 34.0,
            opaque_bottom: 67.0,
        };
        let layers = [
            RenderedLayer {
                card: left,
                offset: -20.0,
            },
            RenderedLayer {
                card: right,
                offset: 20.0,
            },
        ];

        assert!(visible_center_x(&layers).abs() <= 0.5);
    }

    fn padded_card(width: u32, height: u32, left: u32, top: u32) -> RgbaImage {
        let mut image =
            RgbaImage::from_pixel(width + left * 2, height + top * 2, Rgba([0, 0, 0, 0]));
        for y in top..top + height {
            for x in left..left + width {
                image.put_pixel(x, y, Rgba([80, 160, 220, 255]));
            }
        }
        image
    }
}
