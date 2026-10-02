use image::{DynamicImage, GenericImageView, Rgba, RgbaImage};

use super::{
    Fonts, Result,
    utils::{
        adjust_brightness, cover_resize_dimensions, dominant_color, draw_titles_wrapped,
        optimized_blur, tint,
    },
};
use crate::library_poster::RenderConfig;

/// 生成斜线分隔背景与靠右主图的分割风格。
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
    let mut canvas = super::utils::cover(source, width, height);
    canvas = optimized_blur(&canvas, config.blur_radius.max(0.0) * height_f / 1080.0);
    tint(
        &mut canvas,
        adjust_brightness(theme, 0.78),
        config.color_strength.clamp(0.0, 1.0),
    );

    let foreground = align_image_right(source, width, height);
    let feather_width = (width_f * 0.004).max(1.5);
    let shadow_width = (width_f * 0.012).max(3.0);
    let height_denominator = height.max(1) as f32;
    for y in 0..height {
        let progress = y as f32 / height_denominator;
        let boundary = width_f * (0.55 - progress * 0.15);
        let blend_start = (boundary - feather_width).max(0.0) as u32;

        for x in blend_start..width {
            let distance = x as f32 - boundary;
            let foreground_alpha = smoothstep(-feather_width, feather_width, distance);
            let shadow_alpha = edge_shadow(distance, shadow_width);
            let background = *canvas.get_pixel(x, y);
            let foreground = *foreground.get_pixel(x, y);
            canvas.put_pixel(
                x,
                y,
                blend_pixel(background, foreground, foreground_alpha, shadow_alpha),
            );
        }
    }

    draw_titles_wrapped(
        &mut canvas,
        title,
        subtitle,
        fonts,
        (width_f * 0.25) as i32,
        (height_f * 0.36) as i32,
        height_f * 0.15,
        height_f * 0.06,
        Rgba([255, 255, 255, 235]),
        (width_f * 0.40) as i32,
    );
    Ok(canvas)
}

/// 将素材缩放到画布高度，并以右侧展示区域为中心裁剪。
fn align_image_right(source: &DynamicImage, width: u32, height: u32) -> RgbaImage {
    let target_width = (width as f32 * 0.68) as u32;
    let (source_width, source_height) = source.dimensions();
    let (resized_width, resized_height) =
        cover_resize_dimensions(source_width, source_height, target_width, height);
    let resized = source
        .resize_exact(
            resized_width,
            resized_height,
            image::imageops::FilterType::Lanczos3,
        )
        .to_rgba8();
    let crop_left = resized_width.saturating_sub(target_width) / 2;
    let crop_top = resized_height.saturating_sub(height) / 2;
    let cropped =
        image::imageops::crop_imm(&resized, crop_left, crop_top, target_width, height).to_image();
    let mut foreground = RgbaImage::from_pixel(width, height, Rgba([0, 0, 0, 255]));
    image::imageops::overlay(
        &mut foreground,
        &cropped,
        i64::from(width.saturating_sub(target_width)),
        0,
    );
    foreground
}

fn smoothstep(start: f32, end: f32, value: f32) -> f32 {
    let progress = ((value - start) / (end - start)).clamp(0.0, 1.0);
    progress * progress * (3.0 - 2.0 * progress)
}

/// 在分割线右侧生成轻柔投影，避免形成生硬的黑色描边。
fn edge_shadow(distance: f32, width: f32) -> f32 {
    if distance < -width || distance > width * 2.0 {
        return 0.0;
    }
    let center = width * 0.25;
    let deviation = width * 0.75;
    let normalized = (distance - center) / deviation;
    (-0.5 * normalized * normalized).exp() * 0.16
}

fn blend_pixel(
    background: Rgba<u8>,
    foreground: Rgba<u8>,
    foreground_alpha: f32,
    shadow_alpha: f32,
) -> Rgba<u8> {
    let mut pixel = [255; 4];
    for channel in 0..3 {
        let blended = background[channel] as f32 * (1.0 - foreground_alpha)
            + foreground[channel] as f32 * foreground_alpha;
        pixel[channel] = (blended * (1.0 - shadow_alpha)).clamp(0.0, 255.0) as u8;
    }
    Rgba(pixel)
}

#[cfg(test)]
mod tests {
    use image::{DynamicImage, Rgb, RgbImage};

    use super::*;

    #[test]
    fn right_aligned_image_fills_target_area() {
        let source = DynamicImage::ImageRgb8(RgbImage::from_pixel(400, 900, Rgb([40, 120, 220])));

        let foreground = align_image_right(&source, 640, 360);

        assert_eq!(foreground.dimensions(), (640, 360));
        assert_eq!(foreground.get_pixel(639, 180), &Rgba([40, 120, 220, 255]));
    }

    #[test]
    fn split_edge_blends_without_black_outline() {
        let background = Rgba([80, 40, 40, 255]);
        let foreground = Rgba([240, 220, 200, 255]);

        let middle = blend_pixel(background, foreground, 0.5, 0.12);

        assert!(middle[0] > background[0]);
        assert!(middle[0] < foreground[0]);
        assert!(middle[1] > background[1]);
    }
}
