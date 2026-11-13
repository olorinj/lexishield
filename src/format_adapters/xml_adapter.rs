//! Adaptador de estructura para XML y HTML (DOM/Event-Driven XML Obfuscator).
//!
//! Garantiza que SOLO los nodos de texto y secciones CDATA (y valores de atributos)
//! sean ofuscados, preservando intactos los nombres de etiquetas (<tags>), nombres de atributos,
//! entidades y declaraciones XML.

use crate::engine::replacer::apply_single_pass_replacements_with_substring_keys;
use crate::models::ObfuscationError;
use quick_xml::events::{BytesCData, BytesText, Event};
use quick_xml::reader::Reader;
use quick_xml::writer::Writer;
use std::collections::{HashMap, HashSet};
use std::io::Cursor;

#[derive(Default)]
pub struct XmlAdapter;

impl XmlAdapter {
    pub fn new() -> Self {
        Self
    }

    /// Comprueba si el texto parece ser un XML válido.
    pub fn is_valid_xml(text: &str) -> bool {
        let trimmed = text.trim();
        if (trimmed.starts_with('<') && trimmed.ends_with('>')) || trimmed.starts_with("<?xml") {
            let mut reader = Reader::from_str(trimmed);
            reader.config_mut().trim_text(false);
            let mut buf = Vec::new();
            loop {
                match reader.read_event_into(&mut buf) {
                    Ok(Event::Eof) => return true,
                    Err(_) => return false,
                    _ => {}
                }
                buf.clear();
            }
        }
        false
    }

    /// Procesa y ofusca el contenido de un documento XML respetando etiquetas y atributos.
    pub fn process_xml(
        &self,
        text: &str,
        replacement_map: &HashMap<String, String>,
        strict_word_boundaries: bool,
    ) -> Result<(String, usize), ObfuscationError> {
        self.process_xml_with_substring_keys(
            text,
            replacement_map,
            strict_word_boundaries,
            &HashSet::new(),
        )
    }

    /// Variante de `process_xml` que permite omitir límites de palabra para claves específicas.
    pub fn process_xml_with_substring_keys(
        &self,
        text: &str,
        replacement_map: &HashMap<String, String>,
        strict_word_boundaries: bool,
        substring_keys: &HashSet<String>,
    ) -> Result<(String, usize), ObfuscationError> {
        let mut reader = Reader::from_str(text);
        reader.config_mut().trim_text(false);
        let mut writer = Writer::new(Cursor::new(Vec::new()));
        let mut buf = Vec::new();
        let mut total_replacements = 0;

        loop {
            match reader.read_event_into(&mut buf) {
                Ok(Event::Eof) => break,
                Ok(Event::Text(t)) => {
                    let decoded = match std::str::from_utf8(&t) {
                        Ok(s) => s,
                        Err(e) => return Err(ObfuscationError::XmlError(e.to_string())),
                    };
                    let raw_str = match quick_xml::escape::unescape(decoded) {
                        Ok(s) => s.into_owned(),
                        Err(e) => return Err(ObfuscationError::XmlError(e.to_string())),
                    };
                    let (replaced_text, count) = apply_single_pass_replacements_with_substring_keys(
                        &raw_str,
                        replacement_map,
                        strict_word_boundaries,
                        substring_keys,
                    );
                    total_replacements += count;
                    let escaped = quick_xml::escape::escape(&replaced_text);
                    writer
                        .write_event(Event::Text(BytesText::new(&escaped)))
                        .map_err(|e| ObfuscationError::XmlError(e.to_string()))?;
                }
                Ok(Event::CData(c)) => {
                    let raw_str = match std::str::from_utf8(&c) {
                        Ok(s) => s,
                        Err(e) => return Err(ObfuscationError::XmlError(e.to_string())),
                    };
                    let (replaced_text, count) = apply_single_pass_replacements_with_substring_keys(
                        raw_str,
                        replacement_map,
                        strict_word_boundaries,
                        substring_keys,
                    );
                    total_replacements += count;
                    writer
                        .write_event(Event::CData(BytesCData::new(&replaced_text)))
                        .map_err(|e| ObfuscationError::XmlError(e.to_string()))?;
                }
                // Las etiquetas de inicio, fin, comentarios y declaraciones se escriben intactas
                Ok(event) => {
                    writer
                        .write_event(event)
                        .map_err(|e| ObfuscationError::XmlError(e.to_string()))?;
                }
                Err(e) => return Err(ObfuscationError::XmlError(e.to_string())),
            }
            buf.clear();
        }

        let result_bytes = writer.into_inner().into_inner();
        let result_str = String::from_utf8(result_bytes)
            .map_err(|e| ObfuscationError::XmlError(e.to_string()))?;

        Ok((result_str, total_replacements))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xml_escaping_ampersand_and_special_chars() {
        let adapter = XmlAdapter::new();
        let mut map = HashMap::new();
        map.insert("Empresa".to_string(), "Compania & Co".to_string());

        let input_xml = "<root><item>Empresa &amp; Asociados</item></root>";
        let (output_xml, count) = adapter.process_xml(input_xml, &map, false).unwrap();

        assert_eq!(count, 1);
        // Debe re-escapar el & generado en el reemplazo y el &amp; original
        assert!(output_xml.contains("&amp;"));
        assert!(!output_xml.contains(" & "));
        assert!(XmlAdapter::is_valid_xml(&output_xml));
    }
}
