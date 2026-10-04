//! Detectores para escenarios específicos de Office 365 y nombres de archivos adjuntos.

use crate::models::DetectorType;
use once_cell::sync::Lazy;
use rand::Rng;
use regex::Regex;

static SUBJECT_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r#"(?i)"Subject"\s*:\s*"([^"]+)""#).expect("Regex Subject inválida"));

static ORIGINATING_SERVER_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"(?i)"OriginatingServer"\s*:\s*"([a-zA-Z0-9.-]+)""#)
        .expect("Regex OriginatingServer inválida")
});

static ATTACHMENT_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r#"(?i)\b([a-zA-Z0-9_\-\s]{3,}\.(?:docx?|xlsx?|pdf|pptx?|jpg|png|zip|rar|7z|txt|csv))\b"#,
    )
    .expect("Regex Attachment inválida")
});

#[derive(Default)]
pub struct O365SubjectDetector;

impl O365SubjectDetector {
    pub fn new() -> Self {
        Self
    }

    pub fn detector_type(&self) -> DetectorType {
        DetectorType::O365Subject
    }

    /// Extrae las capturas internas del valor del Subject (sin incluir la clave "Subject":).
    pub fn find_matches<'a>(&self, text: &'a str) -> Vec<(usize, usize, &'a str)> {
        SUBJECT_REGEX
            .captures_iter(text)
            .filter_map(|cap| cap.get(1).map(|m| (m.start(), m.end(), m.as_str())))
            .collect()
    }

    pub fn generate_pseudonym(&self, _original: &str) -> String {
        let mut rng = rand::thread_rng();
        let topics = [
            "Reporte confidencial",
            "Actualización de seguridad",
            "Notificación de servicio",
            "Resumen de auditoría",
            "Informe de operaciones",
            "Revisión de accesos",
        ];
        let id: u64 = rng.gen_range(100_000..999_999_999);
        let topic = topics[rng.gen_range(0..topics.len())];
        format!("{} #{}", topic, id)
    }
}

#[derive(Default)]
pub struct O365OriginatingServerDetector;

impl O365OriginatingServerDetector {
    pub fn new() -> Self {
        Self
    }

    pub fn detector_type(&self) -> DetectorType {
        DetectorType::O365OriginatingServer
    }

    pub fn find_matches<'a>(&self, text: &'a str) -> Vec<(usize, usize, &'a str)> {
        ORIGINATING_SERVER_REGEX
            .captures_iter(text)
            .filter_map(|cap| cap.get(1).map(|m| (m.start(), m.end(), m.as_str())))
            .collect()
    }

    pub fn generate_pseudonym(&self, _original: &str) -> String {
        let mut rng = rand::thread_rng();
        let id: u64 = rng.gen_range(100_000..999_999);
        format!("EURPRD{:06}PROD.outlook.example.com", id)
    }
}

#[derive(Default)]
pub struct AttachmentFileNameDetector;

impl AttachmentFileNameDetector {
    pub fn new() -> Self {
        Self
    }

    pub fn detector_type(&self) -> DetectorType {
        DetectorType::AttachmentFileName
    }

    pub fn find_matches<'a>(&self, text: &'a str) -> Vec<(usize, usize, &'a str)> {
        ATTACHMENT_REGEX
            .find_iter(text)
            .map(|m| (m.start(), m.end(), m.as_str()))
            .collect()
    }

    pub fn generate_pseudonym(&self, original: &str) -> String {
        let mut rng = rand::thread_rng();
        let id: u64 = rng.gen_range(100_000_000..999_999_999);
        let ext = original.rsplit('.').next().unwrap_or("dat");
        format!("attachment_{}.{}", id, ext)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_o365_subject_detector() {
        let detector = O365SubjectDetector::new();
        let text = r#"Log: "Subject": "Planilla de sueldos Q3" del usuario"#;
        let matches = detector.find_matches(text);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].2, "Planilla de sueldos Q3");
    }

    #[test]
    fn test_o365_originating_server_detector() {
        let detector = O365OriginatingServerDetector::new();
        let text = r#"Servidor: "OriginatingServer": "AM6EUR05FT022.eop-eur05.prod.protection.outlook.com""#;
        let matches = detector.find_matches(text);
        assert_eq!(matches.len(), 1);
        assert_eq!(
            matches[0].2,
            "AM6EUR05FT022.eop-eur05.prod.protection.outlook.com"
        );
    }

    #[test]
    fn test_attachment_filename_detector() {
        let detector = AttachmentFileNameDetector::new();
        let text = "Descargar reporte_anual_2024.pdf y nominas.xlsx";
        let matches = detector.find_matches(text);
        assert_eq!(matches.len(), 2);
        let pseudo = detector.generate_pseudonym(matches[0].2);
        assert!(pseudo.starts_with("attachment_"));
        assert!(pseudo.ends_with(".pdf"));
    }
}
