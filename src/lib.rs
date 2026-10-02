//! LexiShield - Motor de Ofuscación y Anonimización Inteligente de Logs y Datos.
//!
//! Implementación en Rust de alta eficiencia, bajo consumo de memoria y tolerancia cero a colisiones.

pub mod clipboard;
pub mod config;
pub mod crypto;
pub mod detectors;
pub mod engine;
pub mod format_adapters;
pub mod logger;
pub mod models;
pub mod stopwords;
pub mod validator;

pub use config::LexiConfig;
pub use crypto::{
    decrypt_mappings, encrypt_mappings, is_encrypted_vault, load_mappings_auto, save_mappings_auto,
};
pub use engine::{MappingManager, ObfuscatorEngine};
pub use models::{DetectorType, FormatType, Mapping, ObfuscationError, ObfuscationReport};
