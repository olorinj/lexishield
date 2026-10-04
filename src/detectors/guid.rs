//! Detector y generador de identificadores GUID / UUID.

use crate::models::DetectorType;
use regex::Regex;
use std::sync::LazyLock;
use uuid::Uuid;

static GUID_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}\b")
        .expect("Regex de GUID inválida")
});

#[derive(Default)]
pub struct GuidDetector;

impl GuidDetector {
    pub fn new() -> Self {
        Self
    }

    pub fn detector_type(&self) -> DetectorType {
        DetectorType::GuidUuid
    }

    pub fn find_matches<'a>(&self, text: &'a str) -> Vec<(usize, usize, &'a str)> {
        GUID_REGEX
            .find_iter(text)
            .map(|m| (m.start(), m.end(), m.as_str()))
            .collect()
    }

    /// Genera un UUID versión 4 sintácticamente válido respetando la capitalización original.
    pub fn generate_pseudonym(&self, original: &str) -> String {
        let new_uuid = Uuid::new_v4().to_string();
        if original.chars().any(|c| c.is_ascii_uppercase())
            && !original.chars().any(|c| c.is_ascii_lowercase())
        {
            new_uuid.to_ascii_uppercase()
        } else {
            new_uuid.to_ascii_lowercase()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guid_detection_and_case_preservation() {
        let detector = GuidDetector::new();
        let lower = "c9a646d3-9c61-4cb7-897d-4b958c218a56";
        let upper = "C9A646D3-9C61-4CB7-897D-4B958C218A56";

        let text = format!("Lower: {} Upper: {}", lower, upper);
        let matches = detector.find_matches(&text);
        assert_eq!(matches.len(), 2);

        let pseudo_lower = detector.generate_pseudonym(lower);
        assert_eq!(pseudo_lower, pseudo_lower.to_ascii_lowercase());

        let pseudo_upper = detector.generate_pseudonym(upper);
        assert_eq!(pseudo_upper, pseudo_upper.to_ascii_uppercase());
    }
}
