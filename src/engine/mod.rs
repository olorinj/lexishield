//! Motor principal de ofuscación y desofuscación de LexiShield.

pub mod mapping_manager;
pub mod replacer;

use crate::config::LexiConfig;
use crate::detectors::DetectorRegistry;
use crate::format_adapters::FormatOrchestrator;
use crate::models::{FormatType, Mapping, ObfuscationError, ObfuscationReport};
use crate::validator::SyntaxValidator;
pub use mapping_manager::MappingManager;
use std::fs;
use std::path::Path;
use std::time::Instant;

/// Motor central de LexiShield.
pub struct ObfuscatorEngine {
    pub config: LexiConfig,
    pub manager: MappingManager,
    pub detectors: DetectorRegistry,
    pub format_orchestrator: FormatOrchestrator,
}

impl ObfuscatorEngine {
    pub fn new(config: LexiConfig) -> Self {
        Self {
            config,
            manager: MappingManager::new(),
            detectors: DetectorRegistry::new(),
            format_orchestrator: FormatOrchestrator::new(),
        }
    }

    /// Escanea el contenido y registra automáticamente los mapeos en el gestor.
    pub fn scan_and_register_mappings(
        &mut self,
        content: &str,
    ) -> Result<Vec<Mapping>, ObfuscationError> {
        let detected = self.detectors.scan_text(
            content,
            &self.config.detector_priority_order,
            self.config.min_token_length,
        );
        let newly_added = self.manager.load_mappings(detected)?;
        Ok(newly_added)
    }

    /// Añade un mapeo manual asegurando inyectividad.
    pub fn add_mapping(&mut self, mapping: Mapping) -> Result<(), ObfuscationError> {
        self.manager.add_mapping(mapping)
    }

    /// Aplica la ofuscación completa (Original -> Seudónimo) sobre el texto.
    pub fn obfuscate_text(
        &self,
        text: &str,
        format: FormatType,
    ) -> Result<(String, ObfuscationReport), ObfuscationError> {
        self.transform_text(text, format, false)
    }

    /// Aplica la desofuscación completa (Seudónimo -> Original) sobre el texto.
    pub fn deobfuscate_text(
        &self,
        text: &str,
        format: FormatType,
    ) -> Result<(String, ObfuscationReport), ObfuscationError> {
        self.transform_text(text, format, true)
    }

    fn transform_text(
        &self,
        text: &str,
        format: FormatType,
        reverse: bool,
    ) -> Result<(String, ObfuscationReport), ObfuscationError> {
        let start_time = Instant::now();
        let original_len = text.len();

        let replacement_map = self.manager.get_replacement_map(reverse);

        let (result_text, replacements_count, detected_format) =
            self.format_orchestrator.process_content(
                text,
                format,
                &replacement_map,
                self.config.strict_word_boundaries,
            )?;

        let mut warnings = Vec::new();
        let mut syntax_valid = true;
        let mut schema_intact = true;

        if self.config.validate_syntax_post_process {
            match SyntaxValidator::validate(text, &result_text, detected_format) {
                Ok(vreport) => {
                    syntax_valid = vreport.syntax_valid;
                    schema_intact = vreport.schema_keys_intact;
                    warnings.extend(vreport.issues);
                }
                Err(e) => {
                    warnings.push(format!("Aviso de validación: {}", e));
                    schema_intact = false;
                }
            }
        }

        let elapsed_ms = start_time.elapsed().as_secs_f64() * 1000.0;
        let report = ObfuscationReport {
            format_detected: detected_format,
            original_length: original_len,
            result_length: result_text.len(),
            replacements_applied: replacements_count,
            elapsed_ms,
            schema_intact,
            syntax_valid,
            warnings,
        };

        log::info!(
            "Transformación completada en {:.2}ms [Formato: {:?}, Reemplazos: {}]",
            elapsed_ms,
            detected_format,
            replacements_count
        );

        Ok((result_text, report))
    }

    /// Procesa un archivo en disco de forma segura.
    pub fn process_file(
        &self,
        input_path: &Path,
        output_path: &Path,
        format: FormatType,
        reverse: bool,
    ) -> Result<ObfuscationReport, ObfuscationError> {
        let content = fs::read_to_string(input_path)?;
        let (transformed, report) = self.transform_text(&content, format, reverse)?;
        fs::write(output_path, transformed)?;
        Ok(report)
    }
}
