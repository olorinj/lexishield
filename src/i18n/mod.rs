//! Soporte de internacionalización (i18n) para LexiShield.
//!
//! Este módulo provee tipos y diccionarios para salida multilingüe en CLI y backend,
//! con soporte de compilación condicional para excluir idiomas innecesarios y minimizar
//! el tamaño en disco del ejecutable compilado.

use std::env;
use std::fmt;
use std::str::FromStr;

#[cfg(feature = "lang-de")]
pub mod de;
#[cfg(feature = "lang-en")]
pub mod en;
#[cfg(feature = "lang-es")]
pub mod es;
#[cfg(feature = "lang-fr")]
pub mod fr;
#[cfg(feature = "lang-it")]
pub mod it;
#[cfg(feature = "lang-pt")]
pub mod pt;
#[cfg(feature = "lang-zh")]
pub mod zh;

/// Códigos de idiomas soportados por LexiShield.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Language {
    #[default]
    En,
    Es,
    Fr,
    De,
    Pt,
    It,
    Zh,
}

impl Language {
    /// Devuelve el código ISO 639-1 en minúsculas.
    pub const fn code(&self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Es => "es",
            Self::Fr => "fr",
            Self::De => "de",
            Self::Pt => "pt",
            Self::It => "it",
            Self::Zh => "zh",
        }
    }

    /// Devuelve el nombre del idioma en su propia lengua (endónimo).
    pub const fn name(&self) -> &'static str {
        match self {
            Self::En => "English",
            Self::Es => "Español",
            Self::Fr => "Français",
            Self::De => "Deutsch",
            Self::Pt => "Português",
            Self::It => "Italiano",
            Self::Zh => "简体中文",
        }
    }

    /// Resuelve el idioma efectivo consultando flag CLI, configuración TOML, variables de entorno
    /// (LC_ALL, LC_MESSAGES, LANG) y fallback al valor predeterminado (inglés).
    pub fn resolve(cli_lang: Option<&str>, config_lang: Option<&str>) -> Self {
        if let Some(s) = cli_lang.and_then(|s| Self::from_str(s).ok()) {
            return s;
        }

        if let Some(s) = config_lang.and_then(|s| Self::from_str(s).ok()) {
            return s;
        }

        // Detección automática por variables de entorno Unix / Windows
        for var in &["LC_ALL", "LC_MESSAGES", "LANG"] {
            if let Ok(val) = env::var(var) {
                let lower = val.to_lowercase();
                if lower.starts_with("es") {
                    return Self::Es;
                }
                if lower.starts_with("fr") {
                    return Self::Fr;
                }
                if lower.starts_with("de") {
                    return Self::De;
                }
                if lower.starts_with("pt") {
                    return Self::Pt;
                }
                if lower.starts_with("it") {
                    return Self::It;
                }
                if lower.starts_with("zh") {
                    return Self::Zh;
                }
                if lower.starts_with("en") {
                    return Self::En;
                }
            }
        }

        Self::default()
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.code())
    }
}

impl FromStr for Language {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let clean = s.trim().to_lowercase();
        if clean.starts_with("es") {
            Ok(Self::Es)
        } else if clean.starts_with("en") {
            Ok(Self::En)
        } else if clean.starts_with("fr") {
            Ok(Self::Fr)
        } else if clean.starts_with("de") {
            Ok(Self::De)
        } else if clean.starts_with("pt") {
            Ok(Self::Pt)
        } else if clean.starts_with("it") {
            Ok(Self::It)
        } else if clean.starts_with("zh") {
            Ok(Self::Zh)
        } else {
            Err(format!(
                "Unsupported language: '{s}'. Supported: 'en', 'es', 'fr', 'de', 'pt', 'it', 'zh'"
            ))
        }
    }
}

/// Identificadores tipados de mensajes para el CLI y backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MsgKey {
    CliAbout,
    VerboseDesc,
    DebugDesc,
    ConfigPathDesc,
    RulesPathDesc,
    LangDesc,
    EngineInitialized,
    ScanningStarted,
    FilesProcessed,
    EntitiesFound,
    SavedAndEncryptedMappings,
    DeobfuscationSuccess,
    CorruptedFileError,
    ClipboardWatching,
    ClipboardReplaced,
    ConfigResetSuccess,
}

/// Obtiene el texto traducido según el idioma seleccionado.
/// Si una feature de idioma está deshabilitada en tiempo de compilación, recurre al idioma disponible.
pub fn tr(lang: Language, key: MsgKey) -> &'static str {
    match lang {
        #[cfg(feature = "lang-es")]
        Language::Es => es::translate(key),
        #[cfg(not(feature = "lang-es"))]
        Language::Es => tr(Language::En, key),

        #[cfg(feature = "lang-en")]
        Language::En => en::translate(key),
        #[cfg(not(feature = "lang-en"))]
        Language::En => tr(Language::Es, key),

        #[cfg(feature = "lang-fr")]
        Language::Fr => fr::translate(key),
        #[cfg(not(feature = "lang-fr"))]
        Language::Fr => tr(Language::En, key),

        #[cfg(feature = "lang-de")]
        Language::De => de::translate(key),
        #[cfg(not(feature = "lang-de"))]
        Language::De => tr(Language::En, key),

        #[cfg(feature = "lang-pt")]
        Language::Pt => pt::translate(key),
        #[cfg(not(feature = "lang-pt"))]
        Language::Pt => tr(Language::En, key),

        #[cfg(feature = "lang-it")]
        Language::It => it::translate(key),
        #[cfg(not(feature = "lang-it"))]
        Language::It => tr(Language::En, key),

        #[cfg(feature = "lang-zh")]
        Language::Zh => zh::translate(key),
        #[cfg(not(feature = "lang-zh"))]
        Language::Zh => tr(Language::En, key),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_parsing() {
        assert_eq!(Language::from_str("es").unwrap(), Language::Es);
        assert_eq!(Language::from_str("es_ES.UTF-8").unwrap(), Language::Es);
        assert_eq!(Language::from_str("en").unwrap(), Language::En);
        assert_eq!(Language::from_str("en_US").unwrap(), Language::En);
        assert_eq!(Language::from_str("fr").unwrap(), Language::Fr);
        assert_eq!(Language::from_str("de").unwrap(), Language::De);
        assert_eq!(Language::from_str("pt").unwrap(), Language::Pt);
        assert_eq!(Language::from_str("it").unwrap(), Language::It);
        assert_eq!(Language::from_str("zh").unwrap(), Language::Zh);
        assert!(Language::from_str("xx").is_err());
    }

    #[test]
    fn test_language_resolution_priority() {
        // Flag CLI tiene máxima prioridad
        assert_eq!(Language::resolve(Some("es"), Some("en")), Language::Es);
        // Config TOML entra si no hay flag CLI
        assert_eq!(Language::resolve(None, Some("es")), Language::Es);
    }

    #[test]
    fn test_translations_not_empty() {
        #[cfg(feature = "lang-en")]
        assert!(!tr(Language::En, MsgKey::CliAbout).is_empty());
        #[cfg(feature = "lang-es")]
        assert!(!tr(Language::Es, MsgKey::CliAbout).is_empty());
    }
}
