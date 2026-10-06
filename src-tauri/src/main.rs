#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use lexishield::clipboard::{get_clipboard_text, set_clipboard_text};
use lexishield::config::load_config_with_rules;
use lexishield::crypto::{load_mappings_auto, save_mappings_auto};
use lexishield::engine::ObfuscatorEngine;
use lexishield::logger::{
    clear_logs, get_recent_logs, init_logger, is_debug_mode, set_debug_mode, LogEntry,
};
use lexishield::models::{DetectorType, FormatType, Mapping, ObfuscationReport};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tauri::Emitter;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

static SCAN_CANCELLED: AtomicBool = AtomicBool::new(false);

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct CurrentScanStatus {
    pub is_scanning: bool,
    pub current_file: String,
    pub files_scanned: usize,
    pub entities_found: usize,
}

static CURRENT_SCAN: OnceLock<Mutex<CurrentScanStatus>> = OnceLock::new();

fn get_current_scan_status() -> &'static Mutex<CurrentScanStatus> {
    CURRENT_SCAN.get_or_init(|| Mutex::new(CurrentScanStatus::default()))
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ScanProgressPayload {
    pub current_file: String,
    pub files_scanned: usize,
    pub entities_found: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ObfuscateResponse {
    pub obfuscated_text: String,
    pub report: ObfuscationReport,
    pub mappings: Vec<Mapping>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeobfuscateResponse {
    pub deobfuscated_text: String,
    pub report: ObfuscationReport,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DetectedItem {
    pub original: String,
    pub pseudonym: String,
    pub detector_type: String,
    pub omitted: bool,
}

#[tauri::command]
fn obfuscate_content(
    text: String,
    format_str: String,
    vault_path: Option<String>,
    password: Option<String>,
) -> Result<ObfuscateResponse, String> {
    let config = load_config_with_rules(None, None);
    let mut engine = ObfuscatorEngine::new(config);

    // Si se pasa una bóveda previa, cargar los mapeos existentes
    if let Some(ref path_str) = vault_path {
        let p = Path::new(path_str);
        if p.exists() {
            let existing = load_mappings_auto(p, password.as_deref()).map_err(|e| e.to_string())?;
            engine.manager.load_mappings(existing).map_err(|e| e.to_string())?;
        }
    }

    // Escanear y registrar nuevos mapeos
    engine.scan_and_register_mappings(&text).map_err(|e| e.to_string())?;

    let format = parse_format(&format_str);
    let (obfuscated, report) = engine.obfuscate_text(&text, format).map_err(|e| e.to_string())?;
    let all_mappings = engine.manager.get_mappings();

    // Guardar en la bóveda si se especificó ruta
    if let Some(ref path_str) = vault_path {
        let p = Path::new(path_str);
        save_mappings_auto(p, &all_mappings, password.as_deref()).map_err(|e| e.to_string())?;
    }

    Ok(ObfuscateResponse {
        obfuscated_text: obfuscated,
        report,
        mappings: all_mappings,
    })
}

#[tauri::command]
fn deobfuscate_content(
    text: String,
    format_str: String,
    vault_path: Option<String>,
    password: Option<String>,
    custom_mappings: Option<Vec<Mapping>>,
) -> Result<DeobfuscateResponse, String> {
    let config = load_config_with_rules(None, None);
    let mut engine = ObfuscatorEngine::new(config);

    if let Some(mappings) = custom_mappings {
        engine.manager.load_mappings(mappings).map_err(|e| e.to_string())?;
    } else if let Some(ref path_str) = vault_path {
        let p = Path::new(path_str);
        if !p.exists() {
            return Err("El archivo de bóveda o mapeos no existe".to_string());
        }
        let loaded = load_mappings_auto(p, password.as_deref()).map_err(|e| e.to_string())?;
        engine.manager.load_mappings(loaded).map_err(|e| e.to_string())?;
    } else {
        return Err("Se requiere una bóveda o lista de mapeos para desofuscar".to_string());
    }

    let format = parse_format(&format_str);
    let (deobfuscated, report) = engine.deobfuscate_text(&text, format).map_err(|e| e.to_string())?;

    Ok(DeobfuscateResponse {
        deobfuscated_text: deobfuscated,
        report,
    })
}

#[tauri::command]
fn scan_content(
    text: String,
) -> Result<Vec<DetectedItem>, String> {
    let config = load_config_with_rules(None, None);
    let mut engine = ObfuscatorEngine::new(config);

    let detected = engine.scan_and_register_mappings(&text).map_err(|e| e.to_string())?;
    let items = detected.into_iter().map(|m| DetectedItem {
        original: m.original,
        pseudonym: m.pseudonym,
        detector_type: m.detector_type.to_string(),
        omitted: m.omitted,
    }).collect();

    Ok(items)
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DefaultVaultInfo {
    pub path: String,
    pub exists: bool,
    pub items: Vec<DetectedItem>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct VaultMetadata {
    pub path: String,
    pub exists: bool,
    pub is_encrypted: bool,
    pub is_unlocked: bool,
    pub items_count: usize,
    pub items: Vec<DetectedItem>,
}

#[tauri::command]
fn inspect_vault_file(path: String, password: Option<String>) -> Result<VaultMetadata, String> {
    let p = Path::new(&path);
    if !p.exists() {
        return Ok(VaultMetadata {
            path,
            exists: false,
            is_encrypted: false,
            is_unlocked: false,
            items_count: 0,
            items: Vec::new(),
        });
    }

    let bytes = std::fs::read(p).map_err(|e| e.to_string())?;
    let encrypted = lexishield::crypto::is_encrypted_vault(&bytes);

    if !encrypted {
        // Formato JSON sin contraseña
        let mappings: Vec<Mapping> = serde_json::from_slice(&bytes).unwrap_or_default();
        let items: Vec<DetectedItem> = mappings
            .into_iter()
            .map(|m| DetectedItem {
                original: m.original,
                pseudonym: m.pseudonym,
                detector_type: m.detector_type.to_string(),
                omitted: m.omitted,
            })
            .collect();
        let count = items.len();
        return Ok(VaultMetadata {
            path,
            exists: true,
            is_encrypted: false,
            is_unlocked: true,
            items_count: count,
            items,
        });
    }

    // Está cifrado (.lexi)
    if let Some(ref pwd) = password {
        match load_mappings_auto(p, Some(pwd)) {
            Ok(mappings) => {
                let items: Vec<DetectedItem> = mappings
                    .into_iter()
                    .map(|m| DetectedItem {
                        original: m.original,
                        pseudonym: m.pseudonym,
                        detector_type: m.detector_type.to_string(),
                        omitted: m.omitted,
                    })
                    .collect();
                let count = items.len();
                Ok(VaultMetadata {
                    path,
                    exists: true,
                    is_encrypted: true,
                    is_unlocked: true,
                    items_count: count,
                    items,
                })
            }
            Err(e) => Err(format!("Contraseña incorrecta o error al descifrar: {e}")),
        }
    } else {
        // Cifrado y sin contraseña suministrada aún
        Ok(VaultMetadata {
            path,
            exists: true,
            is_encrypted: true,
            is_unlocked: false,
            items_count: 0,
            items: Vec::new(),
        })
    }
}

#[tauri::command]
fn get_default_vault_info(password: Option<String>) -> Result<DefaultVaultInfo, String> {
    let p = lexishield::config::get_default_vault_path().map_err(|e| e.to_string())?;
    let path_str = p.to_string_lossy().to_string();
    let exists = p.exists();
    let mut items = Vec::new();

    if exists && let Ok(mappings) = load_mappings_auto(&p, password.as_deref()) {
        items = mappings
            .into_iter()
            .map(|m| DetectedItem {
                original: m.original,
                pseudonym: m.pseudonym,
                detector_type: m.detector_type.to_string(),
                omitted: m.omitted,
            })
            .collect();
    }

    Ok(DefaultVaultInfo {
        path: path_str,
        exists,
        items,
    })
}

#[tauri::command]
fn read_vault_file(path: String, password: Option<String>) -> Result<Vec<DetectedItem>, String> {
    let p = Path::new(&path);
    if !p.exists() {
        return Err("El archivo no existe".to_string());
    }
    let mappings = load_mappings_auto(p, password.as_deref()).map_err(|e| e.to_string())?;
    Ok(mappings.into_iter().map(|m| DetectedItem {
        original: m.original,
        pseudonym: m.pseudonym,
        detector_type: m.detector_type.to_string(),
        omitted: m.omitted,
    }).collect())
}

#[tauri::command]
fn save_vault_file(
    path: String,
    password: Option<String>,
    items: Vec<DetectedItem>,
) -> Result<String, String> {
    let mappings: Vec<Mapping> = items.into_iter().map(|it| {
        Mapping {
            original: it.original,
            pseudonym: it.pseudonym,
            detector_type: DetectorType::Custom(it.detector_type),
            omitted: it.omitted,
        }
    }).collect();

    let p = Path::new(&path);
    save_mappings_auto(p, &mappings, password.as_deref()).map_err(|e| e.to_string())?;
    Ok("Bóveda guardada correctamente".to_string())
}

#[tauri::command]
fn get_system_clipboard() -> Result<String, String> {
    get_clipboard_text().map_err(|e| e.to_string())
}

#[tauri::command]
fn set_system_clipboard(text: String) -> Result<(), String> {
    set_clipboard_text(&text).map_err(|e| e.to_string())
}

fn parse_format(f: &str) -> FormatType {
    match f.to_lowercase().as_str() {
        "json" => FormatType::Json,
        "xml" => FormatType::Xml,
        "log" => FormatType::Log,
        "plaintext" | "text" => FormatType::Plaintext,
        _ => FormatType::Auto,
    }
}

#[tauri::command]
fn process_file_command(
    input_path: String,
    output_path: String,
    vault_path: Option<String>,
    password: Option<String>,
    reverse: bool,
) -> Result<ObfuscationReport, String> {
    let config = load_config_with_rules(None, None);
    let mut engine = ObfuscatorEngine::new(config);

    if let Some(ref path_str) = vault_path {
        let p = Path::new(path_str);
        if p.exists() {
            let loaded = load_mappings_auto(p, password.as_deref()).map_err(|e| e.to_string())?;
            engine.manager.load_mappings(loaded).map_err(|e| e.to_string())?;
        }
    }

    if !reverse {
        engine.scan_file(Path::new(&input_path)).map_err(|e| e.to_string())?;
        if let Some(ref path_str) = vault_path {
            let p = Path::new(path_str);
            let all_mappings = engine.manager.get_mappings();
            save_mappings_auto(p, &all_mappings, password.as_deref()).map_err(|e| e.to_string())?;
        }
    }

    let report = engine
        .process_file(
            Path::new(&input_path),
            Path::new(&output_path),
            FormatType::Auto,
            reverse,
        )
        .map_err(|e| e.to_string())?;

    Ok(report)
}

#[tauri::command]
fn reset_config_command() -> Result<String, String> {
    let path = lexishield::config::reset_config().map_err(|e| e.to_string())?;
    Ok(format!("Configuración restablecida con éxito en: {}", path.display()))
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ScanAuditResponse {
    pub files_scanned: usize,
    pub files_ignored: usize,
    pub total_entities: usize,
    pub items: Vec<DetectedItem>,
    pub summary_by_type: std::collections::HashMap<String, usize>,
    pub path_analyzed: String,
}

#[tauri::command]
fn scan_file_command(path: String) -> Result<ScanAuditResponse, String> {
    let p = Path::new(&path);
    if !p.exists() {
        return Err("El archivo no existe".to_string());
    }

    let config = load_config_with_rules(None, None);
    let mut engine = ObfuscatorEngine::new(config);

    let detected = engine.scan_file(p).map_err(|e| e.to_string())?;
    
    let mut summary = std::collections::HashMap::new();
    for m in &detected {
        *summary.entry(m.detector_type.to_string()).or_insert(0) += 1;
    }

    let items: Vec<DetectedItem> = detected.into_iter().map(|m| DetectedItem {
        original: m.original,
        pseudonym: m.pseudonym,
        detector_type: m.detector_type.to_string(),
        omitted: m.omitted,
    }).collect();

    let total = items.len();

    Ok(ScanAuditResponse {
        files_scanned: 1,
        files_ignored: 0,
        total_entities: total,
        items,
        summary_by_type: summary,
        path_analyzed: path,
    })
}

#[tauri::command]
fn cancel_scan_command() -> bool {
    SCAN_CANCELLED.store(true, Ordering::Relaxed);
    log::info!("Cancelación de escaneo solicitada por el usuario");
    true
}

#[tauri::command]
fn get_scan_progress() -> CurrentScanStatus {
    get_current_scan_status().lock().unwrap().clone()
}

#[tauri::command]
async fn scan_directory_command(
    app: tauri::AppHandle,
    dir_path: String,
    recursive: Option<bool>,
    no_git_ignore: Option<bool>,
) -> Result<ScanAuditResponse, String> {
    use ignore::WalkBuilder;
    let base_dir_buf = std::path::PathBuf::from(&dir_path);
    if !base_dir_buf.exists() || !base_dir_buf.is_dir() {
        return Err("El directorio no existe o no es válido".to_string());
    }

    let is_recursive = recursive.unwrap_or(true);
    let git_ignore_active = !no_git_ignore.unwrap_or(false);

    log::info!(
        "Iniciando escaneo asíncrono de directorio: {} (recursivo: {}, git_ignore: {})",
        dir_path,
        is_recursive,
        git_ignore_active
    );
    SCAN_CANCELLED.store(false, Ordering::Relaxed);

    {
        let mut status = get_current_scan_status().lock().unwrap();
        status.is_scanning = true;
        status.current_file = "Iniciando recorrido...".to_string();
        status.files_scanned = 0;
        status.entities_found = 0;
    }

    tauri::async_runtime::spawn_blocking(move || {
        let config = load_config_with_rules(None, None);
        let ignore_cfg = config.ignore.clone();
        let mut engine = ObfuscatorEngine::new(config);

        let ignored_dirs = ignore_cfg.ignored_directories.clone();
        let mut builder = WalkBuilder::new(&base_dir_buf);
        builder.add_custom_ignore_filename(".lexiignore");
        builder.hidden(true);
        builder.git_ignore(git_ignore_active);
        builder.git_global(git_ignore_active);
        builder.git_exclude(git_ignore_active);

        if !is_recursive {
            builder.max_depth(Some(1));
        }

        // Pruning eficiente: No descender en directorios ignorados como target, node_modules, dist, .git, etc.
        builder.filter_entry(move |entry| {
            if entry.file_type().is_some_and(|ft| ft.is_dir())
                && let Some(name) = entry.file_name().to_str()
                && ignored_dirs.iter().any(|d| d.eq_ignore_ascii_case(name))
            {
                return false;
            }
            true
        });

        let mut files_scanned = 0;
        let mut files_ignored = 0;
        let mut last_emit = std::time::Instant::now();

        for result in builder.build() {
            if SCAN_CANCELLED.load(Ordering::Relaxed) {
                log::warn!("Escaneo cancelado por el usuario.");
                break;
            }

            if let Ok(entry) = result {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension().and_then(|e| e.to_str())
                        && ignore_cfg
                            .ignored_extensions
                            .iter()
                            .any(|e| e.eq_ignore_ascii_case(ext))
                    {
                        files_ignored += 1;
                        continue;
                    }

                    let rel_path = path.strip_prefix(&base_dir_buf).unwrap_or(path);
                    let path_str = rel_path.display().to_string();

                    // Actualizar estado compartido y registrar traza ANTES de analizar el fichero
                    if let Ok(mut status) = get_current_scan_status().lock() {
                        status.current_file = path_str.clone();
                        status.files_scanned = files_scanned;
                        status.entities_found = engine.manager.get_mappings().len();
                    }

                    if last_emit.elapsed().as_millis() >= 30 {
                        let _ = app.emit(
                            "scan-progress",
                            ScanProgressPayload {
                                current_file: path_str.clone(),
                                files_scanned,
                                entities_found: engine.manager.get_mappings().len(),
                            },
                        );
                        last_emit = std::time::Instant::now();
                    }

                    if engine.scan_file(path).is_ok() {
                        files_scanned += 1;
                        let entities_now = engine.manager.get_mappings().len();
                        log::info!("Analizado: {} (Total entidades: {})", path_str, entities_now);
                        if let Ok(mut status) = get_current_scan_status().lock() {
                            status.files_scanned = files_scanned;
                            status.entities_found = entities_now;
                        }
                    } else {
                        files_ignored += 1;
                    }
                }
            }
        }

        let is_cancelled = SCAN_CANCELLED.load(Ordering::Relaxed);
        let final_status_msg = if is_cancelled {
            "Escaneo cancelado".to_string()
        } else {
            "Escaneo completado".to_string()
        };

        let entities_found = engine.manager.get_mappings().len();

        if let Ok(mut status) = get_current_scan_status().lock() {
            status.is_scanning = false;
            status.current_file = final_status_msg.clone();
            status.files_scanned = files_scanned;
            status.entities_found = entities_found;
        }

        let _ = app.emit(
            "scan-progress",
            ScanProgressPayload {
                current_file: final_status_msg,
                files_scanned,
                entities_found,
            },
        );

        let all_mappings = engine.manager.get_mappings();
        let mut summary = std::collections::HashMap::new();
        for m in &all_mappings {
            *summary.entry(m.detector_type.to_string()).or_insert(0) += 1;
        }

        let items: Vec<DetectedItem> = all_mappings
            .into_iter()
            .map(|m| DetectedItem {
                original: m.original,
                pseudonym: m.pseudonym,
                detector_type: m.detector_type.to_string(),
                omitted: m.omitted,
            })
            .collect();

        let total = items.len();
        log::info!(
            "Escaneo finalizado (cancelado={}). Analizados: {}, Ignorados: {}, Entidades: {}",
            is_cancelled,
            files_scanned,
            files_ignored,
            total
        );

        Ok(ScanAuditResponse {
            files_scanned,
            files_ignored,
            total_entities: total,
            items,
            summary_by_type: summary,
            path_analyzed: dir_path,
        })
    })
    .await
    .map_err(|e| format!("Error en tarea de escaneo asíncrona: {e}"))?
}

// -----------------------------------------------------------------------------
// Comandos de Gestión de Bóvedas .lexi
// -----------------------------------------------------------------------------

#[tauri::command]
fn list_all_vaults() -> Result<Vec<lexishield::models::VaultFileInfo>, String> {
    lexishield::vault::list_vaults().map_err(|e| e.to_string())
}

#[tauri::command]
fn create_vault_file(name: String, password: Option<String>) -> Result<String, String> {
    let path = lexishield::vault::create_vault(&name, password.as_deref()).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
fn rename_vault_file(old_name: String, new_name: String) -> Result<String, String> {
    let path = lexishield::vault::rename_vault(&old_name, &new_name).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
fn delete_vault_file(name: String) -> Result<(), String> {
    lexishield::vault::delete_vault(&name).map_err(|e| e.to_string())
}

#[tauri::command]
fn export_vault_file(name: String, dest_path: String) -> Result<(), String> {
    lexishield::vault::export_vault(&name, Path::new(&dest_path)).map_err(|e| e.to_string())
}

#[tauri::command]
fn import_vault_file(src_path: String, name: Option<String>) -> Result<String, String> {
    let path = lexishield::vault::import_vault(Path::new(&src_path), name.as_deref()).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
fn restore_vault_backup_file(name: String) -> Result<String, String> {
    let path = lexishield::vault::restore_vault_backup(&name).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
fn has_vault_backup_file(name: String) -> Result<bool, String> {
    lexishield::vault::has_vault_backup(&name).map_err(|e| e.to_string())
}

// -----------------------------------------------------------------------------
// Comandos de Copias de Seguridad Bzip2
// -----------------------------------------------------------------------------

#[tauri::command]
fn create_backup_file(output_path: Option<String>, password: Option<String>) -> Result<String, String> {
    let out_path = if let Some(p) = output_path {
        std::path::PathBuf::from(p)
    } else {
        let config_dir = lexishield::config::get_user_config_dir().map_err(|e| e.to_string())?;
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        config_dir.join(format!("backup_lexishield_{ts}.tar.bz2"))
    };

    let res = lexishield::backup::create_backup(&out_path, password.as_deref()).map_err(|e| e.to_string())?;
    Ok(res.to_string_lossy().to_string())
}

#[tauri::command]
fn restore_backup_file(input_path: String, password: Option<String>) -> Result<usize, String> {
    lexishield::backup::restore_backup(Path::new(&input_path), password.as_deref()).map_err(|e| e.to_string())
}

// -----------------------------------------------------------------------------
// Comandos de Gestión de Pares / Mapeos Individuales y en Lote
// -----------------------------------------------------------------------------

#[tauri::command]
fn update_mapping_pair(
    vault_path: String,
    password: Option<String>,
    original: String,
    pseudonym: String,
    detector_type: Option<String>,
) -> Result<Vec<DetectedItem>, String> {
    let p = Path::new(&vault_path);
    let mut mappings = if p.exists() {
        load_mappings_auto(p, password.as_deref()).map_err(|e| e.to_string())?
    } else {
        Vec::new()
    };

    let omitted = pseudonym == "=";
    let det_type = detector_type
        .map(DetectorType::Custom)
        .unwrap_or(DetectorType::GenericToken);

    let mut found = false;
    for m in &mut mappings {
        if m.original == original {
            m.pseudonym = pseudonym.clone();
            m.omitted = omitted;
            m.detector_type = det_type.clone();
            found = true;
            break;
        }
    }
    if !found {
        mappings.push(Mapping {
            original,
            pseudonym,
            detector_type: det_type,
            omitted,
        });
    }

    save_mappings_auto(p, &mappings, password.as_deref()).map_err(|e| e.to_string())?;

    Ok(mappings
        .into_iter()
        .map(|m| DetectedItem {
            original: m.original,
            pseudonym: m.pseudonym,
            detector_type: m.detector_type.to_string(),
            omitted: m.omitted,
        })
        .collect())
}

#[tauri::command]
fn delete_mapping_pairs(
    vault_path: String,
    password: Option<String>,
    originals: Vec<String>,
) -> Result<Vec<DetectedItem>, String> {
    let p = Path::new(&vault_path);
    if !p.exists() {
        return Err("El archivo de bóveda no existe".to_string());
    }

    let mut mappings = load_mappings_auto(p, password.as_deref()).map_err(|e| e.to_string())?;
    mappings.retain(|m| !originals.contains(&m.original));
    save_mappings_auto(p, &mappings, password.as_deref()).map_err(|e| e.to_string())?;

    Ok(mappings
        .into_iter()
        .map(|m| DetectedItem {
            original: m.original,
            pseudonym: m.pseudonym,
            detector_type: m.detector_type.to_string(),
            omitted: m.omitted,
        })
        .collect())
}

// -----------------------------------------------------------------------------
// Comandos de Portapapeles y Configuración
// -----------------------------------------------------------------------------

#[tauri::command]
fn clear_system_clipboard() -> Result<(), String> {
    lexishield::clipboard::clear_clipboard()
}

#[tauri::command]
fn get_app_settings() -> Result<lexishield::config::LexiConfig, String> {
    Ok(lexishield::config::load_config_with_rules(None, None))
}

#[tauri::command]
fn save_app_settings(config: lexishield::config::LexiConfig) -> Result<(), String> {
    lexishield::config::save_user_config(&config).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_debug_mode() -> bool {
    is_debug_mode()
}

#[tauri::command]
fn set_debug_mode_command(enabled: bool) -> bool {
    set_debug_mode(enabled);
    enabled
}

#[tauri::command]
fn get_debug_logs(limit: Option<usize>) -> Vec<LogEntry> {
    get_recent_logs(limit)
}

#[tauri::command]
fn clear_debug_logs() -> Result<(), String> {
    clear_logs();
    Ok(())
}

fn main() {
    init_logger();
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            obfuscate_content,
            deobfuscate_content,
            scan_content,
            scan_file_command,
            scan_directory_command,
            cancel_scan_command,
            get_scan_progress,
            get_default_vault_info,
            inspect_vault_file,
            read_vault_file,
            save_vault_file,
            list_all_vaults,
            create_vault_file,
            rename_vault_file,
            delete_vault_file,
            export_vault_file,
            import_vault_file,
            restore_vault_backup_file,
            has_vault_backup_file,
            create_backup_file,
            restore_backup_file,
            update_mapping_pair,
            delete_mapping_pairs,
            get_system_clipboard,
            set_system_clipboard,
            clear_system_clipboard,
            get_app_settings,
            save_app_settings,
            process_file_command,
            reset_config_command,
            get_debug_mode,
            set_debug_mode_command,
            get_debug_logs,
            clear_debug_logs
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

