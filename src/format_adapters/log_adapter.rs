//! Adaptador de logs estructurados y semiestructurados (KVP, Logfmt, Syslog, CSV).
//!
//! Respeta cabeceras, nombres de campo (pares clave=valor tipo `src_ip=...`) y
//! marcas de tiempo de syslog, anonimizando exclusivamente los valores sensibles.

use crate::engine::replacer::apply_single_pass_replacements;
use crate::models::ObfuscationError;
use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;

static KVP_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"([a-zA-Z0-9_\-\.]+)=("?[^"\s]+"?[^"\s]*)"#).expect("Regex KVP inválida")
});

#[derive(Default)]
pub struct LogAdapter;

impl LogAdapter {
    pub fn new() -> Self {
        Self
    }

    /// Comprueba si el texto tiene formato típico de líneas de log estructuradas o KVP.
    pub fn is_log_format(text: &str) -> bool {
        let lines: Vec<&str> = text.lines().take(10).collect();
        if lines.is_empty() {
            return false;
        }

        let kvp_count = lines
            .iter()
            .filter(|l| KVP_REGEX.is_match(l) || l.contains(" - - [") || l.contains("]: "))
            .count();

        kvp_count > 0
    }

    /// Procesa el log línea a línea respetando nombres de campos y delimitadores.
    pub fn process_log(
        &self,
        text: &str,
        replacement_map: &HashMap<String, String>,
        strict_word_boundaries: bool,
    ) -> Result<(String, usize), ObfuscationError> {
        let mut total_replacements = 0;
        let mut output_lines = Vec::new();

        for line in text.lines() {
            if line.trim().is_empty() {
                output_lines.push(String::new());
                continue;
            }

            // Si la línea contiene pares clave=valor, procesar respetando la clave
            if KVP_REGEX.is_match(line) {
                let replaced_line = KVP_REGEX.replace_all(line, |caps: &regex::Captures| {
                    let key = &caps[1];
                    let val = &caps[2];

                    let is_quoted = val.starts_with('"') && val.ends_with('"');
                    let raw_val = if is_quoted && val.len() >= 2 {
                        &val[1..val.len() - 1]
                    } else {
                        val
                    };

                    let (new_val, count) = apply_single_pass_replacements(
                        raw_val,
                        replacement_map,
                        strict_word_boundaries,
                    );
                    total_replacements += count;

                    if is_quoted {
                        format!("{key}=\"{new_val}\"")
                    } else {
                        format!("{key}={new_val}")
                    }
                });
                output_lines.push(replaced_line.into_owned());
            } else {
                // Línea de log estándar: aplicar reemplazo con guardia de palabras
                let (replaced_line, count) =
                    apply_single_pass_replacements(line, replacement_map, strict_word_boundaries);
                total_replacements += count;
                output_lines.push(replaced_line);
            }
        }

        let final_output = output_lines.join("\n");
        Ok((final_output, total_replacements))
    }
}
