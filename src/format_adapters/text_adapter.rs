//! Adaptador de texto plano estándar con salvaguardas de tokenización.

use crate::engine::replacer::apply_single_pass_replacements_with_substring_keys;
use crate::models::ObfuscationError;
use std::collections::{HashMap, HashSet};

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
        self.process_text_with_substring_keys(
            text,
            replacement_map,
            strict_word_boundaries,
            &HashSet::new(),
        )
    }

    pub fn process_text_with_substring_keys(
        &self,
        text: &str,
        replacement_map: &HashMap<String, String>,
        strict_word_boundaries: bool,
        substring_keys: &HashSet<String>,
    ) -> Result<(String, usize), ObfuscationError> {
        let (output, count) = apply_single_pass_replacements_with_substring_keys(
            text,
            replacement_map,
            strict_word_boundaries,
            substring_keys,
        );
        Ok((output, count))
    }
}
