//! Módulo de gestión y administración de archivos de bóvedas .lexi.

use crate::config::get_user_config_dir;
use crate::crypto::{is_encrypted_vault, save_mappings_auto};
use crate::models::{ObfuscationError, VaultFileInfo};
use std::fs;
use std::path::{Path, PathBuf};

/// Formatea un tamaño en bytes a una representación legible (B, KB, MB).
#[allow(clippy::cast_precision_loss)]
fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;

    if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}

/// Normaliza el nombre de una bóveda asegurando la extensión .lexi.
pub fn normalize_vault_name(name: &str) -> String {
    let trimmed = name.trim();
    if std::path::Path::new(trimmed)
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("lexi"))
    {
        trimmed.to_string()
    } else {
        format!("{trimmed}.lexi")
    }
}

/// Lista todas las bóvedas presentes en el directorio de usuario ~/.lexishield/.
pub fn list_vaults() -> Result<Vec<VaultFileInfo>, ObfuscationError> {
    let config_dir = get_user_config_dir()?;
    if !config_dir.exists() {
        return Ok(Vec::new());
    }

    let mut list = Vec::new();
    let entries = fs::read_dir(&config_dir).map_err(ObfuscationError::IoError)?;

    for entry in entries.flatten() {
        let path = entry.path();
        let is_lexi = path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| e.eq_ignore_ascii_case("lexi"));

        if path.is_file() && is_lexi {
            let name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();

            let meta = fs::metadata(&path).ok();
            let size_bytes = meta.as_ref().map_or(0, std::fs::Metadata::len);
            let size_formatted = format_size(size_bytes);

            let is_encrypted = fs::read(&path).is_ok_and(|b| is_encrypted_vault(&b));

            let last_modified = meta
                .and_then(|m| m.modified().ok())
                .map(chrono_free_display);

            let bak_path = PathBuf::from(format!("{}.bak", path.display()));
            let has_backup = bak_path.exists() && bak_path.is_file();

            list.push(VaultFileInfo {
                name,
                path: path.to_string_lossy().to_string(),
                size_bytes,
                size_formatted,
                is_encrypted,
                last_modified,
                has_backup,
            });
        }
    }

    list.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(list)
}

/// Helper para formatear SystemTime a cadena ISO simple sin dependencias externas pesadas.
fn chrono_free_display(time: std::time::SystemTime) -> String {
    match time.duration_since(std::time::UNIX_EPOCH) {
        Ok(dur) => {
            let secs = dur.as_secs();
            // Formato aproximado o timestamp
            let days = secs / 86400;
            let rem = secs % 86400;
            let hours = rem / 3600;
            let mins = (rem % 3600) / 60;
            format!("D+{days} {hours:02}:{mins:02} UTC")
        }
        Err(_) => "N/A".to_string(),
    }
}

/// Crea una nueva bóveda vacía cifrada obligatoriamente con contraseña.
pub fn create_vault(name: &str, password: Option<&str>) -> Result<PathBuf, ObfuscationError> {
    let pwd = match password {
        Some(p) if !p.trim().is_empty() => p.trim(),
        _ => {
            return Err(ObfuscationError::CryptoError(
                "Es obligatorio proporcionar una contraseña para crear y proteger la bóveda."
                    .to_string(),
            ));
        }
    };

    let clean_name = normalize_vault_name(name);
    let config_dir = get_user_config_dir()?;
    let target_path = config_dir.join(&clean_name);

    if target_path.exists() {
        return Err(ObfuscationError::ConfigError(format!(
            "La bóveda '{clean_name}' ya existe en {}",
            config_dir.display()
        )));
    }

    save_mappings_auto(&target_path, &[], Some(pwd))?;
    Ok(target_path)
}

/// Renombra una bóveda existente.
pub fn rename_vault(old_name: &str, new_name: &str) -> Result<PathBuf, ObfuscationError> {
    let clean_old = normalize_vault_name(old_name);
    let clean_new = normalize_vault_name(new_name);
    let config_dir = get_user_config_dir()?;

    let old_path = config_dir.join(&clean_old);
    let new_path = config_dir.join(&clean_new);

    if !old_path.exists() {
        return Err(ObfuscationError::ConfigError(format!(
            "La bóveda de origen '{clean_old}' no existe"
        )));
    }

    if new_path.exists() {
        return Err(ObfuscationError::ConfigError(format!(
            "Ya existe una bóveda con el nombre '{clean_new}'"
        )));
    }

    fs::rename(&old_path, &new_path).map_err(ObfuscationError::IoError)?;
    Ok(new_path)
}

/// Elimina permanentemente un archivo de bóveda.
pub fn delete_vault(name: &str) -> Result<(), ObfuscationError> {
    let clean_name = normalize_vault_name(name);
    let config_dir = get_user_config_dir()?;
    let target_path = config_dir.join(&clean_name);

    if !target_path.exists() {
        return Err(ObfuscationError::ConfigError(format!(
            "La bóveda '{clean_name}' no existe"
        )));
    }

    fs::remove_file(&target_path).map_err(ObfuscationError::IoError)?;
    Ok(())
}

/// Exporta una bóveda copiando el archivo a una ruta de destino.
pub fn export_vault(name: &str, dest_path: &Path) -> Result<(), ObfuscationError> {
    let clean_name = normalize_vault_name(name);
    let config_dir = get_user_config_dir()?;
    let src_path = config_dir.join(&clean_name);

    if !src_path.exists() {
        return Err(ObfuscationError::ConfigError(format!(
            "La bóveda de origen '{clean_name}' no existe"
        )));
    }

    let final_dest = if dest_path.is_dir() {
        dest_path.join(&clean_name)
    } else {
        dest_path.to_path_buf()
    };

    fs::copy(&src_path, &final_dest).map_err(ObfuscationError::IoError)?;
    Ok(())
}

/// Importa un archivo de bóveda externo al directorio ~/.lexishield/.
pub fn import_vault(
    src_path: &Path,
    target_name: Option<&str>,
) -> Result<PathBuf, ObfuscationError> {
    if !src_path.exists() || !src_path.is_file() {
        return Err(ObfuscationError::ConfigError(format!(
            "El archivo a importar no existe: {}",
            src_path.display()
        )));
    }

    let config_dir = get_user_config_dir()?;
    let vault_name = if let Some(n) = target_name {
        normalize_vault_name(n)
    } else {
        src_path
            .file_name()
            .and_then(|f| f.to_str())
            .map_or_else(|| "imported.lexi".to_string(), normalize_vault_name)
    };

    let target_path = config_dir.join(&vault_name);
    fs::copy(src_path, &target_path).map_err(ObfuscationError::IoError)?;
    Ok(target_path)
}

/// Restaura una bóveda desde su última copia de seguridad automática (.bak).
pub fn restore_vault_backup(name: &str) -> Result<PathBuf, ObfuscationError> {
    let config_dir = get_user_config_dir()?;
    let vault_name = normalize_vault_name(name);
    let target_path = config_dir.join(&vault_name);
    let bak_path = PathBuf::from(format!("{}.bak", target_path.display()));

    if !bak_path.exists() || !bak_path.is_file() {
        return Err(ObfuscationError::ConfigError(format!(
            "No existe ninguna copia de seguridad previa (.bak) para la bóveda '{vault_name}'."
        )));
    }

    // Copiar .bak sobre la bóveda principal
    fs::copy(&bak_path, &target_path).map_err(ObfuscationError::IoError)?;
    log::info!(
        "Bóveda '{}' restaurada exitosamente desde su copia previa: {}",
        vault_name,
        bak_path.display()
    );
    Ok(target_path)
}

/// Comprueba si existe una copia de seguridad (.bak) para la bóveda indicada.
pub fn has_vault_backup(name: &str) -> Result<bool, ObfuscationError> {
    let config_dir = get_user_config_dir()?;
    let vault_name = normalize_vault_name(name);
    let target_path = config_dir.join(&vault_name);
    let bak_path = PathBuf::from(format!("{}.bak", target_path.display()));
    Ok(bak_path.exists() && bak_path.is_file())
}
