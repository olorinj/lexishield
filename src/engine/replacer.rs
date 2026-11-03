//! Motor de reemplazo de subcadenas en una sola pasada de alto rendimiento (Aho-Corasick Token Replacer).

use aho_corasick::{AhoCorasick, MatchKind};
use std::collections::HashMap;

/// Estructura de reemplazo de tokens precompilada con el autómata Aho-Corasick.
///
/// Ofrece complejidad de búsqueda y reemplazo estrictamente lineal O(N) sin retroceso
/// ni compilación de expresiones regulares pesadas.
#[derive(Debug, Clone)]
pub struct CompiledReplacer {
    automa: Option<AhoCorasick>,
    replacements: Vec<String>,
    strict_word_boundaries: bool,
    pattern_lengths: Vec<usize>,
}

impl CompiledReplacer {
    /// Construye una nueva instancia precompilada ordenando términos por longitud descendente
    /// y configurando coincidencia LeftmostLongest para dar precedencia a patrones específicos.
    pub fn new(replacement_map: &HashMap<String, String>, strict_word_boundaries: bool) -> Self {
        if replacement_map.is_empty() {
            return Self {
                automa: None,
                replacements: Vec::new(),
                strict_word_boundaries,
                pattern_lengths: Vec::new(),
            };
        }

        // Ordenar términos por longitud descendente para que LeftmostLongest
        // y la prioridad de patrones mantengan precedencia idéntica a la especificación.
        let mut entries: Vec<(&String, &String)> = replacement_map.iter().collect();
        entries.sort_by_key(|(orig, _)| std::cmp::Reverse(orig.len()));

        let mut patterns = Vec::with_capacity(entries.len());
        let mut replacements = Vec::with_capacity(entries.len());
        let mut pattern_lengths = Vec::with_capacity(entries.len());

        for (orig, repl) in entries {
            patterns.push(orig.as_str());
            replacements.push(repl.clone());
            pattern_lengths.push(orig.len());
        }

        let automa = AhoCorasick::builder()
            .match_kind(MatchKind::LeftmostLongest)
            .build(&patterns)
            .ok();

        Self {
            automa,
            replacements,
            strict_word_boundaries,
            pattern_lengths,
        }
    }

    /// Comprueba si los límites del término son delimitadores de palabra (\b)
    #[inline]
    fn is_word_boundary(bytes: &[u8], start: usize, end: usize) -> bool {
        let is_prev_word_char = if start > 0 {
            let prev = bytes[start - 1];
            prev.is_ascii_alphanumeric() || prev == b'_'
        } else {
            false
        };

        let is_first_word_char = if start < bytes.len() {
            let cur = bytes[start];
            cur.is_ascii_alphanumeric() || cur == b'_'
        } else {
            false
        };

        if is_prev_word_char && is_first_word_char {
            return false;
        }

        let is_last_word_char = if end > 0 && end <= bytes.len() {
            let cur = bytes[end - 1];
            cur.is_ascii_alphanumeric() || cur == b'_'
        } else {
            false
        };

        let is_next_word_char = if end < bytes.len() {
            let next = bytes[end];
            next.is_ascii_alphanumeric() || next == b'_'
        } else {
            false
        };

        !(is_last_word_char && is_next_word_char)
    }

    /// Ejecuta el reemplazo de tokens sobre el texto proporcionado sin clonaciones intermedias redundantes.
    pub fn replace(&self, text: &str) -> (String, usize) {
        if text.is_empty() {
            return (String::new(), 0);
        }

        let Some(ref automa) = self.automa else {
            return (text.to_string(), 0);
        };

        let text_bytes = text.as_bytes();
        let mut result = String::with_capacity(text.len());
        let mut replacements_count = 0;
        let mut last_end = 0;

        for m in automa.find_iter(text) {
            let start = m.start();
            let end = m.end();

            // Si se requiere word boundaries estrictos, comprobar los límites del match
            if self.strict_word_boundaries {
                let pat_len = self
                    .pattern_lengths
                    .get(m.pattern().as_usize())
                    .copied()
                    .unwrap_or(0);
                if pat_len > 0 && !Self::is_word_boundary(text_bytes, start, end) {
                    continue;
                }
            }

            // Añadir texto no coincidente previo
            if start > last_end {
                result.push_str(&text[last_end..start]);
            }

            // Añadir el reemplazo directamente
            if let Some(replacement) = self.replacements.get(m.pattern().as_usize()) {
                result.push_str(replacement);
                replacements_count += 1;
            } else {
                result.push_str(&text[start..end]);
            }

            last_end = end;
        }

        if last_end < text.len() {
            result.push_str(&text[last_end..]);
        }

        (result, replacements_count)
    }

    /// Comprueba si hay patrones compilados activos para sustitución.
    pub fn is_empty(&self) -> bool {
        self.automa.is_none() || self.replacements.is_empty()
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
