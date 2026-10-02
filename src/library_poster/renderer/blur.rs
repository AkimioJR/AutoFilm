use image::{DynamicImage, Rgba};

use super::{
    Fonts, Result,
    utils::{adjust_brightness, cover, dominant_color, draw_titles_centered, optimized_blur, tint},
};
use crate::library_poster::RenderConfig;

/// 生成以首张素材为全屏模糊背景、标题居中的极简风格。
pub fn render(
    images: &[DynamicImage],
    title: &str,
    subtitle: &str,
    fonts: &Fonts,
    config: &RenderConfig,
    dimensions: (u32, u32),
) -> Result<image::RgbaImage> {
    let source = images.first().ok_or(super::Error::MissingImage)?;
    let (width, height) = dimensions;
    let width_f = width as f32;
    let height_f = height as f32;
    let theme = dominant_color(source);
    let mut canvas = cover(source, width, height);
    canvas = optimized_blur(&canvas, config.blur_radius.max(8.0) * height_f / 1080.0);
    tint(
        &mut canvas,
        adjust_brightness(theme, 0.82),
        config.color_strength.clamp(0.0, 1.0),
    );

    draw_titles_centered(
        &mut canvas,
        title,
        subtitle,
        fonts,
        i32::try_from(width / 2).unwrap_or(i32::MAX),
        i32::try_from(height / 2).unwrap_or(i32::MAX),
        height_f * 0.15,
        height_f * 0.065,
        Rgba([255, 255, 255, 235]),
        (width_f * 0.48) as i32,
    );
    Ok(canvas)
}
