//! Motor de reemplazo de subcadenas en una sola pasada (Single-Pass Token Replacer).

use regex::Regex;
use std::collections::HashMap;

/// Estructura de reemplazo de tokens precompilada en una sola pasada.
///
/// Permite compilar la expresión regular una sola vez y reutilizarla eficientemente
/// a lo largo de miles de líneas o bloques de texto sin recompilar.
#[derive(Debug, Clone)]
pub struct CompiledReplacer {
    regex: Option<Regex>,
    replacement_map: HashMap<String, String>,
}

impl CompiledReplacer {
    /// Construye una nueva instancia precompilada ordenando términos por longitud descendente
    /// para evitar el efecto cascada y dar precedencia a patrones más específicos.
    pub fn new(replacement_map: &HashMap<String, String>, strict_word_boundaries: bool) -> Self {
        if replacement_map.is_empty() {
            return Self {
                regex: None,
                replacement_map: HashMap::new(),
            };
        }

        let mut targets: Vec<&String> = replacement_map.keys().collect();
        targets.sort_by_key(|t| std::cmp::Reverse(t.len()));

        let mut pattern_parts = Vec::with_capacity(targets.len());
        for t in &targets {
            let escaped = regex::escape(t);
            if strict_word_boundaries && t.chars().all(|c| c.is_alphanumeric() || c == '_') {
                pattern_parts.push(format!(r"\b{escaped}\b"));
            } else {
                pattern_parts.push(escaped);
            }
        }

        let pattern_str = pattern_parts.join("|");
        let regex = match Regex::new(&pattern_str) {
            Ok(r) => Some(r),
            Err(e) => {
                log::error!("Error construyendo regex para reemplazos: {e}");
                None
            }
        };

        Self {
            regex,
            replacement_map: replacement_map.clone(),
        }
    }

    /// Ejecuta el reemplazo de tokens sobre el texto proporcionado.
    pub fn replace(&self, text: &str) -> (String, usize) {
        if text.is_empty() {
            return (text.to_string(), 0);
        }

        let Some(ref regex) = self.regex else {
            return (text.to_string(), 0);
        };

        let mut replacements_count = 0;
        let result = regex.replace_all(text, |caps: &regex::Captures| {
            let matched = &caps[0];
            if let Some(replacement) = self.replacement_map.get(matched) {
                replacements_count += 1;
                replacement.clone()
            } else {
                matched.to_string()
            }
        });

        (result.into_owned(), replacements_count)
    }

    /// Comprueba si hay patrones compilados activos para sustitución.
    pub fn is_empty(&self) -> bool {
        self.regex.is_none() || self.replacement_map.is_empty()
    }
}

/// Realiza reemplazos en una sola pasada evitando el efecto cascada (A -> B, B -> C).
///
/// Ordena los términos de búsqueda por longitud descendente para que los patrones
/// más largos tengan precedencia sobre subcadenas parciales más cortas.
pub fn apply_single_pass_replacements(
    text: &str,
    replacement_map: &HashMap<String, String>,
    strict_word_boundaries: bool,
) -> (String, usize) {
    let replacer = CompiledReplacer::new(replacement_map, strict_word_boundaries);
    replacer.replace(text)
}
