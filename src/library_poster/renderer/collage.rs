use image::{DynamicImage, Rgba, RgbaImage, imageops::overlay};
use imageproc::geometric_transformations::{Border, Interpolation, rotate_about_center};

use super::{
    Fonts, Result,
    utils::{
        apply_rounded_corners, cover, dominant_color, draw_titles_wrapped, gradient_background,
        overlay_with_shadow,
    },
};
use crate::library_poster::RenderConfig;

/// 生成左侧标题、右侧九宫格倾斜海报墙的拼贴风格。
pub fn render(
    images: &[DynamicImage],
    title: &str,
    subtitle: &str,
    fonts: &Fonts,
    _config: &RenderConfig,
    dimensions: (u32, u32),
) -> Result<image::RgbaImage> {
    let first = images.first().ok_or(super::Error::MissingImage)?;
    let (width, height) = dimensions;
    let width_f = width as f32;
    let height_f = height as f32;
    let theme = dominant_color(first);
    let mut canvas = gradient_background(width, height, theme);

    let card_width = (width_f * 0.20) as u32;
    let card_width_f = card_width as f32;
    let card_height = (card_width_f * 1.5) as u32;
    let card_height_f = card_height as f32;
    let column_spacing = width_f * 0.025;
    let row_spacing = height_f * 0.05;
    let column_stagger = card_height_f * 0.35;
    let grid_width = card_width_f * 3.0 + column_spacing * 2.0;
    let grid_height = card_height_f * 3.0 + row_spacing * 2.0;
    let start_x = width_f * 0.84 - grid_width / 2.0;
    let start_y = height_f * 0.50 - grid_height / 2.0;
    let angle = 18.0_f32.to_radians();
    let rotation_padding = card_height_f;
    let collage_width = (grid_width + rotation_padding * 2.0).ceil() as u32;
    let collage_height =
        (grid_height + column_stagger + height_f * 0.06 + rotation_padding * 2.0).ceil() as u32;

    // 参考原实现的视觉权重，把前两张素材放在更显眼的中间位置。
    let source_order = [2, 0, 4, 3, 1, 5, 8, 7, 6];
    let mut posters = (0..9)
        .map(|index| {
            let column = index / 3;
            let row = index % 3;
            let center_x =
                start_x + column as f32 * (card_width_f + column_spacing) + card_width_f / 2.0;
            let center_y = start_y
                + row as f32 * (card_height_f + row_spacing)
                + card_height_f / 2.0
                + if column % 2 == 0 {
                    -column_stagger / 2.0
                } else {
                    column_stagger / 2.0
                };
            let source = &images[source_order[index] % images.len()];
            let mut card = cover(source, card_width, card_height);
            apply_rounded_corners(&mut card, (card_width_f * 0.08) as u32);

            PosterLayer {
                card,
                center_x: center_x - start_x + rotation_padding,
                center_y: center_y - start_y + rotation_padding,
            }
        })
        .collect::<Vec<_>>();
    posters.sort_by(|left, right| {
        left.center_y
            .total_cmp(&right.center_y)
            .then_with(|| left.center_x.total_cmp(&right.center_x))
    });

    let mut collage = RgbaImage::from_pixel(collage_width, collage_height, Rgba([0, 0, 0, 0]));
    for poster in posters {
        overlay_with_shadow(
            &mut collage,
            &poster.card,
            (poster.center_x - card_width_f / 2.0) as i64,
            (poster.center_y - card_height_f / 2.0) as i64,
            (width_f * 0.008) as i64,
            (height_f * 0.014) as i64,
            height_f * 0.014,
            150,
        );
    }

    let rotated = rotate_about_center(
        &collage,
        angle,
        Interpolation::Bicubic,
        Border::Constant(Rgba([0, 0, 0, 0])),
    );
    overlay(
        &mut canvas,
        &rotated,
        (start_x - rotation_padding + collage_width as f32 / 2.0 - rotated.width() as f32 / 2.0)
            as i64,
        (start_y - rotation_padding + collage_height as f32 / 2.0 - rotated.height() as f32 / 2.0)
            as i64,
    );

    draw_titles_wrapped(
        &mut canvas,
        title,
        subtitle,
        fonts,
        (width_f * 0.25) as i32,
        (height_f * 0.35) as i32,
        height_f * 0.15,
        height_f * 0.06,
        text_color(theme),
        (width_f * 0.40) as i32,
    );
    Ok(canvas)
}

struct PosterLayer {
    card: RgbaImage,
    center_x: f32,
    center_y: f32,
}

fn text_color(color: Rgba<u8>) -> Rgba<u8> {
    let luminance =
        (0.299 * f32::from(color[0]) + 0.587 * f32::from(color[1]) + 0.114 * f32::from(color[2]))
            / 255.0;
    if luminance > 0.55 {
        Rgba([20, 20, 20, 255])
    } else {
        Rgba([255, 255, 255, 255])
    }
}
