//! Validador sintáctico y de integridad de esquema post-ofuscación (Mejora 3: Syntax & Integrity Linting).

use crate::models::{FormatType, ObfuscationError};
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use serde_json::Value;
use std::collections::HashSet;

/// Informe de validación de integridad.
#[derive(Debug, Clone)]
pub struct ValidationReport {
    pub syntax_valid: bool,
    pub schema_keys_intact: bool,
    pub original_keys_count: usize,
    pub obfuscated_keys_count: usize,
    pub issues: Vec<String>,
}

pub struct SyntaxValidator;

impl SyntaxValidator {
    /// Valida que la estructura del documento ofuscado no se haya corrompido y que las claves permanezcan idénticas.
    pub fn validate(
        original: &str,
        obfuscated: &str,
        format: FormatType,
    ) -> Result<ValidationReport, ObfuscationError> {
        match format {
            FormatType::Json => Self::validate_json(original, obfuscated),
            FormatType::Xml => Self::validate_xml(original, obfuscated),
            FormatType::Log | FormatType::Plaintext | FormatType::Auto => Ok(ValidationReport {
                syntax_valid: true,
                schema_keys_intact: true,
                original_keys_count: 0,
                obfuscated_keys_count: 0,
                issues: Vec::new(),
            }),
        }
    }

    fn validate_json(
        original: &str,
        obfuscated: &str,
    ) -> Result<ValidationReport, ObfuscationError> {
        let orig_val: Value = match serde_json::from_str(original) {
            Ok(v) => v,
            Err(_) => {
                // Si el original no era JSON válido, no validamos claves
                return Ok(ValidationReport {
                    syntax_valid: true,
                    schema_keys_intact: true,
                    original_keys_count: 0,
                    obfuscated_keys_count: 0,
                    issues: Vec::new(),
                });
            }
        };

        let obf_val: Value = serde_json::from_str(obfuscated).map_err(|e| {
            ObfuscationError::ValidationError(format!(
                "El JSON ofuscado quedó sintácticamente inválido: {e}"
            ))
        })?;

        let mut orig_keys = HashSet::new();
        let mut obf_keys = HashSet::new();

        Self::collect_json_keys(&orig_val, "", &mut orig_keys);
        Self::collect_json_keys(&obf_val, "", &mut obf_keys);

        let mut issues = Vec::new();
        let schema_intact = orig_keys == obf_keys;

        if !schema_intact {
            let missing: Vec<_> = orig_keys.difference(&obf_keys).collect();
            let added: Vec<_> = obf_keys.difference(&orig_keys).collect();
            if !missing.is_empty() {
                issues.push(format!("Claves JSON alteradas o perdidas: {missing:?}"));
            }
            if !added.is_empty() {
                issues.push(format!("Claves JSON inesperadas introducidas: {added:?}"));
            }
        }

        Ok(ValidationReport {
            syntax_valid: true,
            schema_keys_intact: schema_intact,
            original_keys_count: orig_keys.len(),
            obfuscated_keys_count: obf_keys.len(),
            issues,
        })
    }

    fn collect_json_keys(val: &Value, prefix: &str, keys: &mut HashSet<String>) {
        match val {
            Value::Object(map) => {
                for (k, v) in map {
                    let full_key = if prefix.is_empty() {
                        k.clone()
                    } else {
                        format!("{prefix}.{k}")
                    };
                    keys.insert(full_key.clone());
                    Self::collect_json_keys(v, &full_key, keys);
                }
            }
            Value::Array(arr) => {
                for (i, v) in arr.iter().enumerate() {
                    let full_key = format!("{prefix}[{i}]");
                    Self::collect_json_keys(v, &full_key, keys);
                }
            }
            _ => {}
        }
    }

    fn validate_xml(
        original: &str,
        obfuscated: &str,
    ) -> Result<ValidationReport, ObfuscationError> {
        let orig_tags = Self::extract_xml_tags(original);
        let obf_tags = Self::extract_xml_tags(obfuscated);

        let mut issues = Vec::new();
        let schema_intact = match (&orig_tags, &obf_tags) {
            (Ok(o), Ok(b)) => {
                if o == b {
                    true
                } else {
                    issues.push(
                        "Las etiquetas XML entre el original y el ofuscado difieren".to_string(),
                    );
                    false
                }
            }
            (Err(_), _) => true, // Si el original no era XML, omitir
            (_, Err(e)) => {
                return Err(ObfuscationError::ValidationError(format!(
                    "El XML ofuscado contiene errores de parseo: {e}"
                )));
            }
        };

        Ok(ValidationReport {
            syntax_valid: obf_tags.is_ok(),
            schema_keys_intact: schema_intact,
            original_keys_count: orig_tags.as_ref().map_or(0, std::vec::Vec::len),
            obfuscated_keys_count: obf_tags.as_ref().map_or(0, std::vec::Vec::len),
            issues,
        })
    }

    fn extract_xml_tags(xml_str: &str) -> Result<Vec<String>, String> {
        let mut reader = Reader::from_str(xml_str);
        reader.config_mut().trim_text(true);
        let mut tags = Vec::new();
        let mut buf = Vec::new();

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Eof) => break,
                Ok(Event::Start(e) | Event::Empty(e)) => {
                    let tag_name = String::from_utf8_lossy(e.name().as_ref()).to_string();
                    tags.push(tag_name);
                }
                Err(e) => return Err(e.to_string()),
                _ => {}
            }
            buf.clear();
        }

        Ok(tags)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_valid_json() {
        let orig = r#"{"usuario": "admin", "servidor": "10.0.0.1"}"#;
        let obf = r#"{"usuario": "user_01", "servidor": "192.0.2.1"}"#;
        let report = SyntaxValidator::validate(orig, obf, FormatType::Json).unwrap();
        assert!(report.syntax_valid);
        assert!(report.schema_keys_intact);
        assert_eq!(report.original_keys_count, 2);
    }

    #[test]
    fn test_validate_corrupted_json_fails() {
        let orig = r#"{"usuario": "admin"}"#;
        let obf = r#"{"usuario": "user_01""#; // Falta cierre
        let result = SyntaxValidator::validate(orig, obf, FormatType::Json);
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_valid_xml() {
        let orig = r"<config><admin>root</admin></config>";
        let obf = r"<config><admin>user01</admin></config>";
        let report = SyntaxValidator::validate(orig, obf, FormatType::Xml).unwrap();
        assert!(report.syntax_valid);
        assert!(report.schema_keys_intact);
    }
}
