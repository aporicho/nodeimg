pub fn format_color_hex(rgba: [f32; 4]) -> String {
    format!(
        "#{:02X}{:02X}{:02X}",
        color_channel(rgba[0]),
        color_channel(rgba[1]),
        color_channel(rgba[2])
    )
}

pub(crate) fn parse_color_hex(text: &str, alpha: f32) -> Option<[f32; 4]> {
    let text = text.trim();
    let hex = text.strip_prefix('#').unwrap_or(text);
    if hex.len() != 6 || !hex.is_ascii() {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some([
        f32::from(r) / 255.0,
        f32::from(g) / 255.0,
        f32::from(b) / 255.0,
        alpha.clamp(0.0, 1.0),
    ])
}

fn color_channel(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_hex_rounds_and_clamps_channels() {
        assert_eq!(format_color_hex([1.0, 0.5, -1.0, 1.0]), "#FF8000");
    }

    #[test]
    fn parse_color_hex_accepts_prefixed_and_plain_text() {
        assert_eq!(
            parse_color_hex("#FF8000", 0.4),
            Some([1.0, 128.0 / 255.0, 0.0, 0.4])
        );
        assert_eq!(
            parse_color_hex("336699", 1.0),
            Some([51.0 / 255.0, 102.0 / 255.0, 153.0 / 255.0, 1.0])
        );
    }

    #[test]
    fn parse_color_hex_rejects_invalid_text() {
        assert_eq!(parse_color_hex("#12345", 1.0), None);
        assert_eq!(parse_color_hex("#GG0000", 1.0), None);
    }
}
