//! Detectores específicos para identificadores de Windows (SIDs, Logon IDs, Hex IDs).

use crate::models::DetectorType;
use once_cell::sync::Lazy;
use rand::Rng;
use regex::Regex;

static SID_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\bS-1-(?:[0-59]|16)(?:-\d+)+\b").expect("Regex de SID inválida"));

static LOGON_ID_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\b0x[0-9a-f]{4,8}\b").expect("Regex de Logon ID inválida"));

static HEX_ID_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\b0x[0-9a-f]{9,}\b").expect("Regex de Hex ID inválida"));

#[derive(Default)]
pub struct WindowsSidDetector;

impl WindowsSidDetector {
    pub fn new() -> Self {
        Self
    }

    pub fn detector_type(&self) -> DetectorType {
        DetectorType::WindowsSid
    }

    pub fn find_matches<'a>(&self, text: &'a str) -> Vec<(usize, usize, &'a str)> {
        SID_REGEX
            .find_iter(text)
            .map(|m| (m.start(), m.end(), m.as_str()))
            .collect()
    }

    /// Genera un SID de Windows sintácticamente válido pero anonimizado.
    pub fn generate_pseudonym(&self, _original: &str) -> String {
        let mut rng = rand::thread_rng();
        let sub1: u32 = rng.gen_range(100_000_000..999_999_999);
        let sub2: u32 = rng.gen_range(100_000_000..999_999_999);
        let sub3: u32 = rng.gen_range(100_000_000..999_999_999);
        let rid: u32 = rng.gen_range(1000..9999);
        format!("S-1-5-21-{}-{}-{}-{}", sub1, sub2, sub3, rid)
    }
}

#[derive(Default)]
pub struct WindowsLogonIdDetector;

impl WindowsLogonIdDetector {
    pub fn new() -> Self {
        Self
    }

    pub fn detector_type(&self) -> DetectorType {
        DetectorType::WindowsLogonId
    }

    pub fn find_matches<'a>(&self, text: &'a str) -> Vec<(usize, usize, &'a str)> {
        LOGON_ID_REGEX
            .find_iter(text)
            .map(|m| (m.start(), m.end(), m.as_str()))
            .collect()
    }

    pub fn generate_pseudonym(&self, original: &str) -> String {
        let mut rng = rand::thread_rng();
        let len = original
            .trim_start_matches("0x")
            .trim_start_matches("0X")
            .len()
            .max(4);
        let val: u32 = rng.gen_range(0x1000..0xFFFF_FFFF);
        let is_upper = original.chars().skip(2).any(|c| c.is_ascii_uppercase());
        if is_upper {
            format!("0x{:0len$X}", val, len = len)
        } else {
            format!("0x{:0len$x}", val, len = len)
        }
    }
}

#[derive(Default)]
pub struct WindowsHexIdDetector;

impl WindowsHexIdDetector {
    pub fn new() -> Self {
        Self
    }

    pub fn detector_type(&self) -> DetectorType {
        DetectorType::WindowsHexId
    }

    pub fn find_matches<'a>(&self, text: &'a str) -> Vec<(usize, usize, &'a str)> {
        HEX_ID_REGEX
            .find_iter(text)
            .map(|m| (m.start(), m.end(), m.as_str()))
            .collect()
    }

    pub fn generate_pseudonym(&self, original: &str) -> String {
        let mut rng = rand::thread_rng();
        let hex_len = original
            .trim_start_matches("0x")
            .trim_start_matches("0X")
            .len();
        let is_upper = original.chars().skip(2).any(|c| c.is_ascii_uppercase());
        let mut hex_str = String::with_capacity(hex_len);
        for _ in 0..hex_len {
            let nibble: u8 = rng.gen_range(0..16);
            if is_upper {
                hex_str.push_str(&format!("{:X}", nibble));
            } else {
                hex_str.push_str(&format!("{:x}", nibble));
            }
        }
        format!("0x{}", hex_str)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_windows_sid_detector() {
        let detector = WindowsSidDetector::new();
        let sid = "S-1-5-21-123456789-987654321-112233445-1001";
        let text = format!("Usuario con SID: {}", sid);
        let matches = detector.find_matches(&text);
        assert_eq!(matches.len(), 1);
        let pseudo = detector.generate_pseudonym(sid);
        assert!(pseudo.starts_with("S-1-5-21-"));
    }

    #[test]
    fn test_windows_logon_id_detector() {
        let detector = WindowsLogonIdDetector::new();
        let matches = detector.find_matches("Logon ID: 0x03e7 y sesión 0x1A2B3C");
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn test_windows_hex_id_detector() {
        let detector = WindowsHexIdDetector::new();
        let matches = detector.find_matches("Handle 0x00000000A1B2C3D4 detectado");
        assert_eq!(matches.len(), 1);
        let pseudo = detector.generate_pseudonym("0x00000000A1B2C3D4");
        assert!(pseudo.starts_with("0x"));
    }
}
