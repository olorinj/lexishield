//! Adaptador para formato SAP IDoc delimitado por caracteres de control ASCII:
//! - `^A` (`0x01`): Separador de campo
//! - `^B` (`0x02`): Separador de segmento
//! - `^C` (`0x03`): Separador de registro

use crate::engine::replacer::apply_single_pass_replacements_with_substring_keys;
use crate::models::ObfuscationError;
use std::collections::{HashMap, HashSet};

/// Separador de campo SAP IDoc (`^A`, 0x01).
pub const FIELD_SEP: char = '\x01';
/// Separador de segmento SAP IDoc (`^B`, 0x02).
pub const SEGMENT_SEP: char = '\x02';
/// Separador de registro SAP IDoc (`^C`, 0x03).
pub const RECORD_SEP: char = '\x03';

#[derive(Default)]
pub struct IdocAdapter;

impl IdocAdapter {
    pub fn new() -> Self {
        Self
    }

    /// Detecta si el contenido contiene delimitadores de control propios de SAP IDoc (`0x01`, `0x02`, `0x03`).
    #[must_use]
    pub fn is_idoc_format(content: &str) -> bool {
        content.contains(FIELD_SEP) || content.contains(SEGMENT_SEP) || content.contains(RECORD_SEP)
    }

    /// Procesa una trama SAP IDoc dividiendo por registros (`^C`), segmentos (`^B`) y campos (`^A`),
    /// aplicando sustituciones dentro de cada campo y preservando los caracteres de control intactos.
    pub fn process_idoc_with_substring_keys(
        &self,
        content: &str,
        replacement_map: &HashMap<String, String>,
        strict_word_boundaries: bool,
        substring_keys: &HashSet<String>,
    ) -> Result<(String, usize), ObfuscationError> {
        let mut total_count = 0usize;
        let mut out = String::with_capacity(content.len());

        for (rec_idx, record) in content.split(RECORD_SEP).enumerate() {
            if rec_idx > 0 {
                out.push(RECORD_SEP);
            }
            for (seg_idx, segment) in record.split(SEGMENT_SEP).enumerate() {
                if seg_idx > 0 {
                    out.push(SEGMENT_SEP);
                }
                for (fld_idx, field) in segment.split(FIELD_SEP).enumerate() {
                    if fld_idx > 0 {
                        out.push(FIELD_SEP);
                    }
                    if field.is_empty() {
                        continue;
                    }
                    let (replaced, count) = apply_single_pass_replacements_with_substring_keys(
                        field,
                        replacement_map,
                        strict_word_boundaries,
                        substring_keys,
                    );
                    out.push_str(&replaced);
                    total_count += count;
                }
            }
        }

        Ok((out, total_count))
    }

    /// Convierte una trama SAP IDoc con caracteres de control (`^A`, `^B`, `^C`) a una
    /// representación en texto plano legible (campos separados por ` | `, segmentos y registros por saltos de línea).
    #[must_use]
    pub fn format_plain_readable(content: &str) -> String {
        let mut out = String::with_capacity(content.len() + 32);
        let mut chars = content.chars().peekable();

        while let Some(ch) = chars.next() {
            match ch {
                FIELD_SEP => out.push_str(" | "),
                SEGMENT_SEP => {
                    if !matches!(chars.peek(), Some('\n' | '\r')) {
                        out.push('\n');
                    }
                }
                RECORD_SEP => {
                    if !out.ends_with('\n') && !out.is_empty() {
                        out.push('\n');
                    }
                    if !matches!(chars.peek(), Some('\n' | '\r')) && chars.peek().is_some() {
                        out.push('\n');
                    }
                }
                other => out.push(other),
            }
        }

        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_idoc_format_detection() {
        assert!(IdocAdapter::is_idoc_format(
            "EDI_DC40\x010000001\x02E2EDK01\x03"
        ));
        assert!(IdocAdapter::is_idoc_format("CAMPO1\x01CAMPO2"));
        assert!(!IdocAdapter::is_idoc_format("{\"key\": \"value\"}"));
        assert!(!IdocAdapter::is_idoc_format("Texto plano normal"));
    }

    #[test]
    fn test_idoc_process_preserves_control_delimiters_and_roundtrip() {
        let adapter = IdocAdapter::new();
        let input = "EDI_DC40\x01192.168.1.50\x01user@empresa.es\x02E2EDKA1\x0112345678Z\x03EDI_DC40\x0110.0.0.1";

        let mut fwd_map = HashMap::new();
        fwd_map.insert("192.168.1.50".to_string(), "10.99.0.1".to_string());
        fwd_map.insert(
            "user@empresa.es".to_string(),
            "u001@anon.invalid".to_string(),
        );
        fwd_map.insert("12345678Z".to_string(), "99999999A".to_string());
        fwd_map.insert("10.0.0.1".to_string(), "10.99.0.2".to_string());

        let (obf, count) = adapter
            .process_idoc_with_substring_keys(input, &fwd_map, true, &HashSet::new())
            .unwrap();

        assert_eq!(count, 4);
        assert_eq!(
            obf,
            "EDI_DC40\x0110.99.0.1\x01u001@anon.invalid\x02E2EDKA1\x0199999999A\x03EDI_DC40\x0110.99.0.2"
        );
        assert_eq!(
            obf.matches(FIELD_SEP).count(),
            input.matches(FIELD_SEP).count()
        );
        assert_eq!(
            obf.matches(SEGMENT_SEP).count(),
            input.matches(SEGMENT_SEP).count()
        );
        assert_eq!(
            obf.matches(RECORD_SEP).count(),
            input.matches(RECORD_SEP).count()
        );

        let rev_map: HashMap<String, String> = fwd_map.into_iter().map(|(k, v)| (v, k)).collect();
        let (restored, rev_count) = adapter
            .process_idoc_with_substring_keys(&obf, &rev_map, true, &HashSet::new())
            .unwrap();
        assert_eq!(rev_count, 4);
        assert_eq!(restored, input);
    }

    #[test]
    fn test_idoc_format_plain_readable() {
        let input = "EDI_DC40\x01100\x01user@acme.com\x02E2EDK01\x01EUR\x03REC2\x01VAL";
        let plain = IdocAdapter::format_plain_readable(input);
        assert_eq!(
            plain,
            "EDI_DC40 | 100 | user@acme.com\nE2EDK01 | EUR\n\nREC2 | VAL"
        );
    }
}
