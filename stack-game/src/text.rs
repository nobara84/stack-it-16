use macroquad::prelude::*;

const GLYPH_WIDTH: f32 = 5.0;
pub const GLYPH_HEIGHT: f32 = 7.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bounds {
    pub left: f32,
    pub top: f32,
    pub right: f32,
    pub bottom: f32,
}

pub fn draw(text: &str, x: f32, y: f32, scale: f32, color: Color) {
    let mut cursor = x;
    for character in text.chars() {
        if character == ' ' {
            cursor += (GLYPH_WIDTH + 2.0) * scale;
            continue;
        }
        let glyph = glyph(character);
        for (row, bits) in glyph.iter().enumerate() {
            for column in 0..5 {
                if bits & (1 << (4 - column)) != 0 {
                    draw_rectangle(
                        cursor + column as f32 * scale,
                        y + row as f32 * scale,
                        scale,
                        scale,
                        color,
                    );
                }
            }
        }
        cursor += (GLYPH_WIDTH + 1.0) * scale;
    }
}

pub fn width(text: &str, scale: f32) -> f32 {
    let mut cursor = 0.0;
    let mut visual_right = 0.0;
    for character in text.chars() {
        if character == ' ' {
            cursor += (GLYPH_WIDTH + 2.0) * scale;
        } else {
            visual_right = cursor + GLYPH_WIDTH * scale;
            cursor += (GLYPH_WIDTH + 1.0) * scale;
        }
    }
    visual_right
}

pub fn bounds(text: &str, x: f32, y: f32, scale: f32) -> Bounds {
    Bounds {
        left: x,
        top: y,
        right: x + width(text, scale),
        bottom: y + GLYPH_HEIGHT * scale,
    }
}

pub fn centered_x(text: &str, viewport_width: f32, scale: f32) -> f32 {
    (viewport_width - width(text, scale)) * 0.5
}

pub fn right_aligned_x(text: &str, right_edge: f32, scale: f32) -> f32 {
    right_edge - width(text, scale)
}

pub fn fit_scale(text: &str, max_width: f32, preferred: f32, minimum: f32) -> f32 {
    let unscaled_width = width(text, 1.0);
    if unscaled_width <= f32::EPSILON {
        return preferred;
    }
    let fitted = (max_width / unscaled_width).min(preferred);
    fitted.max(minimum.min(fitted))
}

pub fn line_height(scale: f32) -> f32 {
    (GLYPH_HEIGHT + 2.0) * scale
}

pub fn clamp_x(x: f32, text_width: f32, viewport_width: f32, margin: f32) -> f32 {
    x.clamp(margin, (viewport_width - margin - text_width).max(margin))
}

fn glyph(character: char) -> [u8; 7] {
    let character = match character {
        'ä' | 'Ä' => 'Ä',
        'ü' | 'Ü' => 'Ü',
        other => other.to_ascii_uppercase(),
    };
    match character {
        'A' => [0x0E, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        'B' => [0x1E, 0x11, 0x11, 0x1E, 0x11, 0x11, 0x1E],
        'C' => [0x0F, 0x10, 0x10, 0x10, 0x10, 0x10, 0x0F],
        'D' => [0x1E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x1E],
        'E' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x1F],
        'F' => [0x1F, 0x10, 0x10, 0x1E, 0x10, 0x10, 0x10],
        'G' => [0x0F, 0x10, 0x10, 0x17, 0x11, 0x11, 0x0F],
        'H' => [0x11, 0x11, 0x11, 0x1F, 0x11, 0x11, 0x11],
        'I' => [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x1F],
        'J' => [0x01, 0x01, 0x01, 0x01, 0x11, 0x11, 0x0E],
        'K' => [0x11, 0x12, 0x14, 0x18, 0x14, 0x12, 0x11],
        'L' => [0x10, 0x10, 0x10, 0x10, 0x10, 0x10, 0x1F],
        'M' => [0x11, 0x1B, 0x15, 0x15, 0x11, 0x11, 0x11],
        'N' => [0x11, 0x19, 0x15, 0x13, 0x11, 0x11, 0x11],
        'O' => [0x0E, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        'P' => [0x1E, 0x11, 0x11, 0x1E, 0x10, 0x10, 0x10],
        'Q' => [0x0E, 0x11, 0x11, 0x11, 0x15, 0x12, 0x0D],
        'R' => [0x1E, 0x11, 0x11, 0x1E, 0x14, 0x12, 0x11],
        'S' => [0x0F, 0x10, 0x10, 0x0E, 0x01, 0x01, 0x1E],
        'T' => [0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04],
        'U' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x11, 0x0E],
        'V' => [0x11, 0x11, 0x11, 0x11, 0x11, 0x0A, 0x04],
        'W' => [0x11, 0x11, 0x11, 0x15, 0x15, 0x1B, 0x11],
        'X' => [0x11, 0x11, 0x0A, 0x04, 0x0A, 0x11, 0x11],
        'Y' => [0x11, 0x11, 0x0A, 0x04, 0x04, 0x04, 0x04],
        'Z' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x10, 0x1F],
        'Ä' => [0x0A, 0x00, 0x0E, 0x11, 0x1F, 0x11, 0x11],
        'Ü' => [0x0A, 0x00, 0x11, 0x11, 0x11, 0x11, 0x0E],
        '0' => [0x0E, 0x11, 0x13, 0x15, 0x19, 0x11, 0x0E],
        '1' => [0x04, 0x0C, 0x04, 0x04, 0x04, 0x04, 0x0E],
        '2' => [0x0E, 0x11, 0x01, 0x02, 0x04, 0x08, 0x1F],
        '3' => [0x1E, 0x01, 0x01, 0x0E, 0x01, 0x01, 0x1E],
        '4' => [0x02, 0x06, 0x0A, 0x12, 0x1F, 0x02, 0x02],
        '5' => [0x1F, 0x10, 0x10, 0x1E, 0x01, 0x01, 0x1E],
        '6' => [0x0E, 0x10, 0x10, 0x1E, 0x11, 0x11, 0x0E],
        '7' => [0x1F, 0x01, 0x02, 0x04, 0x08, 0x08, 0x08],
        '8' => [0x0E, 0x11, 0x11, 0x0E, 0x11, 0x11, 0x0E],
        '9' => [0x0E, 0x11, 0x11, 0x0F, 0x01, 0x01, 0x0E],
        '+' => [0x00, 0x04, 0x04, 0x1F, 0x04, 0x04, 0x00],
        '-' => [0x00, 0x00, 0x00, 0x1F, 0x00, 0x00, 0x00],
        ':' => [0x00, 0x04, 0x04, 0x00, 0x04, 0x04, 0x00],
        '!' => [0x04, 0x04, 0x04, 0x04, 0x04, 0x00, 0x04],
        '.' => [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04],
        _ => [0; 7],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bounds_match_the_pixel_font_and_exclude_trailing_advance() {
        let bounds = bounds("AB", 10.0, 20.0, 2.0);
        assert_eq!(bounds.left, 10.0);
        assert_eq!(bounds.top, 20.0);
        assert_eq!(bounds.right, 32.0);
        assert_eq!(bounds.bottom, 34.0);
    }

    #[test]
    fn alignment_helpers_keep_german_labels_inside_the_viewport() {
        for value in ["DANEBEN!", "WIEDERHOLUNG", "SPIEL VORBEI", "REKORD"] {
            let scale = fit_scale(value, 180.0, 2.0, 0.75);
            let centered = centered_x(value, 320.0, scale);
            let right = right_aligned_x(value, 315.0, scale);
            assert!(bounds(value, centered, 0.0, scale).left >= 0.0);
            assert!(bounds(value, centered, 0.0, scale).right <= 320.0);
            assert!(bounds(value, right, 0.0, scale).right <= 315.0);
        }
    }

    #[test]
    fn line_height_prevents_stacked_text_overlap() {
        let first = bounds("WIEDERHOLUNG", 50.0, 40.0, 1.0);
        let second = bounds("ZEITLUPE", 50.0, 40.0 + line_height(1.0), 1.0);
        assert!(first.bottom <= second.top);
    }
}
