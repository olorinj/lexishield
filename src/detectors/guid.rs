//! Detector y generador de identificadores GUID / UUID.

use crate::models::DetectorType;
use once_cell::sync::Lazy;
use regex::Regex;
use uuid::Uuid;

static GUID_REGEX: Lazy<Regex> = Lazy::new(|| {
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
