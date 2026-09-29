//! Adaptador de texto plano estándar con salvaguardas de tokenización.

use crate::engine::replacer::apply_single_pass_replacements;
use crate::models::ObfuscationError;
use std::collections::HashMap;

#[derive(Default)]
pub struct TextAdapter;

impl TextAdapter {
    pub fn new() -> Self {
        Self
    }

    pub fn process_text(
        &self,
        text: &str,
        replacement_map: &HashMap<String, String>,
        strict_word_boundaries: bool,
    ) -> Result<(String, usize), ObfuscationError> {
        let (output, count) =
            apply_single_pass_replacements(text, replacement_map, strict_word_boundaries);
        Ok((output, count))
    }
}

