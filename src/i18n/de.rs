//! Deutsche Meldungen für LexiShield.

use super::MsgKey;

pub fn translate(key: MsgKey) -> &'static str {
    match key {
        MsgKey::CliAbout => {
            "Hochleistungs-Engine für Anonymisierung und semantischen Schutz für KI-Abfragen und Datenbereinigung"
        }
        MsgKey::VerboseDesc => {
            "Debug-Protokollierung für zusätzliche Details aktivieren (Telemetrie auf stderr)"
        }
        MsgKey::DebugDesc => {
            "Vollständigen DEBUG-Modus mit umfassender Telemetrie auf stderr aktivieren"
        }
        MsgKey::ConfigPathDesc => "Pfad zu einer benutzerdefinierten TOML-Konfigurationsdatei",
        MsgKey::RulesPathDesc => {
            "Pfad zu einer benutzerdefinierten TOML-Regeldatei (~/.lexishield/rules.toml)"
        }
        MsgKey::LangDesc => "Anzeigesprache ('en', 'es', 'fr', 'de', 'pt', 'it', 'zh')",
        MsgKey::EngineInitialized => "LexiShield Core-Engine erfolgreich initialisiert",
        MsgKey::ScanningStarted => "Scannen der Zieldateien gestartet...",
        MsgKey::FilesProcessed => "Dateien verarbeitet",
        MsgKey::EntitiesFound => "Entitäten erkannt und geschützt",
        MsgKey::SavedAndEncryptedMappings => "Zuordnungen gespeichert und VERSCHLÜSSELT unter:",
        MsgKey::DeobfuscationSuccess => "Deanonymisierung erfolgreich abgeschlossen",
        MsgKey::CorruptedFileError => "Datei ist beschädigt oder weist eine ungültige Struktur auf",
        MsgKey::ClipboardWatching => "Zwischenablage wird in Echtzeit überwacht...",
        MsgKey::ClipboardReplaced => "Inhalt der Zwischenablage bereinigt und aktualisiert",
        MsgKey::ConfigResetSuccess => "Standardkonfiguration erfolgreich wiederhergestellt",
    }
}
