//! Módulo de caché incremental híbrida en 2 fases para escaneo de archivos.
//!
//! Implementa una vía rápida por metadatos (mtime + size + rules_hash) y un fallback
//! por hash parcial (primeros y últimos 64 KB) para evitar reanalizar archivos grandes no modificados.

use crate::config::{LexiConfig, get_user_config_dir};
use crate::models::ObfuscationError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::fs::{self, File};
use std::hash::{Hash, Hasher};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const SCAN_CACHE_FILE_NAME: &str = "scan_cache.json";
const PARTIAL_HASH_CHUNK_SIZE: u64 = 65_536; // 64 KB

/// Resultado de comprobación en caché antes de escanear un archivo.
#[derive(Debug, PartialEq, Eq)]
pub enum CacheCheckResult {
    /// El archivo no ha cambiado y las reglas coinciden. Se puede omitir el análisis.
    Skip { entities_found: usize },
    /// El archivo es nuevo, ha sido modificado o las reglas cambiaron. Requiere escaneo completo.
    NeedsScan,
}

/// Registro individual de un archivo en la caché de escaneo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanCacheEntry {
    pub file_size: u64,
    pub mtime_secs: u64,
    pub mtime_nanos: u32,
    pub partial_hash: u64,
    pub rules_hash: u64,
    pub entities_found: usize,
    pub last_scanned_at: u64,
}

/// Gestor de caché persistente de escaneo en memoria y disco.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanCache {
    pub version: u32,
    pub entries: HashMap<String, ScanCacheEntry>,
    #[serde(skip)]
    pub dirty: bool,
}

impl Default for ScanCache {
    fn default() -> Self {
        Self {
            version: 1,
            entries: HashMap::new(),
            dirty: false,
        }
    }
}

impl ScanCache {
    /// Obtiene la ruta del archivo de caché en el directorio del usuario (~/.lexishield/scan_cache.json).
    pub fn get_cache_file_path() -> Result<PathBuf, ObfuscationError> {
        let dir = get_user_config_dir()?;
        if !dir.exists() {
            fs::create_dir_all(&dir).map_err(|e| {
                ObfuscationError::ConfigError(format!(
                    "No se pudo crear el directorio de caché en {}: {e}",
                    dir.display()
                ))
            })?;
        }
        Ok(dir.join(SCAN_CACHE_FILE_NAME))
    }

    /// Carga la caché desde disco o devuelve una instancia vacía si no existe.
    pub fn load() -> Self {
        let Ok(path) = Self::get_cache_file_path() else {
            return Self::default();
        };

        if !path.exists() {
            return Self::default();
        }

        match fs::read_to_string(&path) {
            Ok(content) => match serde_json::from_str::<ScanCache>(&content) {
                Ok(mut cache) => {
                    cache.dirty = false;
                    cache
                }
                Err(e) => {
                    log::warn!(
                        "Caché de escaneo corrupta en {}. Se reiniciará: {e}",
                        path.display()
                    );
                    Self::default()
                }
            },
            Err(e) => {
                log::warn!(
                    "No se pudo leer la caché de escaneo en {}: {e}",
                    path.display()
                );
                Self::default()
            }
        }
    }

    /// Guarda los cambios en disco si la caché ha sido modificada.
    pub fn save(&mut self) -> Result<(), ObfuscationError> {
        if !self.dirty {
            return Ok(());
        }

        let path = Self::get_cache_file_path()?;
        let temp_path = path.with_extension("tmp");

        {
            use std::io::Write;
            let file = File::create(&temp_path).map_err(|e| {
                ObfuscationError::ConfigError(format!(
                    "Error creando archivo temporal de caché en {}: {e}",
                    temp_path.display()
                ))
            })?;
            let mut writer = std::io::BufWriter::new(file);
            serde_json::to_writer_pretty(&mut writer, self).map_err(|e| {
                ObfuscationError::ConfigError(format!("Error serializando caché de escaneo: {e}"))
            })?;
            writer.flush().map_err(|e| {
                ObfuscationError::ConfigError(format!(
                    "Error volcando búfer de caché en {}: {e}",
                    temp_path.display()
                ))
            })?;
        }

        fs::rename(&temp_path, &path).map_err(|e| {
            ObfuscationError::ConfigError(format!(
                "Error actualizando archivo definitivo de caché en {}: {e}",
                path.display()
            ))
        })?;

        self.dirty = false;
        Ok(())
    }

    /// Limpia por completo todas las entradas de caché.
    pub fn clear(&mut self) -> Result<(), ObfuscationError> {
        self.entries.clear();
        self.dirty = true;
        self.save()?;
        log::debug!("Caché de escaneo vaciada con éxito.");
        Ok(())
    }

    /// Calcula un hash representativo de la configuración y reglas activas para invalidación.
    pub fn compute_rules_hash(config: &LexiConfig) -> u64 {
        let mut hasher = DefaultHasher::new();
        config.general.min_token_length.hash(&mut hasher);
        config.general.strict_word_boundaries.hash(&mut hasher);

        for det in &config.detector_priority_order {
            det.hash(&mut hasher);
        }

        for rule in &config.custom_rules {
            rule.hash(&mut hasher);
        }

        hasher.finish()
    }

    /// Calcula un hash parcial rápido leyendo cabecera y cola (64 KB al inicio y 64 KB al final).
    pub fn compute_partial_hash(path: &Path, file_size: u64) -> std::io::Result<u64> {
        let mut hasher = DefaultHasher::new();
        file_size.hash(&mut hasher);

        let mut file = File::open(path)?;
        let chunk_len = usize::try_from(PARTIAL_HASH_CHUNK_SIZE).unwrap_or(65_536);

        if file_size <= (PARTIAL_HASH_CHUNK_SIZE * 2) {
            let cap = usize::try_from(file_size).unwrap_or(chunk_len);
            let mut buf = Vec::with_capacity(cap);
            file.read_to_end(&mut buf)?;
            buf.hash(&mut hasher);
        } else {
            // Cabecera (primeros 64 KB)
            let mut header = vec![0u8; chunk_len];
            file.read_exact(&mut header)?;
            header.hash(&mut hasher);

            // Cola (últimos 64 KB)
            let seek_offset = i64::try_from(PARTIAL_HASH_CHUNK_SIZE).map_or(-65_536, |s| -s);
            file.seek(SeekFrom::End(seek_offset))?;
            let mut footer = vec![0u8; chunk_len];
            file.read_exact(&mut footer)?;
            footer.hash(&mut hasher);
        }

        Ok(hasher.finish())
    }

    /// Comprueba si un archivo puede saltarse usando la estrategia híbrida en 2 fases.
    pub fn check_file(&mut self, path: &Path, current_rules_hash: u64) -> CacheCheckResult {
        let Ok(meta) = fs::metadata(path) else {
            return CacheCheckResult::NeedsScan;
        };

        if !meta.is_file() {
            return CacheCheckResult::NeedsScan;
        }

        let file_size = meta.len();
        let (mtime_secs, mtime_nanos) = match meta.modified() {
            Ok(time) => match time.duration_since(UNIX_EPOCH) {
                Ok(dur) => (dur.as_secs(), dur.subsec_nanos()),
                Err(_) => (0, 0),
            },
            Err(_) => (0, 0),
        };

        let key = path.to_string_lossy().to_string();

        if let Some(entry) = self.entries.get_mut(&key) {
            // Fase 1: Vía rápida por metadatos (mtime + size + rules_hash)
            if entry.rules_hash == current_rules_hash
                && entry.file_size == file_size
                && entry.mtime_secs == mtime_secs
                && entry.mtime_nanos == mtime_nanos
            {
                return CacheCheckResult::Skip {
                    entities_found: entry.entities_found,
                };
            }

            // Fase 2: Fallback por hash parcial (si el tamaño coincide y las reglas son iguales)
            if entry.rules_hash == current_rules_hash
                && entry.file_size == file_size
                && let Ok(partial_hash) = Self::compute_partial_hash(path, file_size)
                && entry.partial_hash == partial_hash
            {
                // Actualizar mtime en caché para futuras comprobaciones ultrarrápidas
                entry.mtime_secs = mtime_secs;
                entry.mtime_nanos = mtime_nanos;
                entry.last_scanned_at = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map_or(0, |d| d.as_secs());
                self.dirty = true;

                return CacheCheckResult::Skip {
                    entities_found: entry.entities_found,
                };
            }
        }

        CacheCheckResult::NeedsScan
    }

    /// Registra o actualiza un archivo en la caché tras finalizar su escaneo.
    pub fn record_file(&mut self, path: &Path, current_rules_hash: u64, entities_found: usize) {
        let Ok(meta) = fs::metadata(path) else {
            return;
        };

        let file_size = meta.len();
        let (mtime_secs, mtime_nanos) = match meta.modified() {
            Ok(time) => match time.duration_since(UNIX_EPOCH) {
                Ok(dur) => (dur.as_secs(), dur.subsec_nanos()),
                Err(_) => (0, 0),
            },
            Err(_) => (0, 0),
        };

        let partial_hash = Self::compute_partial_hash(path, file_size).unwrap_or(0);
        let now_secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_secs());

        let key = path.to_string_lossy().to_string();
        self.entries.insert(
            key,
            ScanCacheEntry {
                file_size,
                mtime_secs,
                mtime_nanos,
                partial_hash,
                rules_hash: current_rules_hash,
                entities_found,
                last_scanned_at: now_secs,
            },
        );
        self.dirty = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_scan_cache_lifecycle() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("lexishield_cache_test.txt");
        let mut f = File::create(&test_file).unwrap();
        writeln!(f, "EMP-12345 secret employee token").unwrap();
        drop(f);

        let mut cache = ScanCache::default();
        let rules_hash_v1 = 123456789u64;
        let rules_hash_v2 = 987654321u64;

        // 1. Archivo nuevo -> NeedsScan
        assert_eq!(
            cache.check_file(&test_file, rules_hash_v1),
            CacheCheckResult::NeedsScan
        );

        // 2. Registrar en caché
        cache.record_file(&test_file, rules_hash_v1, 1);

        // 3. Fast-path hit -> Skip
        assert_eq!(
            cache.check_file(&test_file, rules_hash_v1),
            CacheCheckResult::Skip { entities_found: 1 }
        );

        // 4. Invalidad si cambian las reglas
        assert_eq!(
            cache.check_file(&test_file, rules_hash_v2),
            CacheCheckResult::NeedsScan
        );

        // 5. Modificar archivo -> NeedsScan
        let mut f = File::create(&test_file).unwrap();
        writeln!(f, "EMP-12345 EMP-67890 another line with more content").unwrap();
        drop(f);

        assert_eq!(
            cache.check_file(&test_file, rules_hash_v1),
            CacheCheckResult::NeedsScan
        );

        let _ = fs::remove_file(test_file);
    }

    #[test]
    fn test_partial_hash_computation() {
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("lexishield_partial_hash_test.bin");
        let mut f = File::create(&test_file).unwrap();
        let data = vec![0xABu8; 150_000]; // 150 KB (> 128 KB)
        f.write_all(&data).unwrap();
        drop(f);

        let hash1 = ScanCache::compute_partial_hash(&test_file, 150_000).unwrap();
        assert_ne!(hash1, 0);

        let hash2 = ScanCache::compute_partial_hash(&test_file, 150_000).unwrap();
        assert_eq!(hash1, hash2);

        let _ = fs::remove_file(test_file);
    }
}
