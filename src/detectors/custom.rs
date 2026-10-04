//! Detector dinámico para reglas personalizadas definidas por el usuario en config.toml.

use crate::models::DetectorType;
use rand::Rng;
use regex::Regex;
use serde::{Deserialize, Serialize};

/// Estrategia de generación de seudónimos para reglas personalizadas.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CustomStrategy {
    /// Sustituye únicamente los dígitos por dígitos aleatorios preservando la longitud.
    #[default]
    RandomDigits,
    /// Sustituye caracteres hexadecimales por otros aleatorios preservando capitalización.
    RandomHex,
    /// Sustituye caracteres alfanuméricos por otros aleatorios.
    RandomAlphanumeric,
    /// Genera un prefijo seguido de un identificador numérico aleatorio (ej. EMP_123456).
    PrefixSeq,
    /// Enmascara el valor sensible con asteriscos o un marcador fijo.
    Mask,
}

/// Definición de una regla personalizada en el archivo de configuración TOML.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CustomRule {
    /// Nombre descriptivo de la regla (ej. "Identificador de Empleado").
    pub name: String,
    /// Expresión regular para detectar el patrón.
    pub pattern: String,
    /// Prefijo opcional a conservar o añadir.
    #[serde(default)]
    pub prefix: Option<String>,
    /// Estrategia de generación sintética.
    #[serde(default)]
    pub strategy: CustomStrategy,
    /// Si es true, las coincidencias se marcarán como omitidas (=) sin ofuscar.
    #[serde(default)]
    pub omitted: bool,
}

/// Regla personalizada compilada y lista para ejecutar escaneos y generación.
#[derive(Debug, Clone)]
pub struct CompiledCustomRule {
    pub name: String,
    pub regex: Regex,
    pub prefix: Option<String>,
    pub strategy: CustomStrategy,
    pub omitted: bool,
}

impl CompiledCustomRule {
    pub fn try_from_rule(rule: &CustomRule) -> Result<Self, regex::Error> {
        let regex = Regex::new(&rule.pattern)?;
        Ok(Self {
            name: rule.name.clone(),
            regex,
            prefix: rule.prefix.clone(),
            strategy: rule.strategy.clone(),
            omitted: rule.omitted,
        })
    }

    pub fn detector_type(&self) -> DetectorType {
        DetectorType::Custom(self.name.clone())
    }

    pub fn find_matches<'a>(&self, text: &'a str) -> Vec<(usize, usize, &'a str)> {
        self.regex
            .find_iter(text)
            .map(|m| (m.start(), m.end(), m.as_str()))
            .collect()
    }

    pub fn generate_pseudonym(&self, original: &str) -> String {
        if self.omitted {
            return "=".to_string();
        }

        let mut rng = rand::thread_rng();

        match self.strategy {
            CustomStrategy::RandomDigits => {
                let mut result = String::with_capacity(original.len());
                for ch in original.chars() {
                    if ch.is_ascii_digit() {
                        let d = rng.gen_range(0..=9);
                        result.push(char::from_digit(d, 10).unwrap());
                    } else {
                        result.push(ch);
                    }
                }
                result
            }
            CustomStrategy::RandomHex => {
                const HEX_CHARS: &[u8] = b"0123456789abcdef";
                const HEX_CHARS_UPPER: &[u8] = b"0123456789ABCDEF";
                let mut result = String::with_capacity(original.len());
                for ch in original.chars() {
                    if ch.is_ascii_hexdigit() {
                        let idx = rng.gen_range(0..16);
                        if ch.is_ascii_uppercase() {
                            result.push(HEX_CHARS_UPPER[idx] as char);
                        } else {
                            result.push(HEX_CHARS[idx] as char);
                        }
                    } else {
                        result.push(ch);
                    }
                }
                result
            }
            CustomStrategy::RandomAlphanumeric => {
                const ALNUM: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
                let mut result = String::with_capacity(original.len());
                for ch in original.chars() {
                    if ch.is_alphanumeric() {
                        let idx = rng.gen_range(0..ALNUM.len());
                        let chosen = ALNUM[idx] as char;
                        if ch.is_lowercase() {
                            result.push(chosen.to_ascii_lowercase());
                        } else {
                            result.push(chosen);
                        }
                    } else {
                        result.push(ch);
                    }
                }
                result
            }
            CustomStrategy::PrefixSeq => {
                let p = self.prefix.as_deref().unwrap_or("CUSTOM_");
                let num: u32 = rng.gen_range(100_000..999_999);
                format!("{p}{num:06}")
            }
            CustomStrategy::Mask => {
                if let Some(p) = &self.prefix {
                    format!("{p}***")
                } else {
                    "***REDACTED***".to_string()
                }
            }
        }
    }
}
