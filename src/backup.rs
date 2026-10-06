//! Módulo de copias de seguridad de alta compresión (tar.bz2) y restauración con cifrado seguro.

use crate::config::get_user_config_dir;
use crate::crypto::{decrypt_bytes, encrypt_bytes, is_encrypted_vault};
use crate::models::ObfuscationError;
use bzip2::Compression;
use bzip2::read::BzDecoder;
use bzip2::write::BzEncoder;
use std::fs::{self, File};
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use tar::{Archive, Builder};

/// Crea una copia de seguridad completa del perfil de usuario (~/.lexishield) comprimida con Bzip2.
/// Si se pasa contraseña, el paquete tar.bz2 resultante se cifra con AES-256 / ChaCha20-Poly1305.
/// Empaqueta y comprime un directorio origen en formato tar.bz2 (con cifrado opcional).
pub fn create_backup_from_dir(
    source_dir: &Path,
    output_path: &Path,
    password: Option<&str>,
) -> Result<PathBuf, ObfuscationError> {
    if !source_dir.exists() {
        return Err(ObfuscationError::ConfigError(format!(
            "El directorio de origen no existe: {}",
            source_dir.display()
        )));
    }

    log::info!(
        "Iniciando empaquetado de copia de seguridad Bzip2 desde: {}",
        source_dir.display()
    );

    // 1. Crear el tar en memoria
    let tar_buffer = Vec::new();
    let mut tar_builder = Builder::new(tar_buffer);

    let entries = fs::read_dir(source_dir).map_err(ObfuscationError::IoError)?;
    let mut files_packed = 0;

    for entry in entries.flatten() {
        let path = entry.path();
        let is_valid_ext = path.extension().and_then(|e| e.to_str()).is_some_and(|e| {
            e.eq_ignore_ascii_case("toml")
                || e.eq_ignore_ascii_case("lexi")
                || e.eq_ignore_ascii_case("json")
        });

        if path.is_file()
            && is_valid_ext
            && let Some(filename) = path.file_name().and_then(|n| n.to_str())
        {
            let mut file = File::open(&path).map_err(ObfuscationError::IoError)?;
            tar_builder
                .append_file(filename, &mut file)
                .map_err(ObfuscationError::IoError)?;
            files_packed += 1;
            log::debug!("Empaquetado en backup: {filename}");
        }
    }

    let uncompressed_tar = tar_builder
        .into_inner()
        .map_err(ObfuscationError::IoError)?;

    log::info!(
        "Archivados {files_packed} ficheros ({} bytes sin comprimir). Comprimiendo con Bzip2 (Best)...",
        uncompressed_tar.len()
    );

    // 2. Comprimir con Bzip2 nivel 9 (máxima compresión)
    let mut encoder = BzEncoder::new(Vec::new(), Compression::best());
    encoder
        .write_all(&uncompressed_tar)
        .map_err(ObfuscationError::IoError)?;
    let compressed_bz2 = encoder.finish().map_err(ObfuscationError::IoError)?;

    log::info!(
        "Compresión Bzip2 completada. Tamaño comprimido: {} bytes",
        compressed_bz2.len()
    );

    // 3. Cifrado si se especifica contraseña
    let final_data = if let Some(pwd) = password {
        if pwd.trim().is_empty() {
            compressed_bz2
        } else {
            log::info!("Cifrando archivo de copia de seguridad con clave derivada...");
            encrypt_bytes(&compressed_bz2, pwd.trim())?
        }
    } else {
        compressed_bz2
    };

    // 4. Guardar archivo en disco
    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent).map_err(ObfuscationError::IoError)?;
    }

    fs::write(output_path, final_data).map_err(ObfuscationError::IoError)?;
    log::info!(
        "Copia de seguridad guardada con éxito en: {}",
        output_path.display()
    );

    Ok(output_path.to_path_buf())
}

/// Crea una copia de seguridad completa del perfil de usuario (~/.lexishield) comprimida con Bzip2.
pub fn create_backup(
    output_path: &Path,
    password: Option<&str>,
) -> Result<PathBuf, ObfuscationError> {
    let _ = crate::config::ensure_user_config_initialized();
    let config_dir = get_user_config_dir()?;
    create_backup_from_dir(&config_dir, output_path, password)
}

/// Restaura una copia de seguridad Bzip2 (opcionalmente cifrada) sobre un directorio destino.
pub fn restore_backup_to_dir(
    backup_path: &Path,
    target_dir: &Path,
    password: Option<&str>,
) -> Result<usize, ObfuscationError> {
    if !backup_path.exists() || !backup_path.is_file() {
        return Err(ObfuscationError::ConfigError(format!(
            "El archivo de copia de seguridad no existe: {}",
            backup_path.display()
        )));
    }

    let raw_bytes = fs::read(backup_path).map_err(ObfuscationError::IoError)?;
    log::info!(
        "Leyendo archivo de copia de seguridad ({} bytes)...",
        raw_bytes.len()
    );

    // 1. Descifrar si está cifrado
    let bz2_bytes = if is_encrypted_vault(&raw_bytes) {
        log::info!("Copia de seguridad cifrada detectada. Descifrando...");
        match password {
            Some(pwd) if !pwd.trim().is_empty() => decrypt_bytes(&raw_bytes, pwd.trim())?,
            _ => {
                return Err(ObfuscationError::CryptoError(
                    "La copia de seguridad está cifrada. Debe proporcionar la contraseña correcta."
                        .into(),
                ));
            }
        }
    } else {
        raw_bytes
    };

    // 2. Descomprimir Bzip2
    log::info!("Descomprimiendo flujo Bzip2...");
    let mut decoder = BzDecoder::new(Cursor::new(bz2_bytes));
    let mut decompressed_tar = Vec::new();
    decoder.read_to_end(&mut decompressed_tar).map_err(|e| {
        ObfuscationError::IoError(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("Error al descomprimir Bzip2: {e}"),
        ))
    })?;

    // 3. Extraer el archivo Tar en el directorio de destino
    if !target_dir.exists() {
        fs::create_dir_all(target_dir).map_err(ObfuscationError::IoError)?;
    }

    let mut archive = Archive::new(Cursor::new(decompressed_tar));
    let mut restored_count = 0;

    let entries = archive.entries().map_err(ObfuscationError::IoError)?;
    for entry_result in entries {
        let mut entry = entry_result.map_err(ObfuscationError::IoError)?;
        let path = entry
            .path()
            .map_err(ObfuscationError::IoError)?
            .to_path_buf();

        // Prevención de path traversal (seguridad zip-slip)
        if let Some(file_name) = path.file_name() {
            let dest_file = target_dir.join(file_name);
            entry
                .unpack(&dest_file)
                .map_err(ObfuscationError::IoError)?;
            restored_count += 1;
            log::info!("Restaurado fichero: {}", dest_file.display());
        }
    }

    log::info!("Restauración completada. {restored_count} ficheros recuperados.");
    Ok(restored_count)
}

/// Restaura una copia de seguridad Bzip2 (opcionalmente cifrada) sobre el perfil ~/.lexishield.
pub fn restore_backup(
    backup_path: &Path,
    password: Option<&str>,
) -> Result<usize, ObfuscationError> {
    let config_dir = get_user_config_dir()?;
    restore_backup_to_dir(backup_path, &config_dir, password)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backup_and_restore_roundtrip() {
        let temp_dir = std::env::temp_dir().join(format!("lexi_test_dir_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);

        let sample_file = temp_dir.join("sample.toml");
        fs::write(&sample_file, "key = 'value'").unwrap();

        let backup_file =
            std::env::temp_dir().join(format!("lexi_test_backup_{}.tar.bz2", std::process::id()));
        let restore_dest =
            std::env::temp_dir().join(format!("lexi_restore_{}", std::process::id()));

        let pwd = "SuperBackupPassword123";
        let created = create_backup_from_dir(&temp_dir, &backup_file, Some(pwd))
            .expect("Fallo al crear backup");
        assert!(created.exists());

        let count = restore_backup_to_dir(&backup_file, &restore_dest, Some(pwd))
            .expect("Fallo al restaurar backup");
        assert_eq!(count, 1);
        assert!(restore_dest.join("sample.toml").exists());

        let _ = fs::remove_dir_all(&temp_dir);
        let _ = fs::remove_dir_all(&restore_dest);
        let _ = fs::remove_file(&backup_file);
    }
}
