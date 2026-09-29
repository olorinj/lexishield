//! Motor de reemplazo de subcadenas en una sola pasada (Single-Pass Token Replacer).

use std::collections::HashMap;
use regex::Regex;

/// Realiza reemplazos en una sola pasada evitando el efecto cascada (A -> B, B -> C).
///
/// Ordena los términos de búsqueda por longitud descendente para que los patrones
/// más largos tengan precedencia sobre subcadenas parciales más cortas.
pub fn apply_single_pass_replacements(
    text: &str,
    replacement_map: &HashMap<String, String>,
    strict_word_boundaries: bool,
) -> (String, usize) {
    if replacement_map.is_empty() || text.is_empty() {
        return (text.to_string(), 0);
    }

    // Ordenar por longitud descendente
    let mut targets: Vec<&String> = replacement_map.keys().collect();
    targets.sort_by_key(|t| std::cmp::Reverse(t.len()));

    let mut pattern_parts = Vec::with_capacity(targets.len());
    for t in &targets {
        let escaped = regex::escape(t);
        if strict_word_boundaries && t.chars().all(|c| c.is_alphanumeric() || c == '_') {
            pattern_parts.push(format!(r"\b{}\b", escaped));
        } else {
            pattern_parts.push(escaped);
        }
    }

    let pattern_str = pattern_parts.join("|");
    let regex = match Regex::new(&pattern_str) {
        Ok(r) => r,
        Err(e) => {
            log::error!("Error construyendo regex para reemplazos: {}", e);
            return (text.to_string(), 0);
        }
    };

    let mut replacements_count = 0;
    let result = regex.replace_all(text, |caps: &regex::Captures| {
        let matched = &caps[0];
        if let Some(replacement) = replacement_map.get(matched) {
            replacements_count += 1;
            replacement.clone()
        } else {
            matched.to_string()
        }
    });

    (result.into_owned(), replacements_count)
}

