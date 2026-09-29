//! Adaptador estructurado para JSON (AST-Driven JSON Obfuscator).
//!
//! Garantiza que SOLO los valores de las cadenas sean ofuscados, manteniendo intactas
//! todas las claves (keys), tipos numéricos, booleanos, nulos y estructura de objetos/arrays.

use crate::engine::replacer::apply_single_pass_replacements;
use crate::models::ObfuscationError;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Default)]
pub struct JsonAdapter;

impl JsonAdapter {
    pub fn new() -> Self {
        Self
    }

    /// Comprueba si el texto de entrada es un JSON sintácticamente válido.
    pub fn is_valid_json(text: &str) -> bool {
        let trimmed = text.trim();
        if (trimmed.starts_with('{') && trimmed.ends_with('}'))
            || (trimmed.starts_with('[') && trimmed.ends_with(']'))
        {
            serde_json::from_str::<Value>(trimmed).is_ok()
        } else {
            false
        }
    }

    /// Procesa y ofusca el contenido de un documento JSON respetando las claves.
    pub fn process_json(
        &self,
        text: &str,
        replacement_map: &HashMap<String, String>,
        strict_word_boundaries: bool,
    ) -> Result<(String, usize), ObfuscationError> {
        let mut parsed: Value = serde_json::from_str(text)
            .map_err(ObfuscationError::JsonError)?;

        let mut total_replacements = 0;
        self.traverse_and_replace(&mut parsed, replacement_map, strict_word_boundaries, &mut total_replacements);

        let output = if text.contains('\n') {
            serde_json::to_string_pretty(&parsed)
                .map_err(ObfuscationError::JsonError)?
        } else {
            serde_json::to_string(&parsed)
                .map_err(ObfuscationError::JsonError)?
        };

        Ok((output, total_replacements))
    }

    fn traverse_and_replace(
        &self,
        val: &mut Value,
        replacement_map: &HashMap<String, String>,
        strict_word_boundaries: bool,
        counter: &mut usize,
    ) {
        match val {
            Value::Object(map) => {
                // Las claves (keys) NUNCA se tocan, solo se recorren los valores
                for (_key, value) in map.iter_mut() {
                    self.traverse_and_replace(value, replacement_map, strict_word_boundaries, counter);
                }
            }
            Value::Array(arr) => {
                for item in arr.iter_mut() {
                    self.traverse_and_replace(item, replacement_map, strict_word_boundaries, counter);
                }
            }
            Value::String(s) => {
                let (replaced, count) = apply_single_pass_replacements(s, replacement_map, strict_word_boundaries);
                if count > 0 {
                    *s = replaced;
                    *counter += count;
                }
            }
            // Tipos primitivos no se ven alterados
            Value::Null | Value::Bool(_) | Value::Number(_) => {}
        }
    }
}

