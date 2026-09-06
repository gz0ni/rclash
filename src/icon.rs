use anyhow::Result;

pub fn decode_png_rgba(bytes: &[u8]) -> Result<(Vec<u8>, u32, u32)> {
    let decoder = png::Decoder::new(bytes);
    let mut reader = decoder.read_info()?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf)?;
    let (w, h) = (info.width, info.height);
    let rgba = match info.color_type {
        png::ColorType::Rgba => buf,
        png::ColorType::Rgb => buf
            .chunks_exact(3)
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => buf
            .chunks_exact(2)
            .flat_map(|p| [p[0], p[0], p[0], p[1]])
            .collect(),
        png::ColorType::Grayscale => buf.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        _ => anyhow::bail!("unsupported icon color type"),
    };
    Ok((rgba, w, h))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tray_icon_decodes_to_32x32_rgba() {
        let bytes = include_bytes!("../assets/icons/tray/tray-32.png");
        let (rgba, w, h) = decode_png_rgba(bytes).expect("tray icon must decode");
        assert_eq!((w, h), (32, 32));
        assert_eq!(rgba.len(), 32 * 32 * 4);
    }

    #[test]
    fn window_icon_decodes_to_256x256_rgba() {
        let bytes = include_bytes!("../assets/icons/app/app-256.png");
        let (rgba, w, h) = decode_png_rgba(bytes).expect("window icon must decode");
        assert_eq!((w, h), (256, 256));
        assert_eq!(rgba.len(), 256 * 256 * 4);
    }
}
