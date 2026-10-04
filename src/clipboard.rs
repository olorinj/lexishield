//! Gestión y monitorización en tiempo real del portapapeles con arboard y ctrlc.

use crate::config::LexiConfig;
use crate::crypto::{load_mappings_auto, save_mappings_auto};
use crate::engine::ObfuscatorEngine;
use crate::models::FormatType;
use arboard::Clipboard;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

/// Obtiene el texto actual del portapapeles del sistema.
pub fn get_clipboard_text() -> Result<String, String> {
    let mut clipboard =
        Clipboard::new().map_err(|e| format!("Error al inicializar el portapapeles: {e}"))?;
    clipboard
        .get_text()
        .map_err(|e| format!("Error al leer texto del portapapeles: {e}"))
}

/// Escribe texto en el portapapeles del sistema.
pub fn set_clipboard_text(text: &str) -> Result<(), String> {
    let mut clipboard =
        Clipboard::new().map_err(|e| format!("Error al inicializar el portapapeles: {e}"))?;
    clipboard
        .set_text(text.to_string())
        .map_err(|e| format!("Error al escribir texto en el portapapeles: {e}"))
}

/// Dirección de monitorización para el comando watch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchDirection {
    Obfuscate,
    Deobfuscate,
}

/// Bucle de monitorización continua del portapapeles con supresión de eco y soporte cifrado.
pub fn watch_clipboard_loop(
    direction: WatchDirection,
    format: FormatType,
    config: LexiConfig,
    mappings_file: Option<&Path>,
    password: Option<&str>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = ObfuscatorEngine::new(config);

    if let Some(map_path) = mappings_file
        && map_path.exists()
    {
        let mappings = load_mappings_auto(map_path, password)?;
        engine.manager.load_mappings(mappings)?;
        log::info!(
            "Se cargaron {} mapeos previos desde: {}",
            engine.manager.count(),
            map_path.display()
        );
    }

    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    let _ = ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    });

    log::info!(
        "Iniciando monitorización de portapapeles [Modo: {direction:?}]. Presiona Ctrl+C para detener."
    );

    let mut last_seen = get_clipboard_text().unwrap_or_default();

    let mut clipboard =
        Clipboard::new().map_err(|e| format!("Error al inicializar acceso a portapapeles: {e}"))?;

    while running.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(250));

        if let Ok(current_text) = clipboard.get_text() {
            let trimmed = current_text.trim();
            if !trimmed.is_empty() && current_text != last_seen {
                match direction {
                    WatchDirection::Obfuscate => {
                        let _ = engine.scan_and_register_mappings(&current_text);
                        if let Ok((obfuscated, report)) =
                            engine.obfuscate_text(&current_text, format)
                        {
                            if obfuscated == current_text {
                                last_seen = current_text;
                            } else {
                                log::info!(
                                    "Portapapeles ofuscado automáticamente ({} reemplazos aplicados).",
                                    report.replacements_applied
                                );
                                last_seen.clone_from(&obfuscated);
                                let _ = clipboard.set_text(obfuscated);

                                if let Some(map_path) = mappings_file {
                                    let current_maps = engine.manager.get_mappings();
                                    if let Err(e) =
                                        save_mappings_auto(map_path, &current_maps, password)
                                    {
                                        log::error!("Error al guardar mapeos: {e}");
                                    }
                                }
                            }
                        }
                    }
                    WatchDirection::Deobfuscate => {
                        if let Ok((deobfuscated, report)) =
                            engine.deobfuscate_text(&current_text, format)
                        {
                            if deobfuscated == current_text {
                                last_seen = current_text;
                            } else {
                                log::info!(
                                    "Portapapeles desofuscado automáticamente ({} reemplazos restaurados).",
                                    report.replacements_applied
                                );
                                last_seen.clone_from(&deobfuscated);
                                let _ = clipboard.set_text(deobfuscated);
                            }
                        }
                    }
                }
            }
        }
    }

    log::info!("Monitorización del portapapeles detenida correctamente.");
    Ok(())
}
