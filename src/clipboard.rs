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
    let err = match Clipboard::new() {
        Ok(mut clipboard) => match clipboard.get_text() {
            Ok(text) => return Ok(text),
            Err(e) => format!("Error al leer texto del portapapeles: {e}"),
        },
        Err(e) => format!("Error al inicializar el portapapeles: {e}"),
    };

    #[cfg(target_os = "linux")]
    {
        get_clipboard_text_linux_fallback(&err)
    }
    #[cfg(not(target_os = "linux"))]
    {
        Err(err)
    }
}

/// Escribe texto en el portapapeles del sistema.
pub fn set_clipboard_text(text: &str) -> Result<(), String> {
    let err = match Clipboard::new() {
        Ok(mut clipboard) => match clipboard.set_text(text.to_string()) {
            Ok(()) => return Ok(()),
            Err(e) => format!("Error al escribir texto en el portapapeles: {e}"),
        },
        Err(e) => format!("Error al inicializar el portapapeles: {e}"),
    };

    #[cfg(target_os = "linux")]
    {
        set_clipboard_text_linux_fallback(text, &err)
    }
    #[cfg(not(target_os = "linux"))]
    {
        Err(err)
    }
}

/// Vacía de forma explícita el contenido del portapapeles del sistema.
pub fn clear_clipboard() -> Result<(), String> {
    set_clipboard_text("")
}

#[cfg(target_os = "linux")]
fn get_clipboard_text_linux_fallback(arboard_err: &str) -> Result<String, String> {
    // 1. Wayland: wl-paste (herramienta estándar para entornos Wayland)
    if let Ok(output) = std::process::Command::new("wl-paste")
        .arg("--no-newline")
        .output()
        && output.status.success()
    {
        return String::from_utf8(output.stdout)
            .map_err(|e| format!("Error de codificación UTF-8 en wl-paste: {e}"));
    }

    // 2. KDE Klipper vía D-Bus (qdbus6 o qdbus)
    for qdbus_bin in &["qdbus6", "qdbus"] {
        if let Ok(output) = std::process::Command::new(qdbus_bin)
            .args(["org.kde.klipper", "/klipper", "getClipboardContents"])
            .output()
            && output.status.success()
        {
            let text = String::from_utf8_lossy(&output.stdout)
                .trim_end_matches('\n')
                .to_string();
            return Ok(text);
        }
    }

    // 3. X11: xclip
    if let Ok(output) = std::process::Command::new("xclip")
        .args(["-selection", "clipboard", "-o"])
        .output()
        && output.status.success()
    {
        return String::from_utf8(output.stdout)
            .map_err(|e| format!("Error de codificación UTF-8 en xclip: {e}"));
    }

    // 4. X11: xsel
    if let Ok(output) = std::process::Command::new("xsel")
        .args(["-b", "-o"])
        .output()
        && output.status.success()
    {
        return String::from_utf8(output.stdout)
            .map_err(|e| format!("Error de codificación UTF-8 en xsel: {e}"));
    }

    Err(format!(
        "No se pudo acceder al portapapeles en Linux ({arboard_err}). En Wayland instala 'wl-clipboard' (sudo apt install wl-clipboard) o activa Klipper; en X11 instala 'xclip'."
    ))
}

#[cfg(target_os = "linux")]
fn set_clipboard_text_linux_fallback(text: &str, arboard_err: &str) -> Result<(), String> {
    use std::io::Write;

    // 1. Wayland: wl-copy
    if let Ok(mut child) = std::process::Command::new("wl-copy")
        .stdin(std::process::Stdio::piped())
        .spawn()
    {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(text.as_bytes());
        }
        if let Ok(status) = child.wait()
            && status.success()
        {
            return Ok(());
        }
    }

    // 2. KDE Klipper vía D-Bus (qdbus6 o qdbus)
    for qdbus_bin in &["qdbus6", "qdbus"] {
        if let Ok(status) = std::process::Command::new(qdbus_bin)
            .args(["org.kde.klipper", "/klipper", "setClipboardContents", text])
            .status()
            && status.success()
        {
            return Ok(());
        }
    }

    // 3. X11: xclip
    if let Ok(mut child) = std::process::Command::new("xclip")
        .args(["-selection", "clipboard"])
        .stdin(std::process::Stdio::piped())
        .spawn()
    {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(text.as_bytes());
        }
        if let Ok(status) = child.wait()
            && status.success()
        {
            return Ok(());
        }
    }

    // 4. X11: xsel
    if let Ok(mut child) = std::process::Command::new("xsel")
        .args(["-b", "-i"])
        .stdin(std::process::Stdio::piped())
        .spawn()
    {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(text.as_bytes());
        }
        if let Ok(status) = child.wait()
            && status.success()
        {
            return Ok(());
        }
    }

    Err(format!(
        "No se pudo escribir en el portapapeles en Linux ({arboard_err}). En Wayland instala 'wl-clipboard' (sudo apt install wl-clipboard); en X11 instala 'xclip'."
    ))
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

    let mut clipboard = match Clipboard::new() {
        Ok(cb) => Some(cb),
        Err(e) => {
            log::warn!(
                "No se pudo inicializar arboard directamente ({e}); usando respaldo del sistema."
            );
            None
        }
    };

    while running.load(Ordering::SeqCst) {
        thread::sleep(Duration::from_millis(250));

        let read_result = match &mut clipboard {
            Some(cb) => cb.get_text().or_else(|_| get_clipboard_text()),
            None => get_clipboard_text(),
        };

        if let Ok(current_text) = read_result {
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
                                let _ = match &mut clipboard {
                                    Some(cb) => cb
                                        .set_text(obfuscated.clone())
                                        .or_else(|_| set_clipboard_text(&obfuscated)),
                                    None => set_clipboard_text(&obfuscated),
                                };

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
                                let _ = match &mut clipboard {
                                    Some(cb) => cb
                                        .set_text(deobfuscated.clone())
                                        .or_else(|_| set_clipboard_text(&deobfuscated)),
                                    None => set_clipboard_text(&deobfuscated),
                                };
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
