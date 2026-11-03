//! Catálogo de mensajes en español para LexiShield.

use super::MsgKey;

pub fn translate(key: MsgKey) -> &'static str {
    match key {
        MsgKey::CliAbout => {
            "Motor de anonimización y protección semántica de alto rendimiento para consultas de IA y sanitización de datos"
        }
        MsgKey::VerboseDesc => {
            "Activa el nivel de log Debug para más detalles (Telemetría en stderr)"
        }
        MsgKey::DebugDesc => {
            "Activa el modo de depuración detallada (DEBUG) con telemetría completa en stderr"
        }
        MsgKey::ConfigPathDesc => "Ruta a un archivo de configuración TOML personalizado",
        MsgKey::RulesPathDesc => {
            "Ruta a un archivo de reglas TOML personalizado (~/.lexishield/rules.toml)"
        }
        MsgKey::LangDesc => "Idioma de visualización ('en' para inglés, 'es' para español)",
        MsgKey::EngineInitialized => "Motor central de LexiShield inicializado correctamente",
        MsgKey::ScanningStarted => "Iniciando escaneo de archivos objetivo...",
        MsgKey::FilesProcessed => "Archivos procesados",
        MsgKey::EntitiesFound => "Entidades detectadas y protegidas",
        MsgKey::SavedAndEncryptedMappings => "Mapeos guardados y CIFRADOS en:",
        MsgKey::DeobfuscationSuccess => "Desanonimización completada exitosamente",
        MsgKey::CorruptedFileError => "El archivo está corrupto o posee una estructura inválida",
        MsgKey::ClipboardWatching => "Supervisando el portapapeles en tiempo real...",
        MsgKey::ClipboardReplaced => "Contenido del portapapeles sanitizado y actualizado",
        MsgKey::ConfigResetSuccess => "Configuración predeterminada restaurada correctamente",
    }
}
