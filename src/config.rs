//! Gestión de configuración y persistencia según estándares universales en formato TOML.

use crate::detectors::custom::CustomRule;
use crate::models::{DetectorType, ObfuscationError};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// Nombre del directorio de configuración del usuario en su perfil.
pub const USER_CONFIG_DIR_NAME: &str = ".lexishield";
pub const CONFIG_FILE_NAME: &str = "config.toml";
pub const RULES_FILE_NAME: &str = "rules.toml";

/// Plantilla predeterminada y comentada para reglas personalizadas de usuario (~/.lexishield/rules.toml).
pub const DEFAULT_RULES_TOML: &str = r#"# ==============================================================================
# Reglas Personalizadas de Detección para LexiShield (~/.lexishield/rules.toml)
# ==============================================================================
# Aquí puedes añadir tus propios patrones de expresiones regulares para detectar
# datos sensibles específicos de tu organización sin tener que recompilar el binario.
#
# Estrategias disponibles:
#   - "random_digits": Genera dígitos aleatorios preservando longitud y formato.
#   - "random_hex": Genera caracteres hexadecimales aleatorios.
#   - "random_alphanumeric": Genera caracteres alfanuméricos aleatorios.
#   - "prefix_seq": Prefijo seguido de un identificador numérico aleatorio.
#   - "mask": Enmascara la coincidencia con asteriscos o marcador fijo.
#
# Ejemplos:

[[rules]]
name = "Identificador de Empleado"
pattern = '(?i)\bEMP-\d{4,6}\b'
prefix = "EMP-"
strategy = "random_digits"

[[rules]]
name = "Código de Proyecto Interno"
pattern = '(?i)\bPRJ-[A-Z0-9]{3,6}\b'
prefix = "PRJ-"
strategy = "random_alphanumeric"

[[rules]]
name = "Token de Acceso a Servicios"
pattern = '(?i)\b(ghp|glpat|npm)_[a-zA-Z0-9]{20,}\b'
strategy = "mask"
"#;

/// Plantilla predeterminada y comentada de configuración en formato TOML.
pub const DEFAULT_CONFIG_TOML: &str = r#"# ==============================================================================
# Configuración Global de LexiShield (~/.lexishield/config.toml)
# ==============================================================================

[general]
# Longitud mínima requerida para tokens genéricos no estructurados.
min_token_length = 4

# Exigir límites de palabra (\b) para evitar sustituciones accidentales en subcadenas.
strict_word_boundaries = true

# Preservar claves de JSON sin alterar.
preserve_json_keys = true

# Preservar nombres de etiquetas y atributos XML.
preserve_xml_tags = true

# Preservar cabeceras y nombres de campo en logs estructurados.
preserve_log_headers = true

# Realizar validación sintáctica post-ofuscación (Syntax Linting).
validate_syntax_post_process = true

# Garantizar biyección e inyectividad estricta 1:1 en mapeos.
enforce_injective_mappings = true

# Generar seudónimos sintácticamente válidos con checksums reales (DNI, Tarjetas).
generate_valid_checksums = true

[vault]
# Nombre o ruta del archivo de bóveda por defecto dentro de ~/.lexishield/
default_vault = "default.lexi"

[ignore]
# Directorios ignorados automáticamente durante el escaneo recursivo
ignored_directories = [
    ".git",
    "node_modules",
    "target",
    ".vagrant",
    ".vagrant.d",
    ".vscode",
    ".idea",
    "vendor",
    "dist",
    "build",
]

# Extensiones de archivos binarios/multimedia ignoradas durante el escaneo
ignored_extensions = [
    "png", "jpg", "jpeg", "gif", "ico", "svg", "webp",
    "pdf", "zip", "tar", "gz", "tgz", "bz2", "7z", "rar",
    "exe", "bin", "dll", "so", "dylib", "lock",
    "woff", "woff2", "ttf", "eot", "mp4", "mp3", "avi", "mkv",
]

# Orden de prioridad de los detectores estándar incorporados
detector_priority_order = [
    "GuidUuid",
    "WindowsSid",
    "WindowsLogonId",
    "WindowsHexId",
    "SpanishDniNie",
    "CreditCard",
    "Email",
    "IPv4",
    "IPv6",
    "DomainFqdn",
    "O365Subject",
    "O365OriginatingServer",
    "AttachmentFileName",
    "Hostname",
    "Telephone",
    "GenericToken",
]

# ==============================================================================
# Reglas personalizadas de detección definidas por el usuario (Custom Rules)
# ==============================================================================
# Estrategias disponibles:
#   - "random_digits": Genera dígitos aleatorios preservando longitud y formato.
#   - "random_hex": Genera caracteres hexadecimales aleatorios.
#   - "random_alphanumeric": Genera caracteres alfanuméricos aleatorios.
#   - "prefix_seq": Prefijo seguido de un identificador numérico aleatorio.
#   - "mask": Enmascara la coincidencia con asteriscos o marcador fijo.
#
# Ejemplos (descomentar y adaptar para activar):
#
# [[custom_rules]]
# name = "Identificador de Empleado"
# pattern = '(?i)\bEMP-\d{4,6}\b'
# prefix = "EMP-"
# strategy = "random_digits"
#
# [[custom_rules]]
# name = "Código de Proyecto Interno"
# pattern = '(?i)\bPRJ-[A-Z0-9]{3,6}\b'
# prefix = "PRJ-"
# strategy = "random_alphanumeric"
#
# [[custom_rules]]
# name = "Token de Acceso de Servicio"
# pattern = '(?i)\b(ghp|glpat|npm)_[a-zA-Z0-9]{20,}\b'
# strategy = "mask"
"#;

/// Configuración general de opciones de procesamiento.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(clippy::struct_excessive_bools)]
pub struct GeneralConfig {
    #[serde(default = "default_min_token_length")]
    pub min_token_length: usize,
    #[serde(default = "default_true")]
    pub strict_word_boundaries: bool,
    #[serde(default = "default_true")]
    pub preserve_json_keys: bool,
    #[serde(default = "default_true")]
    pub preserve_xml_tags: bool,
    #[serde(default = "default_true")]
    pub preserve_log_headers: bool,
    #[serde(default = "default_true")]
    pub validate_syntax_post_process: bool,
    #[serde(default = "default_true")]
    pub enforce_injective_mappings: bool,
    #[serde(default = "default_true")]
    pub generate_valid_checksums: bool,
}

fn default_min_token_length() -> usize {
    4
}

fn default_true() -> bool {
    true
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            min_token_length: default_min_token_length(),
            strict_word_boundaries: true,
            preserve_json_keys: true,
            preserve_xml_tags: true,
            preserve_log_headers: true,
            validate_syntax_post_process: true,
            enforce_injective_mappings: true,
            generate_valid_checksums: true,
        }
    }
}

/// Configuración de bóvedas y persistencia.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultConfig {
    #[serde(default = "default_vault_name")]
    pub default_vault: String,
}

fn default_vault_name() -> String {
    "default.lexi".to_string()
}

impl Default for VaultConfig {
    fn default() -> Self {
        Self {
            default_vault: default_vault_name(),
        }
    }
}

/// Configuración de exclusiones e ignorados durante escaneos.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IgnoreConfig {
    #[serde(default = "default_ignored_directories")]
    pub ignored_directories: Vec<String>,
    #[serde(default = "default_ignored_extensions")]
    pub ignored_extensions: Vec<String>,
}

fn default_ignored_directories() -> Vec<String> {
    vec![
        ".git".into(),
        "node_modules".into(),
        "target".into(),
        ".vagrant".into(),
        ".vagrant.d".into(),
        ".vscode".into(),
        ".idea".into(),
        "vendor".into(),
        "dist".into(),
        "build".into(),
    ]
}

fn default_ignored_extensions() -> Vec<String> {
    vec![
        "png".into(),
        "jpg".into(),
        "jpeg".into(),
        "gif".into(),
        "ico".into(),
        "svg".into(),
        "webp".into(),
        "pdf".into(),
        "zip".into(),
        "tar".into(),
        "gz".into(),
        "tgz".into(),
        "bz2".into(),
        "7z".into(),
        "rar".into(),
        "exe".into(),
        "bin".into(),
        "dll".into(),
        "so".into(),
        "dylib".into(),
        "lock".into(),
        "woff".into(),
        "woff2".into(),
        "ttf".into(),
        "eot".into(),
        "mp4".into(),
        "mp3".into(),
        "avi".into(),
        "mkv".into(),
    ]
}

impl Default for IgnoreConfig {
    fn default() -> Self {
        Self {
            ignored_directories: default_ignored_directories(),
            ignored_extensions: default_ignored_extensions(),
        }
    }
}

fn default_priority_order() -> Vec<DetectorType> {
    vec![
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
        // 4. Genéricos y numéricos
        DetectorType::Telephone,
        DetectorType::GenericToken,
    ]
}

/// Configuración global del motor LexiShield.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LexiConfig {
    #[serde(default)]
    pub general: GeneralConfig,

    #[serde(default)]
    pub vault: VaultConfig,

    #[serde(default)]
    pub ignore: IgnoreConfig,

    #[serde(default = "default_priority_order")]
    pub detector_priority_order: Vec<DetectorType>,

    #[serde(default)]
    pub custom_rules: Vec<CustomRule>,
}

impl Default for LexiConfig {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            vault: VaultConfig::default(),
            ignore: IgnoreConfig::default(),
            detector_priority_order: default_priority_order(),
            custom_rules: Vec::new(),
        }
    }
}

impl LexiConfig {
    // Métodos de conveniencia para mantener compatibilidad con accesos directos
    pub fn min_token_length(&self) -> usize {
        self.general.min_token_length
    }
    pub fn strict_word_boundaries(&self) -> bool {
        self.general.strict_word_boundaries
    }
    pub fn preserve_json_keys(&self) -> bool {
        self.general.preserve_json_keys
    }
    pub fn preserve_xml_tags(&self) -> bool {
        self.general.preserve_xml_tags
    }
    pub fn preserve_log_headers(&self) -> bool {
        self.general.preserve_log_headers
    }
    pub fn validate_syntax_post_process(&self) -> bool {
        self.general.validate_syntax_post_process
    }
    pub fn enforce_injective_mappings(&self) -> bool {
        self.general.enforce_injective_mappings
    }
    pub fn generate_valid_checksums(&self) -> bool {
        self.general.generate_valid_checksums
    }
}

/// Obtiene el directorio de configuración del usuario (~/.lexishield).
pub fn get_user_config_dir() -> Result<PathBuf, ObfuscationError> {
    let home = dirs::home_dir().ok_or_else(|| {
        ObfuscationError::ConfigError("No se pudo resolver el directorio HOME del usuario".into())
    })?;
    Ok(home.join(USER_CONFIG_DIR_NAME))
}

/// Obtiene la ruta del fichero de bóveda por defecto a partir de la configuración.
pub fn get_default_vault_path() -> Result<PathBuf, ObfuscationError> {
    let config_dir = get_user_config_dir()?;
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir).map_err(|e| {
            ObfuscationError::ConfigError(format!(
                "Error al crear directorio de configuración {}: {}",
                config_dir.display(),
                e
            ))
        })?;
    }
    Ok(config_dir.join("default.lexi"))
}

/// Obtiene la ruta del archivo de configuración del usuario (~/.lexishield/config.toml).
pub fn get_user_config_path() -> Result<PathBuf, ObfuscationError> {
    let dir = get_user_config_dir()?;
    Ok(dir.join(CONFIG_FILE_NAME))
}

/// Obtiene la ruta del archivo de reglas personalizadas del usuario (~/.lexishield/rules.toml).
pub fn get_user_rules_path() -> Result<PathBuf, ObfuscationError> {
    let dir = get_user_config_dir()?;
    Ok(dir.join(RULES_FILE_NAME))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RulesContainer {
    #[serde(default, alias = "rules")]
    pub custom_rules: Vec<CustomRule>,
}

/// Carga reglas personalizadas desde un archivo TOML arbitrario.
pub fn load_rules_from_file(path: &Path) -> Vec<CustomRule> {
    if !path.exists() {
        return Vec::new();
    }
    match fs::read_to_string(path) {
        Ok(content) => {
            if let Ok(container) = toml::from_str::<RulesContainer>(&content)
                && !container.custom_rules.is_empty()
            {
                return container.custom_rules;
            }
            if let Ok(direct_list) = toml::from_str::<Vec<CustomRule>>(&content) {
                return direct_list;
            }
            log::warn!(
                "No se pudieron interpretar las reglas personalizadas de {}",
                path.display()
            );
            Vec::new()
        }
        Err(e) => {
            log::warn!("Error al leer archivo de reglas {}: {}", path.display(), e);
            Vec::new()
        }
    }
}

/// Restablece la configuración predeterminada creando o sobreescribiendo ~/.lexishield/config.toml.
pub fn reset_config() -> Result<PathBuf, ObfuscationError> {
    let config_dir = get_user_config_dir()?;
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir).map_err(|e| {
            ObfuscationError::ConfigError(format!(
                "Error al crear directorio de configuración {}: {}",
                config_dir.display(),
                e
            ))
        })?;
    }

    let config_path = config_dir.join(CONFIG_FILE_NAME);
    fs::write(&config_path, DEFAULT_CONFIG_TOML).map_err(|e| {
        ObfuscationError::ConfigError(format!(
            "Error al restaurar archivo {}: {}",
            config_path.display(),
            e
        ))
    })?;

    let rules_path = config_dir.join(RULES_FILE_NAME);
    let _ = fs::write(&rules_path, DEFAULT_RULES_TOML);

    log::info!(
        "Configuración y reglas restauradas a valores por defecto en: {}",
        config_path.display()
    );
    Ok(config_path)
}

/// Inicializa el directorio y archivos de configuración en el perfil del usuario.
///
/// Cumple la regla universal: Si los archivos no existen, los crea con las plantillas
/// predeterminadas. Si ya existen, NO los sobrescribe para respetar los datos del usuario.
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
        fs::write(&config_path, DEFAULT_CONFIG_TOML).map_err(|e| {
            ObfuscationError::ConfigError(format!(
                "Error escribiendo {}: {}",
                config_path.display(),
                e
            ))
        })?;
        log::info!(
            "Plantilla de configuración TOML creada en {}",
            config_path.display()
        );
    }

    let rules_path = config_dir.join(RULES_FILE_NAME);
    if !rules_path.exists() {
        let _ = fs::write(&rules_path, DEFAULT_RULES_TOML);
        log::info!(
            "Plantilla de reglas personalizadas creada en {}",
            rules_path.display()
        );
    }

    Ok(config_path)
}

/// Carga la configuración del usuario y combina reglas dinámicas adicionales.
pub fn load_config_with_rules(
    custom_config: Option<&Path>,
    custom_rules: Option<&Path>,
) -> LexiConfig {
    let path_to_load = match custom_config {
        Some(p) => p.to_path_buf(),
        None => match ensure_user_config_initialized() {
            Ok(p) => p,
            Err(err) => {
                log::warn!(
                    "No se pudo inicializar config de usuario ({err}); usando valores por defecto"
                );
                return LexiConfig::default();
            }
        },
    };

    let mut cfg = match fs::read_to_string(&path_to_load) {
        Ok(data) => match toml::from_str::<LexiConfig>(&data) {
            Ok(c) => c,
            Err(e) => {
                log::warn!(
                    "Error al parsear archivo TOML {} ({}); usando valores por defecto",
                    path_to_load.display(),
                    e
                );
                LexiConfig::default()
            }
        },
        Err(e) => {
            log::warn!(
                "Error al leer {} ({}); usando valores por defecto",
                path_to_load.display(),
                e
            );
            LexiConfig::default()
        }
    };

    // Cargar reglas dinámicas adicionales desde rules.toml o ruta indicada
    let rules_to_load = match custom_rules {
        Some(p) => Some(p.to_path_buf()),
        None => get_user_rules_path().ok(),
    };

    if let Some(r_path) = rules_to_load
        && r_path.exists()
    {
        let external_rules = load_rules_from_file(&r_path);
        if !external_rules.is_empty() {
            log::info!(
                "Cargadas {} reglas personalizadas desde {}",
                external_rules.len(),
                r_path.display()
            );
            for r in external_rules {
                if !cfg
                    .custom_rules
                    .iter()
                    .any(|existing| existing.name == r.name)
                {
                    cfg.custom_rules.push(r);
                }
            }
        }
    }

    cfg
}

/// Carga la configuración del usuario desde disco o devuelve la predeterminada en caso de fallo.
pub fn load_config(custom_path: Option<&Path>) -> LexiConfig {
    load_config_with_rules(custom_path, None)
}
