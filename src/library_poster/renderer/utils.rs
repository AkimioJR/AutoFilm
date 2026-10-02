use std::collections::HashMap;

use ab_glyph::{Font, FontArc, PxScale, ScaleFont};
use image::{
    DynamicImage, GenericImageView, Rgba, RgbaImage,
    imageops::{FilterType, blur, overlay},
};
use imageproc::drawing::draw_text_mut;

use super::Fonts;

/// 将素材按“填满画布并居中裁剪”的方式缩放。
pub fn cover(image: &DynamicImage, width: u32, height: u32) -> RgbaImage {
    let (source_width, source_height) = image.dimensions();
    let (resized_width, resized_height) =
        cover_resize_dimensions(source_width, source_height, width, height);
    let resized = image
        .resize_exact(resized_width, resized_height, FilterType::Lanczos3)
        .to_rgba8();
    let left = resized_width.saturating_sub(width) / 2;
    let top = resized_height.saturating_sub(height) / 2;
    image::imageops::crop_imm(&resized, left, top, width, height).to_image()
}

/// 计算覆盖目标区域所需的缩放尺寸，把浮点转换集中在图像库边界。
pub(super) fn cover_resize_dimensions(
    source_width: u32,
    source_height: u32,
    target_width: u32,
    target_height: u32,
) -> (u32, u32) {
    let source_width_f = source_width as f32;
    let source_height_f = source_height as f32;
    let scale = f32::max(
        target_width as f32 / source_width_f,
        target_height as f32 / source_height_f,
    );
    (
        (source_width_f * scale).ceil() as u32,
        (source_height_f * scale).ceil() as u32,
    )
}

/// 使用缩略图量化统计提取适合作为背景的主题色。
pub fn dominant_color(image: &DynamicImage) -> Rgba<u8> {
    let thumbnail = image.thumbnail(96, 96).to_rgb8();
    let mut colors = HashMap::<[u8; 3], usize>::new();
    for pixel in thumbnail.pixels() {
        let quantized = [pixel[0] / 32 * 32, pixel[1] / 32 * 32, pixel[2] / 32 * 32];
        *colors.entry(quantized).or_default() += 1;
    }
    let color = colors
        .into_iter()
        .filter(|(color, _)| is_suitable_theme_color(*color))
        .max_by_key(|(_, count)| *count)
        .map(|(color, _)| color)
        .unwrap_or([96, 96, 96]);
    Rgba([color[0], color[1], color[2], 255])
}

fn is_suitable_theme_color(color: [u8; 3]) -> bool {
    let [red, green, blue] = color;
    let red = f32::from(red);
    let green = f32::from(green);
    let blue = f32::from(blue);
    let maximum = red.max(green).max(blue) / 255.0;
    let minimum = red.min(green).min(blue) / 255.0;
    let saturation = if maximum == 0.0 {
        0.0
    } else {
        (maximum - minimum) / maximum
    };
    let luminance = (0.299 * red + 0.587 * green + 0.114 * blue) / 255.0;

    (0.16..=0.86).contains(&luminance) && saturation >= 0.16
}

pub fn gradient_background(width: u32, height: u32, color: Rgba<u8>) -> RgbaImage {
    let left = adjust_brightness(color, 0.48);
    let right = adjust_brightness(color, 1.32);
    let width_f = width.max(1) as f32;
    RgbaImage::from_fn(width, height, |x, _| {
        let progress = x as f32 / width_f;
        mix_colors(left, right, progress.powf(0.72))
    })
}

pub fn adjust_brightness(color: Rgba<u8>, factor: f32) -> Rgba<u8> {
    Rgba([
        (color[0] as f32 * factor).clamp(0.0, 255.0) as u8,
        (color[1] as f32 * factor).clamp(0.0, 255.0) as u8,
        (color[2] as f32 * factor).clamp(0.0, 255.0) as u8,
        color[3],
    ])
}

pub fn mix_colors(left: Rgba<u8>, right: Rgba<u8>, ratio: f32) -> Rgba<u8> {
    let ratio = ratio.clamp(0.0, 1.0);
    let mut result = [0; 4];
    for channel in 0..4 {
        result[channel] =
            (left[channel] as f32 * (1.0 - ratio) + right[channel] as f32 * ratio) as u8;
    }
    Rgba(result)
}

pub fn tint(image: &mut RgbaImage, color: Rgba<u8>, strength: f32) {
    let strength = strength.clamp(0.0, 1.0);
    for pixel in image.pixels_mut() {
        for channel in 0..3 {
            pixel[channel] =
                (pixel[channel] as f32 * (1.0 - strength) + color[channel] as f32 * strength) as u8;
        }
    }
}

/// 在较小图像上执行高斯模糊，再恢复原尺寸以降低大分辨率渲染开销。
pub fn optimized_blur(image: &RgbaImage, radius: f32) -> RgbaImage {
    if radius <= 0.0 {
        return image.clone();
    }

    let longest_side = image.width().max(image.height());
    if longest_side <= 720 {
        return blur(image, radius);
    }

    let scale = 720.0 / longest_side as f32;
    let small_width = (image.width() as f32 * scale).round().max(1.0) as u32;
    let small_height = (image.height() as f32 * scale).round().max(1.0) as u32;
    let small = image::imageops::resize(image, small_width, small_height, FilterType::Triangle);
    let blurred = blur(&small, (radius * scale).max(1.0));
    image::imageops::resize(
        &blurred,
        image.width(),
        image.height(),
        FilterType::Lanczos3,
    )
}

pub fn apply_rounded_corners(image: &mut RgbaImage, radius: u32) {
    let width = image.width();
    let height = image.height();
    let radius = radius.min(width / 2).min(height / 2);
    let radius_squared = i64::from(radius).pow(2);
    for y in 0..height {
        for x in 0..width {
            let corner = match (
                x < radius,
                x >= width - radius,
                y < radius,
                y >= height - radius,
            ) {
                (true, _, true, _) => Some((radius - x, radius - y)),
                (_, true, true, _) => Some((x - (width - radius - 1), radius - y)),
                (true, _, _, true) => Some((radius - x, y - (height - radius - 1))),
                (_, true, _, true) => Some((x - (width - radius - 1), y - (height - radius - 1))),
                _ => None,
            };
            if let Some((distance_x, distance_y)) = corner
                && i64::from(distance_x).pow(2) + i64::from(distance_y).pow(2) > radius_squared
            {
                image.get_pixel_mut(x, y)[3] = 0;
            }
        }
    }
}

/// 将图片和柔化投影绘制到画布上。
#[allow(clippy::too_many_arguments)]
pub fn overlay_with_shadow(
    canvas: &mut RgbaImage,
    image: &RgbaImage,
    x: i64,
    y: i64,
    offset_x: i64,
    offset_y: i64,
    blur_radius: f32,
    opacity: u8,
) {
    let padding = blur_radius.ceil() as u32 * 3;
    let mut shadow = RgbaImage::from_pixel(
        image.width() + padding * 2,
        image.height() + padding * 2,
        Rgba([0, 0, 0, 0]),
    );
    for (source_x, source_y, pixel) in image.enumerate_pixels() {
        if pixel[3] == 0 {
            continue;
        }
        shadow.put_pixel(
            source_x + padding,
            source_y + padding,
            Rgba([0, 0, 0, pixel[3].min(opacity)]),
        );
    }
    let shadow = optimized_blur(&shadow, blur_radius);
    overlay(
        canvas,
        &shadow,
        x + offset_x - i64::from(padding),
        y + offset_y - i64::from(padding),
    );
    overlay(canvas, image, x, y);
}

#[allow(clippy::too_many_arguments)]
pub fn draw_titles_wrapped(
    canvas: &mut RgbaImage,
    title: &str,
    subtitle: &str,
    fonts: &Fonts,
    center_x: i32,
    title_y: i32,
    title_size: f32,
    subtitle_size: f32,
    color: Rgba<u8>,
    max_width: i32,
) {
    let title_scale = PxScale::from(title_size);
    let subtitle_scale = PxScale::from(subtitle_size);
    let subtitle_lines = wrap_text(&fonts.subtitle, subtitle_scale, subtitle, max_width);
    let title_height = line_height(&fonts.title, title_scale);
    let subtitle_height = line_height(&fonts.subtitle, subtitle_scale);

    draw_centered_text(
        canvas,
        title,
        &fonts.title,
        center_x,
        title_y,
        title_size,
        color,
    );
    let mut subtitle_y = title_y + title_height + (title_size * 0.18) as i32;
    for line in subtitle_lines {
        draw_centered_text(
            canvas,
            &line,
            &fonts.subtitle,
            center_x,
            subtitle_y,
            subtitle_size,
            color,
        );
        subtitle_y += subtitle_height + (subtitle_size * 0.22) as i32;
    }
}

#[allow(clippy::too_many_arguments)]
pub fn draw_titles_centered(
    canvas: &mut RgbaImage,
    title: &str,
    subtitle: &str,
    fonts: &Fonts,
    center_x: i32,
    center_y: i32,
    title_size: f32,
    subtitle_size: f32,
    color: Rgba<u8>,
    max_width: i32,
) {
    let title_scale = PxScale::from(title_size);
    let subtitle_scale = PxScale::from(subtitle_size);
    let subtitle_lines = wrap_text(&fonts.subtitle, subtitle_scale, subtitle, max_width);
    let title_height = line_height(&fonts.title, title_scale);
    let subtitle_height = line_height(&fonts.subtitle, subtitle_scale);
    let title_spacing = (title_size * 0.18) as i32;
    let line_spacing = (subtitle_size * 0.22) as i32;
    let subtitle_block_height = if subtitle_lines.is_empty() {
        0
    } else {
        subtitle_height * subtitle_lines.len() as i32
            + line_spacing * subtitle_lines.len().saturating_sub(1) as i32
            + title_spacing
    };
    let block_height = title_height + subtitle_block_height;

    draw_titles_wrapped(
        canvas,
        title,
        subtitle,
        fonts,
        center_x,
        center_y - block_height / 2,
        title_size,
        subtitle_size,
        color,
        max_width,
    );
}

pub fn wrap_text(font: &FontArc, scale: PxScale, text: &str, max_width: i32) -> Vec<String> {
    if text.trim().is_empty() {
        return Vec::new();
    }
    if text_width(font, scale, text) <= max_width || !text.contains(char::is_whitespace) {
        return vec![text.to_string()];
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let candidate = if current.is_empty() {
            word.to_string()
        } else {
            format!("{current} {word}")
        };
        if current.is_empty() || text_width(font, scale, &candidate) <= max_width {
            current = candidate;
        } else {
            lines.push(current);
            current = word.to_string();
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

fn line_height(font: &FontArc, scale: PxScale) -> i32 {
    let scaled = font.as_scaled(scale);
    (scaled.ascent() - scaled.descent()).ceil() as i32
}

#[allow(clippy::too_many_arguments)]
fn draw_centered_text(
    canvas: &mut RgbaImage,
    text: &str,
    font: &FontArc,
    center_x: i32,
    y: i32,
    size: f32,
    color: Rgba<u8>,
) {
    let scale = PxScale::from(size);
    let width = text_width(font, scale, text);
    let x = center_x - width / 2;
    let offset = (size * 0.045).max(2.0) as i32;
    let mut shadow_layer =
        RgbaImage::from_pixel(canvas.width(), canvas.height(), Rgba([0, 0, 0, 0]));
    for step in (2..=offset.max(2)).step_by(2) {
        draw_text_mut(
            &mut shadow_layer,
            Rgba([0, 0, 0, 96]),
            x + step,
            y + step,
            scale,
            font,
            text,
        );
    }
    let shadow_layer = optimized_blur(&shadow_layer, (size * 0.035).max(2.0));
    overlay(canvas, &shadow_layer, 0, 0);
    draw_text_mut(canvas, color, x, y, scale, font, text);
}

fn text_width(font: &FontArc, scale: PxScale, text: &str) -> i32 {
    let scaled = font.as_scaled(scale);
    text.chars()
        .map(|character| {
            let glyph = scaled.scaled_glyph(character);
            scaled.h_advance(glyph.id)
        })
        .sum::<f32>()
        .ceil() as i32
}

#[cfg(test)]
mod tests {
    use ab_glyph::FontArc;
    use image::{DynamicImage, Rgb, RgbImage, Rgba};

    use super::*;

    fn font() -> FontArc {
        FontArc::try_from_slice(include_bytes!("../../../fonts/en.otf")).unwrap()
    }

    #[test]
    fn dominant_color_ignores_black_white_and_gray() {
        let mut image = RgbImage::from_pixel(100, 100, Rgb([16, 16, 16]));
        for y in 0..70 {
            for x in 0..70 {
                image.put_pixel(x, y, Rgb([32, 128, 224]));
            }
        }
        for y in 70..100 {
            for x in 0..100 {
                image.put_pixel(x, y, Rgb([224, 224, 224]));
            }
        }

        assert_eq!(
            dominant_color(&DynamicImage::ImageRgb8(image)),
            Rgba([32, 128, 224, 255])
        );
    }

    #[test]
    fn dominant_color_uses_stable_fallback() {
        let image = DynamicImage::ImageRgb8(RgbImage::from_pixel(20, 20, Rgb([240, 240, 240])));

        assert_eq!(dominant_color(&image), Rgba([96, 96, 96, 255]));
    }

    #[test]
    fn wraps_english_title_to_available_width() {
        let font = font();
        let scale = PxScale::from(40.0);
        let width = text_width(&font, scale, "MEDIA");

        assert_eq!(
            wrap_text(&font, scale, "MEDIA LIBRARY COLLECTION", width),
            vec!["MEDIA", "LIBRARY", "COLLECTION"]
        );
        assert!(wrap_text(&font, scale, "   ", width).is_empty());
    }

    #[test]
    fn optimized_blur_preserves_dimensions() {
        let image = RgbaImage::from_pixel(1920, 1080, Rgba([20, 80, 160, 255]));

        assert_eq!(optimized_blur(&image, 50.0).dimensions(), (1920, 1080));
    }
}
