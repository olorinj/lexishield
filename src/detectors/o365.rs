//! Detectores para escenarios específicos de Office 365 y nombres de archivos adjuntos.

use crate::models::DetectorType;
use rand::Rng;
use regex::Regex;
use std::sync::LazyLock;

static SUBJECT_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)"Subject"\s*:\s*"([^"]+)""#).expect("Regex Subject inválida")
});

static ORIGINATING_SERVER_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)"OriginatingServer"\s*:\s*"([a-zA-Z0-9.-]+)""#)
        .expect("Regex OriginatingServer inválida")
});

static ATTACHMENT_KEYWORDS_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\b(?:archivo|fichero|file|attachment|adjunto|documento|document)\s*:?\s+((?:[\p{L}\p{M}\p{N}_\-]+\s+)*[\p{L}\p{M}\p{N}_\-]+\.(?:docx?|xlsx?|pdf|pptx?|jpg|png|zip|rar|7z|txt|csv|md|markdown))\b",
    )
    .expect("Regex Attachment Keywords inválida")
});

static ATTACHMENT_QUOTED_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?i)["'`]((?:[\p{L}\p{M}\p{N}_\-\s]{2,})\.(?:docx?|xlsx?|pdf|pptx?|jpg|png|zip|rar|7z|txt|csv|md|markdown))["'`]"#,
    )
    .expect("Regex Attachment Quoted inválida")
});

static ATTACHMENT_DELIMITED_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)(?:;\s*)((?:[\p{L}\p{M}\p{N}_\-]+\s+)*[\p{L}\p{M}\p{N}_\-]+\.(?:docx?|xlsx?|pdf|pptx?|jpg|png|zip|rar|7z|txt|csv|md|markdown))\b",
    )
    .expect("Regex Attachment Delimited inválida")
});

static ATTACHMENT_TOKEN_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\b([\p{L}\p{M}\p{N}_\-]{2,}\.(?:docx?|xlsx?|pdf|pptx?|jpg|png|zip|rar|7z|txt|csv|md|markdown))\b",
    )
    .expect("Regex Attachment Token inválida")
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
        let id: u32 = rng.gen_range(0x1000_0000..0xFFFF_FFFF);
        let topic = topics[rng.gen_range(0..topics.len())];
        format!("{topic} [REF-{id:08X}]")
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
        let id: u32 = rng.gen_range(0x10_0000..0xFF_FFFF);
        format!("EURPRD{id:06X}PROD.outlook.example.com")
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
        let mut matches = Vec::new();

        // 1. Archivos precedidos por palabras clave (ej: "archivo Informe de auditoría.docx")
        for cap in ATTACHMENT_KEYWORDS_REGEX.captures_iter(text) {
            if let Some(m) = cap.get(1) {
                matches.push((m.start(), m.end(), m.as_str()));
            }
        }

        // 2. Archivos entre comillas (ej: "Documento Confidencial.docx")
        for cap in ATTACHMENT_QUOTED_REGEX.captures_iter(text) {
            if let Some(m) = cap.get(1) {
                matches.push((m.start(), m.end(), m.as_str()));
            }
        }

        // 3. Archivos en listas delimitadas por punto y coma (ej: "image.png; File 01.xlsx")
        for cap in ATTACHMENT_DELIMITED_REGEX.captures_iter(text) {
            if let Some(m) = cap.get(1) {
                matches.push((m.start(), m.end(), m.as_str()));
            }
        }

        // 4. Tokens individuales sin espacios (ej: "reporte_anual_2024.pdf")
        for m in ATTACHMENT_TOKEN_REGEX.find_iter(text) {
            matches.push((m.start(), m.end(), m.as_str()));
        }

        // Deduplicar y ordenar por posición y longitud para resolver posibles solapamientos
        matches.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| b.1.cmp(&a.1)));
        let mut filtered: Vec<(usize, usize, &'a str)> = Vec::new();
        for (start, end, val) in matches {
            if let Some(last) = filtered.last()
                && start < last.1
            {
                continue; // Solapamiento con coincidencia previa de mayor especificidad
            }
            filtered.push((start, end, val));
        }

        filtered
    }

    pub fn generate_pseudonym(&self, original: &str) -> String {
        let mut rng = rand::thread_rng();
        let id: u32 = rng.gen_range(0x1000_0000..0xFFFF_FFFF);
        let ext = original.rsplit('.').next().unwrap_or("dat");
        format!("attachment_{id:08x}.{ext}")
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
    #[allow(clippy::case_sensitive_file_extension_comparisons)]
    fn test_attachment_filename_detector() {
        let detector = AttachmentFileNameDetector::new();
        let text = "Descargar reporte_anual_2024.pdf y nominas.xlsx";
        let matches = detector.find_matches(text);
        assert_eq!(matches.len(), 2);
        let pseudo = detector.generate_pseudonym(matches[0].2);
        assert!(pseudo.starts_with("attachment_"));
        assert!(pseudo.ends_with(".pdf"));

        // Prueba con caso real que contiene tilde diacrítica en forma combinada Unicode (NFD)
        let unicode_text = "Iniciando trabajo fa7ba0ab-c413-4633-9663-e6470c6fbf48 para archivo Informe de auditori\u{0301}a.docx.";
        let unicode_matches = detector.find_matches(unicode_text);
        assert_eq!(unicode_matches.len(), 1);
        assert_eq!(unicode_matches[0].2, "Informe de auditori\u{0301}a.docx");
        let unicode_pseudo = detector.generate_pseudonym(unicode_matches[0].2);
        assert!(unicode_pseudo.ends_with(".docx"));
    }
}
