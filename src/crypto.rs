//! Módulo de criptografía y almacenamiento seguro para tablas de mapeo de LexiShield.
//!
//! Implementa el formato de caja fuerte empaquetada (Vault) 100% en Rust puro:
//! - Derivación de claves basada en contraseña con Argon2id.
//! - Cifrado autenticado de datos (AEAD) con ChaCha20-Poly1305.
//! - Detección automática y compatibilidad bidireccional con archivos JSON planos.

use crate::models::{Mapping, ObfuscationError};
use argon2::Argon2;
use chacha20poly1305::ChaCha20Poly1305;
use chacha20poly1305::aead::{Aead, KeyInit, Nonce};
use rand::RngCore;
use std::fs;
use std::path::{Path, PathBuf};

/// Magic bytes que identifican un archivo de mapeos cifrado de LexiShield versión 1.
pub const VAULT_MAGIC_V1: &[u8; 5] = b"LEXI\x01";
const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const MIN_VAULT_LEN: usize = VAULT_MAGIC_V1.len() + SALT_LEN + NONCE_LEN + 16; // 16 bytes mínimo de tag Poly1305

/// Deriva una clave de 32 bytes a partir de una contraseña y una sal utilizando Argon2.
fn derive_key(password: &str, salt: &[u8]) -> Result<[u8; 32], ObfuscationError> {
    let mut key = [0u8; 32];
    let argon2 = Argon2::default();
    argon2
        .hash_password_into(password.as_bytes(), salt, &mut key)
        .map_err(|e| {
            ObfuscationError::CryptoError(format!("Error en derivación de clave Argon2: {e}"))
        })?;
    Ok(key)
}

/// Comprueba si un búfer de bytes corresponde a una caja fuerte cifrada de LexiShield.
pub fn is_encrypted_vault(data: &[u8]) -> bool {
    data.len() >= VAULT_MAGIC_V1.len() && &data[..VAULT_MAGIC_V1.len()] == VAULT_MAGIC_V1
}

/// Cifra una lista de mapeos con una contraseña y devuelve el binario empaquetado.
pub fn encrypt_mappings(mappings: &[Mapping], password: &str) -> Result<Vec<u8>, ObfuscationError> {
    if password.is_empty() {
        return Err(ObfuscationError::CryptoError(
            "La contraseña de cifrado no puede estar vacía".into(),
        ));
    }

    let json_bytes = serde_json::to_vec(mappings)?;
    log::debug!(
        "Cifrando {} mapeos con ChaCha20-Poly1305 (Payload serializado: {} bytes)...",
        mappings.len(),
        json_bytes.len()
    );

    let mut salt = [0u8; SALT_LEN];
    let mut nonce_bytes = [0u8; NONCE_LEN];
    let mut rng = rand::thread_rng();
    rng.fill_bytes(&mut salt);
    rng.fill_bytes(&mut nonce_bytes);

    let key = derive_key(password, &salt)?;
    let cipher = ChaCha20Poly1305::new_from_slice(&key)
        .map_err(|e| ObfuscationError::CryptoError(format!("Error inicializando cifrador: {e}")))?;
    let nonce = Nonce::<ChaCha20Poly1305>::from_slice(&nonce_bytes);

    let ciphertext = cipher.encrypt(nonce, json_bytes.as_ref()).map_err(|e| {
        ObfuscationError::CryptoError(format!("Error durante el cifrado AEAD: {e}"))
    })?;

    log::debug!(
        "Cifrado completado con éxito. Tamaño final del paquete: {} bytes",
        VAULT_MAGIC_V1.len() + SALT_LEN + NONCE_LEN + ciphertext.len()
    );

    let mut vault_data =
        Vec::with_capacity(VAULT_MAGIC_V1.len() + SALT_LEN + NONCE_LEN + ciphertext.len());
    vault_data.extend_from_slice(VAULT_MAGIC_V1);
    vault_data.extend_from_slice(&salt);
    vault_data.extend_from_slice(&nonce_bytes);
    vault_data.extend_from_slice(&ciphertext);

    Ok(vault_data)
}

/// Cifra un búfer arbitrario de bytes con una contraseña y devuelve el paquete cifrado.
pub fn encrypt_bytes(data: &[u8], password: &str) -> Result<Vec<u8>, ObfuscationError> {
    if password.is_empty() {
        return Err(ObfuscationError::CryptoError(
            "La contraseña de cifrado no puede estar vacía".into(),
        ));
    }

    let mut salt = [0u8; SALT_LEN];
    let mut nonce_bytes = [0u8; NONCE_LEN];
    let mut rng = rand::thread_rng();
    rng.fill_bytes(&mut salt);
    rng.fill_bytes(&mut nonce_bytes);

    let key = derive_key(password, &salt)?;
    let cipher = ChaCha20Poly1305::new_from_slice(&key)
        .map_err(|e| ObfuscationError::CryptoError(format!("Error inicializando cifrador: {e}")))?;
    let nonce = Nonce::<ChaCha20Poly1305>::from_slice(&nonce_bytes);

    let ciphertext = cipher.encrypt(nonce, data).map_err(|e| {
        ObfuscationError::CryptoError(format!("Error durante el cifrado AEAD: {e}"))
    })?;

    let mut vault_data =
        Vec::with_capacity(VAULT_MAGIC_V1.len() + SALT_LEN + NONCE_LEN + ciphertext.len());
    vault_data.extend_from_slice(VAULT_MAGIC_V1);
    vault_data.extend_from_slice(&salt);
    vault_data.extend_from_slice(&nonce_bytes);
    vault_data.extend_from_slice(&ciphertext);

    Ok(vault_data)
}

/// Descifra un búfer cifrado con contraseña y devuelve los bytes originales.
pub fn decrypt_bytes(data: &[u8], password: &str) -> Result<Vec<u8>, ObfuscationError> {
    if data.len() < MIN_VAULT_LEN {
        return Err(ObfuscationError::CryptoError(
            "El archivo cifrado está truncado o es inválido".into(),
        ));
    }

    if &data[..VAULT_MAGIC_V1.len()] != VAULT_MAGIC_V1 {
        return Err(ObfuscationError::CryptoError(
            "Formato de archivo cifrado no reconocido (Magic inválido)".into(),
        ));
    }

    let salt_start = VAULT_MAGIC_V1.len();
    let nonce_start = salt_start + SALT_LEN;
    let cipher_start = nonce_start + NONCE_LEN;

    let salt = &data[salt_start..nonce_start];
    let nonce_bytes = &data[nonce_start..cipher_start];
    let ciphertext = &data[cipher_start..];

    let key = derive_key(password, salt)?;
    let cipher = ChaCha20Poly1305::new_from_slice(&key).map_err(|e| {
        ObfuscationError::CryptoError(format!("Error inicializando descifrador: {e}"))
    })?;
    let nonce = Nonce::<ChaCha20Poly1305>::from_slice(nonce_bytes);

    cipher.decrypt(nonce, ciphertext).map_err(|_| {
        ObfuscationError::CryptoError(
            "Contraseña incorrecta o el archivo ha sido manipulado/dañado".into(),
        )
    })
}

/// Descifra un binario de caja fuerte empaquetada y devuelve la lista de mapeos.
pub fn decrypt_mappings(data: &[u8], password: &str) -> Result<Vec<Mapping>, ObfuscationError> {
    let decrypted_bytes = decrypt_bytes(data, password)?;
    let mappings: Vec<Mapping> = serde_json::from_slice(&decrypted_bytes)?;
    log::debug!(
        "Descifrado completado con éxito: {} mapeos recuperados.",
        mappings.len()
    );
    Ok(mappings)
}

/// Carga los mapeos desde un archivo, detectando automáticamente si está cifrado o en texto plano (JSON).
pub fn load_mappings_auto(
    path: &Path,
    password: Option<&str>,
) -> Result<Vec<Mapping>, ObfuscationError> {
    log::debug!("Cargando archivo de mapeos desde: {}...", path.display());
    let data = fs::read(path)?;
    if is_encrypted_vault(&data) {
        log::debug!("Detectada bóveda binaria cifrada (.lexi)");
        match password {
            Some(pwd) => decrypt_mappings(&data, pwd),
            None => Err(ObfuscationError::CryptoError(
                "El archivo de mapeos está cifrado. Debe proporcionar una contraseña para abrirlo."
                    .into(),
            )),
        }
    } else {
        log::debug!("Detectada bóveda en texto plano (JSON)");
        let mappings: Vec<Mapping> = serde_json::from_slice(&data)?;
        Ok(mappings)
    }
}

/// Guarda los mapeos en disco con escritura atómica y respaldo previo (.bak) anti-corrupción.
/// Si se proporciona contraseña, se guarda cifrado en formato seguro Vault.
pub fn save_mappings_auto(
    path: &Path,
    mappings: &[Mapping],
    password: Option<&str>,
) -> Result<(), ObfuscationError> {
    log::debug!(
        "Guardando {} mapeos en: {} (Cifrado activo: {})...",
        mappings.len(),
        path.display(),
        password.is_some()
    );

    // 1. Si el archivo ya existe y no está vacío, crear una copia de seguridad .bak
    if path.exists() && path.is_file() {
        let bak_path = PathBuf::from(format!("{}.bak", path.display()));
        if let Err(e) = fs::copy(path, &bak_path) {
            log::warn!(
                "No se pudo crear copia de seguridad previa en {}: {}",
                bak_path.display(),
                e
            );
        } else {
            log::debug!(
                "Copia de seguridad previa (.bak) actualizada en {}",
                bak_path.display()
            );
        }
    }

    // 2. Preparar los datos a escribir
    let data_to_write = if let Some(pwd) = password {
        encrypt_mappings(mappings, pwd)?
    } else {
        serde_json::to_string_pretty(mappings)?.into_bytes()
    };

    // 3. Escritura atómica a través de archivo temporal en el mismo directorio
    let tmp_path = PathBuf::from(format!("{}.tmp.{}", path.display(), std::process::id()));
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(ObfuscationError::IoError)?;
    }

    fs::write(&tmp_path, &data_to_write).map_err(ObfuscationError::IoError)?;

    // Reemplazo atómico
    fs::rename(&tmp_path, path).map_err(|e| {
        let _ = fs::remove_file(&tmp_path);
        ObfuscationError::IoError(e)
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::DetectorType;

    #[test]
    fn test_vault_encryption_roundtrip() {
        let mappings = vec![
            Mapping::new("192.168.1.50", "192.168.22.100", DetectorType::IPv4),
            Mapping::new("admin@empresa.com", "user001@acme.com", DetectorType::Email),
        ];

        let password = "SuperSecretPassword123!";
        let encrypted = encrypt_mappings(&mappings, password).expect("Cifrado falló");

        assert!(is_encrypted_vault(&encrypted));
        assert_ne!(&encrypted[..], serde_json::to_vec(&mappings).unwrap());

        let decrypted = decrypt_mappings(&encrypted, password).expect("Descifrado falló");
        assert_eq!(mappings, decrypted);
    }

    #[test]
    fn test_vault_wrong_password_fails() {
        let mappings = vec![Mapping::new("test", "pseudo", DetectorType::GenericToken)];
        let encrypted = encrypt_mappings(&mappings, "correct_password").unwrap();

        let result = decrypt_mappings(&encrypted, "wrong_password");
        assert!(result.is_err());
        if let Err(ObfuscationError::CryptoError(msg)) = result {
            assert!(msg.contains("Contraseña incorrecta"));
        } else {
            panic!("Se esperaba error de criptografía");
        }
    }

    #[test]
    fn test_vault_tampered_ciphertext_fails() {
        let mappings = vec![Mapping::new("test", "pseudo", DetectorType::GenericToken)];
        let mut encrypted = encrypt_mappings(&mappings, "password").unwrap();

        // Manipular un byte del texto cifrado
        let last_idx = encrypted.len() - 1;
        encrypted[last_idx] ^= 0xFF;

        let result = decrypt_mappings(&encrypted, "password");
        assert!(result.is_err());
    }

    #[test]
    fn test_save_mappings_auto_creates_backup() {
        let temp_dir = std::env::temp_dir().join(format!("lexi_bak_test_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);
        let file_path = temp_dir.join("test_vault.lexi");
        let bak_path = temp_dir.join("test_vault.lexi.bak");

        let initial_mappings = vec![Mapping::new("user1", "pseudo1", DetectorType::GenericToken)];
        save_mappings_auto(&file_path, &initial_mappings, None).expect("Guardado inicial falló");
        assert!(file_path.exists());
        assert!(!bak_path.exists()); // Primer guardado no tiene .bak previo

        let updated_mappings = vec![
            Mapping::new("user1", "pseudo1", DetectorType::GenericToken),
            Mapping::new("user2", "pseudo2", DetectorType::GenericToken),
        ];
        save_mappings_auto(&file_path, &updated_mappings, None).expect("Segundo guardado falló");
        assert!(file_path.exists());
        assert!(bak_path.exists()); // Segundo guardado crea .bak del estado previo

        // Comprobar que el .bak contiene el estado previo (1 mapping)
        let loaded_bak = load_mappings_auto(&bak_path, None).expect("Carga de .bak falló");
        assert_eq!(loaded_bak.len(), 1);

        // Comprobar que el actual contiene el nuevo estado (2 mappings)
        let loaded_curr = load_mappings_auto(&file_path, None).expect("Carga actual falló");
        assert_eq!(loaded_curr.len(), 2);

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
