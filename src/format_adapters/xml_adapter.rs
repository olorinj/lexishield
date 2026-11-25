//! Adaptador de estructura para XML y HTML (DOM/Event-Driven XML Obfuscator).
//!
//! Garantiza que SOLO los nodos de texto y secciones CDATA (y valores de atributos)
//! sean ofuscados, preservando intactos los nombres de etiquetas (<tags>), nombres de atributos,
//! entidades y declaraciones XML.

use crate::engine::replacer::apply_single_pass_replacements_with_substring_keys;
use crate::models::ObfuscationError;
use quick_xml::events::{BytesCData, BytesStart, BytesText, Event};
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
                Ok(Event::Start(e)) => {
                    let (maybe_new, count) = Self::process_tag_attributes(
                        &e,
                        replacement_map,
                        strict_word_boundaries,
                        substring_keys,
                    )?;
                    total_replacements += count;
                    let ev = match maybe_new {
                        Some(rebuilt) => Event::Start(rebuilt),
                        None => Event::Start(e),
                    };
                    writer
                        .write_event(ev)
                        .map_err(|err| ObfuscationError::XmlError(err.to_string()))?;
                }
                Ok(Event::Empty(e)) => {
                    let (maybe_new, count) = Self::process_tag_attributes(
                        &e,
                        replacement_map,
                        strict_word_boundaries,
                        substring_keys,
                    )?;
                    total_replacements += count;
                    let ev = match maybe_new {
                        Some(rebuilt) => Event::Empty(rebuilt),
                        None => Event::Empty(e),
                    };
                    writer
                        .write_event(ev)
                        .map_err(|err| ObfuscationError::XmlError(err.to_string()))?;
                }
                Ok(Event::Comment(c)) => {
                    if let Ok(raw_str) = std::str::from_utf8(&c) {
                        let (replaced_text, count) =
                            apply_single_pass_replacements_with_substring_keys(
                                raw_str,
                                replacement_map,
                                strict_word_boundaries,
                                substring_keys,
                            );
                        if count > 0 && !replaced_text.contains("--") {
                            total_replacements += count;
                            writer
                                .write_event(Event::Comment(BytesText::from_escaped(
                                    &replaced_text,
                                )))
                                .map_err(|err| ObfuscationError::XmlError(err.to_string()))?;
                        } else {
                            writer
                                .write_event(Event::Comment(c))
                                .map_err(|err| ObfuscationError::XmlError(err.to_string()))?;
                        }
                    } else {
                        writer
                            .write_event(Event::Comment(c))
                            .map_err(|err| ObfuscationError::XmlError(err.to_string()))?;
                    }
                }
                // Las etiquetas de fin y declaraciones XML se escriben intactas
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

    fn process_tag_attributes(
        e: &BytesStart<'_>,
        replacement_map: &HashMap<String, String>,
        strict_word_boundaries: bool,
        substring_keys: &HashSet<String>,
    ) -> Result<(Option<BytesStart<'static>>, usize), ObfuscationError> {
        let raw_bytes = e.as_ref();
        let raw_str = std::str::from_utf8(raw_bytes)
            .map_err(|err| ObfuscationError::XmlError(err.to_string()))?;
        let name_len = e.name().as_ref().len();
        if raw_str.len() <= name_len {
            return Ok((None, 0));
        }

        // Escanear pares key="val" o key='val' preservando exactamente espacios, saltos de línea y estilo de cierre (" />" vs "/>")
        let mut rebuilt = String::with_capacity(raw_str.len());
        rebuilt.push_str(&raw_str[..name_len]);

        let bytes = raw_str.as_bytes();
        let mut idx = name_len;
        let mut tag_replacements = 0usize;

        while idx < bytes.len() {
            // Copiar espacios o separadores previos al nombre del atributo
            let ws_start = idx;
            while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
                idx += 1;
            }
            rebuilt.push_str(&raw_str[ws_start..idx]);
            if idx >= bytes.len() {
                break;
            }

            // Leer nombre del atributo hasta '=' o espacio
            let key_start = idx;
            while idx < bytes.len() && bytes[idx] != b'=' && !bytes[idx].is_ascii_whitespace() {
                idx += 1;
            }
            let key_str = &raw_str[key_start..idx];
            rebuilt.push_str(key_str);

            // Copiar posibles espacios antes de '='
            let eq_ws_start = idx;
            while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
                idx += 1;
            }
            rebuilt.push_str(&raw_str[eq_ws_start..idx]);

            if idx >= bytes.len() || bytes[idx] != b'=' {
                continue;
            }
            // Copiar '=' y posibles espacios posteriores
            rebuilt.push('=');
            idx += 1;
            let post_eq_ws = idx;
            while idx < bytes.len() && bytes[idx].is_ascii_whitespace() {
                idx += 1;
            }
            rebuilt.push_str(&raw_str[post_eq_ws..idx]);

            if idx >= bytes.len() || (bytes[idx] != b'"' && bytes[idx] != b'\'') {
                continue;
            }
            let quote = bytes[idx];
            rebuilt.push(quote as char);
            idx += 1;

            let val_start = idx;
            while idx < bytes.len() && bytes[idx] != quote {
                idx += 1;
            }
            let raw_val = &raw_str[val_start..idx];

            let is_ns =
                key_str == "xmlns" || key_str.starts_with("xmlns:") || key_str.starts_with("xsi:");

            if is_ns {
                rebuilt.push_str(raw_val);
            } else {
                let unescaped = quick_xml::escape::unescape(raw_val)
                    .map_err(|err| ObfuscationError::XmlError(err.to_string()))?;
                let (replaced_val, count) = apply_single_pass_replacements_with_substring_keys(
                    &unescaped,
                    replacement_map,
                    strict_word_boundaries,
                    substring_keys,
                );
                if count > 0 {
                    tag_replacements += count;
                    let esc = quick_xml::escape::escape(&replaced_val);
                    if quote == b'"' {
                        rebuilt.push_str(&esc.replace('"', "&quot;"));
                    } else {
                        rebuilt.push_str(&esc.replace('\'', "&apos;"));
                    }
                } else {
                    rebuilt.push_str(raw_val);
                }
            }

            if idx < bytes.len() && bytes[idx] == quote {
                rebuilt.push(quote as char);
                idx += 1;
            }
        }

        if tag_replacements == 0 {
            return Ok((None, 0));
        }

        Ok((
            Some(BytesStart::from_content(rebuilt, name_len)),
            tag_replacements,
        ))
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
