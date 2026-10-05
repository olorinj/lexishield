#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use lexishield::clipboard::{get_clipboard_text, set_clipboard_text};
use lexishield::config::load_config_with_rules;
use lexishield::crypto::{load_mappings_auto, save_mappings_auto};
use lexishield::engine::ObfuscatorEngine;
use lexishield::models::{DetectorType, FormatType, Mapping, ObfuscationReport};
use serde::{Deserialize, Serialize};
use std::path::Path;

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

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            obfuscate_content,
            deobfuscate_content,
            scan_content,
            read_vault_file,
            save_vault_file,
            get_system_clipboard,
            set_system_clipboard,
            process_file_command
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

