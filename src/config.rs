//! Gestión de configuración y persistencia según estándares universales.

use crate::models::{DetectorType, ObfuscationError};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Nombre del directorio de configuración del usuario.
pub const USER_CONFIG_DIR_NAME: &str = ".lexishield";
pub const CONFIG_FILE_NAME: &str = "config.json";

/// Configuración global del motor LexiShield.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LexiConfig {
    /// Longitud mínima requerida para tokens genéricos no estructurados.
    pub min_token_length: usize,
    /// Exigir límites de palabra (\b) para evitar sustituciones en subcadenas.
    pub strict_word_boundaries: bool,
    /// Preservar claves de JSON sin alterar.
    pub preserve_json_keys: bool,
    /// Preservar nombres de etiquetas y atributos XML.
    pub preserve_xml_tags: bool,
    /// Preservar cabeceras y nombres de campo en logs estructurados.
    pub preserve_log_headers: bool,
    /// Realizar validación sintáctica post-ofuscación (Syntax Linting).
    pub validate_syntax_post_process: bool,
    /// Garantizar biyección e inyectividad estricta 1:1 en mapeos.
    pub enforce_injective_mappings: bool,
    /// Generar seudónimos sintácticamente válidos con checksums reales (DNI, Tarjetas).
    pub generate_valid_checksums: bool,
    /// Orden de prioridad de los detectores (del más específico al más genérico).
    pub detector_priority_order: Vec<DetectorType>,
}

impl Default for LexiConfig {
    fn default() -> Self {
        Self {
            min_token_length: 4,
            strict_word_boundaries: true,
            preserve_json_keys: true,
            preserve_xml_tags: true,
            preserve_log_headers: true,
            validate_syntax_post_process: true,
            enforce_injective_mappings: true,
            generate_valid_checksums: true,
            detector_priority_order: vec![
                // 1. Identificadores rígidos y específicos (máxima especificidad)
                DetectorType::GuidUuid,
                DetectorType::WindowsSid,
                DetectorType::WindowsLogonId,
                DetectorType::WindowsHexId,
                DetectorType::SpanishDniNie,
                DetectorType::CreditCard,
                // 2. Red y comunicaciones
                DetectorType::Email,
                DetectorType::IPv4,
                DetectorType::IPv6,
                DetectorType::DomainFqdn,
                // 3. Específicos de registros y servicios
                DetectorType::O365Subject,
                DetectorType::O365OriginatingServer,
                DetectorType::AttachmentFileName,
                DetectorType::Hostname,
                // 4. Genéricos y numéricos (última prioridad para evitar falsos positivos)
                DetectorType::Telephone,
                DetectorType::GenericToken,
            ],
        }
    }
}

/// Obtiene el directorio de configuración del usuario (~/.lexishield).
pub fn get_user_config_dir() -> Result<PathBuf, ObfuscationError> {
    let home = dirs::home_dir().ok_or_else(|| {
        ObfuscationError::ConfigError("No se pudo resolver el directorio HOME del usuario".into())
    })?;
    Ok(home.join(USER_CONFIG_DIR_NAME))
}

/// Inicializa el directorio y archivo de configuración en el perfil del usuario.
///
/// Cumple la regla universal: Si el archivo no existe, lo crea con la configuración
/// por defecto. Si ya existe, NO lo sobrescribe ni elimina información previa.
pub fn ensure_user_config_initialized() -> Result<PathBuf, ObfuscationError> {
    let config_dir = get_user_config_dir()?;
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir).map_err(|e| {
            ObfuscationError::ConfigError(format!(
                "Error al crear directorio de configuración {}: {}",
                config_dir.display(),
                e
            ))
        })?;
        log::info!("Directorio de usuario creado en: {}", config_dir.display());
    }

    let config_path = config_dir.join(CONFIG_FILE_NAME);
    if !config_path.exists() {
        let default_config = LexiConfig::default();
        let content = serde_json::to_string_pretty(&default_config)
            .map_err(|e| ObfuscationError::ConfigError(format!("Error serializando config: {}", e)))?;
        fs::write(&config_path, content).map_err(|e| {
            ObfuscationError::ConfigError(format!("Error escribiendo {}: {}", config_path.display(), e))
        })?;
        log::info!("Configuración base copiada a {}", config_path.display());
    }

    Ok(config_path)
}

/// Carga la configuración del usuario desde disco o devuelve la predeterminada en caso de fallo.
pub fn load_config(custom_path: Option<&Path>) -> LexiConfig {
    let path_to_load = match custom_path {
        Some(p) => p.to_path_buf(),
        None => match ensure_user_config_initialized() {
            Ok(p) => p,
            Err(err) => {
                log::warn!("No se pudo inicializar config de usuario ({}); usando valores por defecto", err);
                return LexiConfig::default();
            }
        },
    };

    match fs::read_to_string(&path_to_load) {
        Ok(data) => match serde_json::from_str::<LexiConfig>(&data) {
            Ok(cfg) => cfg,
            Err(e) => {
                log::warn!("Error al parsear {} ({}); usando valores por defecto", path_to_load.display(), e);
                LexiConfig::default()
            }
        },
        Err(e) => {
            log::warn!("Error al leer {} ({}); usando valores por defecto", path_to_load.display(), e);
            LexiConfig::default()
        }
    }
}

