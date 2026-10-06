//! Módulo de inicialización de logging centralizado para LexiShield.

use log::{Level, LevelFilter, Metadata, Record};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::fs::OpenOptions;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};

/// Capacidad máxima del ring buffer en memoria para logs de la interfaz gráfica.
const MAX_LOG_BUFFER_ENTRIES: usize = 1000;

/// Entrada de registro estructurada para telemetría y UI.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub message: String,
}

static LOGGER: OnceLock<LexiShieldLogger> = OnceLock::new();

struct LexiShieldLogger {
    debug_mode: AtomicBool,
    buffer: Mutex<VecDeque<LogEntry>>,
}

impl LexiShieldLogger {
    fn new(initial_debug: bool) -> Self {
        Self {
            debug_mode: AtomicBool::new(initial_debug),
            buffer: Mutex::new(VecDeque::with_capacity(MAX_LOG_BUFFER_ENTRIES)),
        }
    }

    fn push_entry(&self, entry: LogEntry) {
        if let Ok(mut buf) = self.buffer.lock() {
            if buf.len() >= MAX_LOG_BUFFER_ENTRIES {
                buf.pop_front();
            }
            buf.push_back(entry);
        }
    }
}

fn format_local_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let total_secs = duration.as_secs();
    let millis = duration.subsec_millis();
    let secs = total_secs % 60;
    let mins = (total_secs / 60) % 60;
    let hours = (total_secs / 3600) % 24;
    format!("{hours:02}:{mins:02}:{secs:02}.{millis:03}")
}

impl log::Log for LexiShieldLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        let is_debug = self.debug_mode.load(Ordering::Relaxed);
        if is_debug {
            metadata.level() <= Level::Debug
        } else {
            metadata.level() <= Level::Info
        }
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        // Excluir ruido de bibliotecas externas a nivel trace/debug si no provienen de lexishield
        let target = record.target();
        let is_debug = self.debug_mode.load(Ordering::Relaxed);
        if is_debug && record.level() == Level::Debug && !target.starts_with("lexishield") {
            return;
        }

        let now = format_local_timestamp();
        let level_str = record.level().as_str();
        let message = format!("{}", record.args());

        // 1. Escribir a stderr respetando los principios de CLI pura de Unix
        let color_code = match record.level() {
            Level::Error => "\x1b[31m", // Rojo
            Level::Warn => "\x1b[33m",  // Amarillo
            Level::Info => "\x1b[32m",  // Verde
            Level::Debug => "\x1b[36m", // Cian
            Level::Trace => "\x1b[90m", // Gris
        };
        let reset_code = "\x1b[0m";

        eprintln!("{color_code}[{now} {level_str:<5} {target}]{reset_code} {message}");

        // 2. Almacenar en el buffer en memoria para la GUI
        let entry = LogEntry {
            timestamp: now.clone(),
            level: level_str.to_string(),
            target: target.to_string(),
            message: message.clone(),
        };
        self.push_entry(entry);

        // 3. Si el modo debug está activo, escribir opcionalmente al archivo ~/.lexishield/debug.log
        if is_debug && let Some(mut log_file_path) = dirs::home_dir() {
            log_file_path.push(".lexishield");
            let _ = std::fs::create_dir_all(&log_file_path);
            log_file_path.push("debug.log");
            if let Ok(mut file) = OpenOptions::new()
                .create(true)
                .append(true)
                .open(log_file_path)
            {
                let _ = writeln!(file, "[{now} {level_str:<5} {target}] {message}");
            }
        }
    }

    fn flush(&self) {
        let _ = std::io::stderr().flush();
    }
}

/// Inicializa el logger centralizado de la aplicación.
pub fn init_logger() {
    let initial_debug = std::env::var("RUST_LOG").is_ok_and(|v| v.to_lowercase().contains("debug"))
        || std::env::var("LEXISHIELD_DEBUG").is_ok();

    let logger = LOGGER.get_or_init(|| LexiShieldLogger::new(initial_debug));
    let max_level = if initial_debug {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };
    let _ = log::set_logger(logger);
    log::set_max_level(max_level);
}

/// Inicializa el logger con un nivel específico (útil para pruebas y modo debug explícito).
pub fn init_logger_with_level(level: LevelFilter) {
    init_logger();
    set_debug_mode(level >= LevelFilter::Debug);
}

/// Activa o desactiva dinámicamente el modo DEBUG en tiempo de ejecución.
pub fn set_debug_mode(enabled: bool) {
    if let Some(logger) = LOGGER.get() {
        logger.debug_mode.store(enabled, Ordering::Relaxed);
        if enabled {
            log::set_max_level(LevelFilter::Debug);
            log::debug!("Modo DEBUG activado dinámicamente.");
        } else {
            log::set_max_level(LevelFilter::Info);
            log::info!("Modo DEBUG desactivado.");
        }
    }
}

/// Consulta si el modo DEBUG está actualmente activo.
pub fn is_debug_mode() -> bool {
    LOGGER
        .get()
        .is_some_and(|l| l.debug_mode.load(Ordering::Relaxed))
}

/// Recupera los registros recientes almacenados en memoria para visualizarlos en la GUI.
pub fn get_recent_logs(limit: Option<usize>) -> Vec<LogEntry> {
    LOGGER
        .get()
        .and_then(|logger| logger.buffer.lock().ok())
        .map(|buf| {
            let count = limit.unwrap_or(buf.len()).min(buf.len());
            buf.iter().rev().take(count).rev().cloned().collect()
        })
        .unwrap_or_default()
}

/// Limpia el buffer de registros en memoria.
pub fn clear_logs() {
    if let Some(mut buf) = LOGGER.get().and_then(|l| l.buffer.lock().ok()) {
        buf.clear();
    }
}
