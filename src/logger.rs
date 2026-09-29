//! Módulo de inicialización de logging centralizado para LexiShield.

use log::LevelFilter;
use std::sync::Once;

static INIT_LOGGER: Once = Once::new();

/// Inicializa el logger centralizado de la aplicación.
///
/// Configura el nivel de log mediante la variable de entorno `RUST_LOG` o por defecto en `INFO`.
pub fn init_logger() {
    INIT_LOGGER.call_once(|| {
        let env = env_logger::Env::default().default_filter_or("info");
        env_logger::Builder::from_env(env)
            .format_timestamp_millis()
            .init();
        log::info!("Logger centralizado inicializado con éxito.");
    });
}

/// Inicializa el logger con un nivel específico (útil para pruebas y modo debug).
pub fn init_logger_with_level(level: LevelFilter) {
    INIT_LOGGER.call_once(|| {
        env_logger::Builder::new()
            .filter_level(level)
            .format_timestamp_millis()
            .init();
        log::info!("Logger centralizado inicializado en nivel {:?}", level);
    });
}

