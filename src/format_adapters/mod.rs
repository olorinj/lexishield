//! Módulo de adaptadores de formato de LexiShield (Mejora 1: Format Adapters).

pub mod json_adapter;
pub mod log_adapter;
pub mod office_adapter;
pub mod text_adapter;
pub mod xml_adapter;

use crate::models::{FormatType, ObfuscationError};
use json_adapter::JsonAdapter;
use log_adapter::LogAdapter;
use office_adapter::OfficeAdapter;
use std::collections::HashMap;
use text_adapter::TextAdapter;
use xml_adapter::XmlAdapter;

/// Orquestador de formatos con detección inteligente.
pub struct FormatOrchestrator {
    pub json: JsonAdapter,
    pub xml: XmlAdapter,
    pub log: LogAdapter,
    pub text: TextAdapter,
    pub office: OfficeAdapter,
}

impl Default for FormatOrchestrator {
    fn default() -> Self {
        Self::new()
    }
}

impl FormatOrchestrator {
    pub fn new() -> Self {
        Self {
            json: JsonAdapter::new(),
            xml: XmlAdapter::new(),
            log: LogAdapter::new(),
            text: TextAdapter::new(),
            office: OfficeAdapter::new(),
        }
    }

    /// Detecta automáticamente el tipo de formato del documento.
    pub fn detect_format(&self, content: &str) -> FormatType {
        if JsonAdapter::is_valid_json(content) {
            FormatType::Json
        } else if XmlAdapter::is_valid_xml(content) {
            FormatType::Xml
        } else if LogAdapter::is_log_format(content) {
            FormatType::Log
        } else {
            FormatType::Plaintext
        }
    }

    /// Procesa el contenido delegando al adaptador de formato correspondiente.
    pub fn process_content(
        &self,
        content: &str,
        format: FormatType,
        replacement_map: &HashMap<String, String>,
        strict_word_boundaries: bool,
    ) -> Result<(String, usize, FormatType), ObfuscationError> {
        self.process_content_with_substring_keys(
            content,
            format,
            replacement_map,
            strict_word_boundaries,
            &std::collections::HashSet::new(),
        )
    }

    /// Variante de `process_content` que permite omitir límites de palabra para claves específicas.
    pub fn process_content_with_substring_keys(
        &self,
        content: &str,
        format: FormatType,
        replacement_map: &HashMap<String, String>,
        strict_word_boundaries: bool,
        substring_keys: &std::collections::HashSet<String>,
    ) -> Result<(String, usize, FormatType), ObfuscationError> {
        let actual_format = match format {
            FormatType::Auto => self.detect_format(content),
            other => other,
        };

        log::debug!("Procesando contenido con adaptador de formato: {actual_format:?}");

        let (result_text, count) = match actual_format {
            FormatType::Json => self.json.process_json_with_substring_keys(
                content,
                replacement_map,
                strict_word_boundaries,
                substring_keys,
            )?,
            FormatType::Xml => self.xml.process_xml_with_substring_keys(
                content,
                replacement_map,
                strict_word_boundaries,
                substring_keys,
            )?,
            FormatType::Log => self.log.process_log_with_substring_keys(
                content,
                replacement_map,
                strict_word_boundaries,
                substring_keys,
            )?,
            FormatType::Plaintext | FormatType::Auto => {
                self.text.process_text_with_substring_keys(
                    content,
                    replacement_map,
                    strict_word_boundaries,
                    substring_keys,
                )?
            }
        };

        Ok((result_text, count, actual_format))
    }
}
