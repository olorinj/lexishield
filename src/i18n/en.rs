//! English messages catalog for LexiShield.

use super::MsgKey;

pub fn translate(key: MsgKey) -> &'static str {
    match key {
        MsgKey::CliAbout => {
            "High-performance semantic protection and anonymization engine for AI queries and data sanitization"
        }
        MsgKey::VerboseDesc => {
            "Enable Debug log level for additional details (Telemetry on stderr)"
        }
        MsgKey::DebugDesc => "Enable full DEBUG mode with exhaustive telemetry on stderr",
        MsgKey::ConfigPathDesc => "Path to a custom TOML configuration file",
        MsgKey::RulesPathDesc => "Path to a custom TOML rules file (~/.lexishield/rules.toml)",
        MsgKey::LangDesc => "Display language ('en' for English, 'es' for Spanish)",
        MsgKey::EngineInitialized => "LexiShield core engine initialized successfully",
        MsgKey::ScanningStarted => "Scanning target files...",
        MsgKey::FilesProcessed => "Files processed",
        MsgKey::EntitiesFound => "Entities detected and protected",
        MsgKey::SavedAndEncryptedMappings => "Mappings saved and ENCRYPTED at:",
        MsgKey::DeobfuscationSuccess => "De-obfuscation completed successfully",
        MsgKey::CorruptedFileError => "File is corrupted or has an invalid structure",
        MsgKey::ClipboardWatching => "Watching clipboard in real-time...",
        MsgKey::ClipboardReplaced => "Clipboard content sanitized and updated",
        MsgKey::ConfigResetSuccess => "Default configuration restored successfully",
    }
}
