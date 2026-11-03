//! Messaggi in italiano per LexiShield.

use super::MsgKey;

pub fn translate(key: MsgKey) -> &'static str {
    match key {
        MsgKey::CliAbout => {
            "Motore di anonimizzazione e protezione semantica ad alte prestazioni per query IA e sanitizzazione dati"
        }
        MsgKey::VerboseDesc => {
            "Attiva livello di log Debug per dettagli aggiuntivi (telemetria su stderr)"
        }
        MsgKey::DebugDesc => "Attiva modalità DEBUG completa con telemetria esaustiva su stderr",
        MsgKey::ConfigPathDesc => "Percorso di un file di configurazione TOML personalizzato",
        MsgKey::RulesPathDesc => {
            "Percorso di un file di regole TOML personalizzato (~/.lexishield/rules.toml)"
        }
        MsgKey::LangDesc => "Lingua dell'interfaccia ('en', 'es', 'fr', 'de', 'pt', 'it', 'zh')",
        MsgKey::EngineInitialized => "Motore centrale LexiShield inizializzato con successo",
        MsgKey::ScanningStarted => "Avvio scansione dei file di destinazione...",
        MsgKey::FilesProcessed => "File elaborati",
        MsgKey::EntitiesFound => "Entità rilevate e protette",
        MsgKey::SavedAndEncryptedMappings => "Mappature salvate e CIFRATE in:",
        MsgKey::DeobfuscationSuccess => "Deanonimizzazione completata con successo",
        MsgKey::CorruptedFileError => "Il file è corrotto o presenta una struttura non valida",
        MsgKey::ClipboardWatching => "Monitoraggio degli appunti in tempo reale...",
        MsgKey::ClipboardReplaced => "Contenuto degli appunti sanitizzato e aggiornato",
        MsgKey::ConfigResetSuccess => "Configurazione predefinita ripristinata con successo",
    }
}
