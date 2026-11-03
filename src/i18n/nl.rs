//! Nederlandse berichten voor LexiShield.

use super::MsgKey;

pub fn translate(key: MsgKey) -> &'static str {
    match key {
        MsgKey::CliAbout => {
            "Krachtige engine voor semantische bescherming en anonimisering voor AI-query's en gegevensopschoning"
        }
        MsgKey::VerboseDesc => {
            "Schakel Debug-logniveau in voor aanvullende details (telemetrie op stderr)"
        }
        MsgKey::DebugDesc => {
            "Volledige DEBUG-modus inschakelen met uitputtende telemetrie op stderr"
        }
        MsgKey::ConfigPathDesc => "Pad naar een aangepast TOML-configuratiebestand",
        MsgKey::RulesPathDesc => {
            "Pad naar een aangepast TOML-regelbestand (~/.lexishield/rules.toml)"
        }
        MsgKey::LangDesc => {
            "Weergavetaal ('en', 'es', 'fr', 'de', 'pt', 'it', 'zh', 'ja', 'ru', 'pl', 'nl', 'ko')"
        }
        MsgKey::EngineInitialized => "LexiShield core engine succesvol geïnitialiseerd",
        MsgKey::ScanningStarted => "Scannen van doelbestanden starten...",
        MsgKey::FilesProcessed => "Verwerkte bestanden",
        MsgKey::EntitiesFound => "Gedetecteerde en beveiligde entiteiten",
        MsgKey::SavedAndEncryptedMappings => "Toewijzingen opgeslagen en VERSLEUTELD op:",
        MsgKey::DeobfuscationSuccess => "De-anonimisering succesvol voltooid",
        MsgKey::CorruptedFileError => "Bestand is beschadigd of heeft een ongeldige structuur",
        MsgKey::ClipboardWatching => "Klembord in realtime bewaken...",
        MsgKey::ClipboardReplaced => "Klembordinhoud opgeschoond en bijgewerkt",
        MsgKey::ConfigResetSuccess => "Standaardconfiguratie succesvol hersteld",
    }
}
