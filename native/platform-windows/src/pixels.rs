//! Bounded BGRA -> NV12 conversion for the attended Windows capture path.
pub fn dimensions(width: u32, height: u32) -> Option<(u32, u32)> {
    if !(2..=8192).contains(&width) || !(2..=8192).contains(&height) {
        return None;
    }
    let scale = (1280.0 / width as f64).min(720.0 / height as f64).min(1.0);
    Some((
        ((width as f64 * scale) as u32 / 2 * 2).max(2),
        ((height as f64 * scale) as u32 / 2 * 2).max(2),
    ))
}
pub fn nv12(
    bgra: &[u8],
    width: u32,
    height: u32,
    out_width: u32,
    out_height: u32,
) -> Option<Vec<u8>> {
    dimensions(width, height)?;
    if out_width < 2
        || out_height < 2
        || out_width > 1280
        || out_height > 720
        || !out_width.is_multiple_of(2)
        || !out_height.is_multiple_of(2)
        || bgra.len() != width as usize * height as usize * 4
    {
        return None;
    }
    let (w, h) = (out_width as usize, out_height as usize);
    let mut data = vec![0; w * h * 3 / 2];
    let rgb = |x: usize, y: usize| {
        let sx = x * width as usize / w;
        let sy = y * height as usize / h;
        let p = (sy * width as usize + sx) * 4;
        (bgra[p + 2] as i32, bgra[p + 1] as i32, bgra[p] as i32)
    };
    let clamp = |v: i32| v.clamp(0, 255) as u8;
    for y in 0..h {
        for x in 0..w {
            let (r, g, b) = rgb(x, y);
            data[y * w + x] = clamp(((66 * r + 129 * g + 25 * b + 128) >> 8) + 16);
        }
    }
    for y in (0..h).step_by(2) {
        for x in (0..w).step_by(2) {
            let (mut r, mut g, mut b) = (0, 0, 0);
            for dy in 0..2 {
                for dx in 0..2 {
                    let p = rgb(x + dx, y + dy);
                    r += p.0;
                    g += p.1;
                    b += p.2;
                }
            }
            r /= 4;
            g /= 4;
            b /= 4;
            let p = w * h + (y / 2) * w + x;
            data[p] = clamp(((-38 * r - 74 * g + 112 * b + 128) >> 8) + 128);
            data[p + 1] = clamp(((112 * r - 94 * g - 18 * b + 128) >> 8) + 128);
        }
    }
    Some(data)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn limits_and_known_black_white_nv12() {
        assert_eq!(dimensions(1920, 1080), Some((1280, 720)));
        assert_eq!(dimensions(1080, 1920), Some((404, 720)));
        assert_eq!(dimensions(0, 100), None);
        assert_eq!(
            nv12(&[0; 16], 2, 2, 2, 2),
            Some(vec![16, 16, 16, 16, 128, 128])
        );
        assert_eq!(
            nv12(&[255; 16], 2, 2, 2, 2),
            Some(vec![235, 235, 235, 235, 128, 128])
        );
        assert!(nv12(&[0; 15], 2, 2, 2, 2).is_none());
        assert!(nv12(&[0; 16], 2, 2, 3, 2).is_none());
    }
}
