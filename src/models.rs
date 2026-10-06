//! Modelos de datos y tipos de errores para LexiShield.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Tipos de formatos estructurados soportados por el motor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FormatType {
    Auto,
    Json,
    Xml,
    Log,
    Plaintext,
}

/// Tipos de detectores semánticos soportados por LexiShield.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DetectorType {
    GuidUuid,
    WindowsSid,
    WindowsLogonId,
    WindowsHexId,
    IPv4,
    IPv6,
    DomainFqdn,
    Email,
    Hostname,
    SpanishDniNie,
    CreditCard,
    Telephone,
    O365Subject,
    O365OriginatingServer,
    AttachmentFileName,
    GenericToken,
    Custom(String),
}

impl std::str::FromStr for DetectorType {

    type Err = ();



    fn from_str(s: &str) -> Result<Self, Self::Err> {

        match s {

            "GUID/UUID" => Ok(Self::GuidUuid),

            "Windows SID" => Ok(Self::WindowsSid),

            "Windows Logon ID" => Ok(Self::WindowsLogonId),

            "Windows Hex ID" => Ok(Self::WindowsHexId),

            "IPv4" => Ok(Self::IPv4),

            "IPv6" => Ok(Self::IPv6),

            "Domain/FQDN" => Ok(Self::DomainFqdn),

            "Email" => Ok(Self::Email),

            "Hostname" => Ok(Self::Hostname),

            "DNI/NIE" => Ok(Self::SpanishDniNie),

            "Credit Card" => Ok(Self::CreditCard),

            "Telephone" => Ok(Self::Telephone),

            "O365 Subject" => Ok(Self::O365Subject),

            "O365 Server Hostname" => Ok(Self::O365OriginatingServer),

            "Attachment File Name" => Ok(Self::AttachmentFileName),

            "Generic Token" => Ok(Self::GenericToken),

            other => {
                let mut current = other.trim();
                let mut peeled = false;
                while current.starts_with("Custom (") && current.ends_with(')') {
                    current = current[8..current.len() - 1].trim();
                    peeled = true;
                }
                
                if peeled {
                    // Try parsing the inner value in case it's a known standard type
                    if let Ok(standard_type) = current.parse::<Self>() {
                        return Ok(standard_type);
                    }
                }
                
                Ok(Self::Custom(current.to_string()))
            }

        }

    }

}


impl fmt::Display for DetectorType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GuidUuid => write!(f, "GUID/UUID"),
            Self::WindowsSid => write!(f, "Windows SID"),
            Self::WindowsLogonId => write!(f, "Windows Logon ID"),
            Self::WindowsHexId => write!(f, "Windows Hex ID"),
            Self::IPv4 => write!(f, "IPv4"),
            Self::IPv6 => write!(f, "IPv6"),
            Self::DomainFqdn => write!(f, "Domain/FQDN"),
            Self::Email => write!(f, "Email"),
            Self::Hostname => write!(f, "Hostname"),
            Self::SpanishDniNie => write!(f, "DNI/NIE"),
            Self::CreditCard => write!(f, "Credit Card"),
            Self::Telephone => write!(f, "Telephone"),
            Self::O365Subject => write!(f, "O365 Subject"),
            Self::O365OriginatingServer => write!(f, "O365 Server Hostname"),
            Self::AttachmentFileName => write!(f, "Attachment File Name"),
            Self::GenericToken => write!(f, "Generic Token"),
            Self::Custom(name) => write!(f, "Custom ({name})"),
        }
    }
}

/// Representa una entrada de mapeo bidireccional entre el valor original y su seudónimo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mapping {
    pub original: String,
    pub pseudonym: String,
    pub detector_type: DetectorType,
    pub omitted: bool,
}

impl Mapping {
    pub fn new(
        original: impl Into<String>,
        pseudonym: impl Into<String>,
        detector_type: DetectorType,
    ) -> Self {
        let orig = original.into();
        let pseudo = pseudonym.into();
        let omitted = orig == "=" || pseudo == "=";
        Self {
            original: orig,
            pseudonym: pseudo,
            detector_type,
            omitted,
        }
    }
}

/// Informe de resultados de una operación de ofuscación o desofuscación.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObfuscationReport {
    pub format_detected: FormatType,
    pub original_length: usize,
    pub result_length: usize,
    pub replacements_applied: usize,
    pub elapsed_ms: f64,
    pub schema_intact: bool,
    pub syntax_valid: bool,
    pub warnings: Vec<String>,
}

/// Tipos de error del motor de ofuscación.
#[derive(Debug)]
pub enum ObfuscationError {
    IoError(std::io::Error),
    JsonError(serde_json::Error),
    XmlError(String),
    CollisionError(String),
    ValidationError(String),
    InvalidFormat(String),
    ConfigError(String),
    CryptoError(String),
}

impl fmt::Display for ObfuscationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IoError(e) => write!(f, "Error de E/S: {e}"),
            Self::JsonError(e) => write!(f, "Error de sintaxis JSON: {e}"),
            Self::XmlError(msg) => write!(f, "Error de estructura XML: {msg}"),
            Self::CollisionError(msg) => write!(f, "Colisión en tabla de mapeos: {msg}"),
            Self::ValidationError(msg) => write!(f, "Error de validación post-ofuscación: {msg}"),
            Self::InvalidFormat(msg) => write!(f, "Formato no válido: {msg}"),
            Self::ConfigError(msg) => write!(f, "Error en configuración: {msg}"),
            Self::CryptoError(msg) => write!(f, "Error criptográfico/bóveda segura: {msg}"),
        }
    }
}

impl std::error::Error for ObfuscationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::IoError(e) => Some(e),
            Self::JsonError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for ObfuscationError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}

impl From<serde_json::Error> for ObfuscationError {
    fn from(err: serde_json::Error) -> Self {
        Self::JsonError(err)
    }
}

/// Información descriptiva de un archivo de bóveda .lexi en disco.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultFileInfo {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub size_formatted: String,
    pub is_encrypted: bool,
    pub last_modified: Option<String>,
    #[serde(default)]
    pub has_backup: bool,
}

/// Parámetros de auditoría detallada de escaneo.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScanAuditMetrics {
    pub path_analyzed: String,
    pub files_scanned: usize,
    pub files_ignored: usize,
    pub total_entities: usize,
    pub duration_ms: f64,
}
