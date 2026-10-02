//! Módulo de adaptadores de formato de LexiShield (Mejora 1: Format Adapters).

pub mod json_adapter;
pub mod log_adapter;
pub mod text_adapter;
pub mod xml_adapter;

use crate::models::{FormatType, ObfuscationError};
use json_adapter::JsonAdapter;
use log_adapter::LogAdapter;
use std::collections::HashMap;
use text_adapter::TextAdapter;
use xml_adapter::XmlAdapter;

/// Orquestador de formatos con detección inteligente.
pub struct FormatOrchestrator {
    pub json: JsonAdapter,
    pub xml: XmlAdapter,
    pub log: LogAdapter,
    pub text: TextAdapter,
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
        let actual_format = match format {
            FormatType::Auto => self.detect_format(content),
            other => other,
        };

        log::debug!(
            "Procesando contenido con adaptador de formato: {:?}",
            actual_format
        );

        let (result_text, count) = match actual_format {
            FormatType::Json => {
                self.json
                    .process_json(content, replacement_map, strict_word_boundaries)?
            }
            FormatType::Xml => {
                self.xml
                    .process_xml(content, replacement_map, strict_word_boundaries)?
            }
            FormatType::Log => {
                self.log
                    .process_log(content, replacement_map, strict_word_boundaries)?
            }
            FormatType::Plaintext | FormatType::Auto => {
                self.text
                    .process_text(content, replacement_map, strict_word_boundaries)?
            }
        };

        Ok((result_text, count, actual_format))
    }
}
