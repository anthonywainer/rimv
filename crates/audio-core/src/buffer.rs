/// Signed PCM normalization. Negative full scale is exactly -1.
pub fn normalize_i16(sample: i16) -> f32 {
    sample as f32 / 32768.0
}
/// Unsigned PCM is centered on 32768 (digital silence).
pub fn normalize_u16(sample: u16) -> f32 {
    (sample as f32 - 32768.0) / 32768.0
}
/// Keep finite float PCM unchanged (including occasional values outside [-1,1]).
/// Replace invalid floating-point values with silence.
pub fn normalize_f32(sample: f32) -> f32 {
    if sample.is_finite() { sample } else { 0.0 }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pcm_boundaries_and_silence() {
        assert_eq!(normalize_i16(i16::MIN), -1.0);
        assert_eq!(normalize_i16(0), 0.0);
        assert_eq!(normalize_i16(i16::MAX), 32767.0 / 32768.0);
        assert_eq!(normalize_u16(0), -1.0);
        assert_eq!(normalize_u16(32768), 0.0);
        assert_eq!(normalize_u16(u16::MAX), 32767.0 / 32768.0);
        assert_eq!(normalize_f32(f32::NAN), 0.0);
        assert_eq!(normalize_f32(f32::INFINITY), 0.0);
        assert_eq!(normalize_f32(1.2), 1.2);
    }
}
