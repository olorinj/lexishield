//! Messages en français pour LexiShield.

use super::MsgKey;

pub fn translate(key: MsgKey) -> &'static str {
    match key {
        MsgKey::CliAbout => {
            "Moteur d'anonymisation et de protection sémantique haute performance pour requêtes IA et assainissement des données"
        }
        MsgKey::VerboseDesc => {
            "Activer le niveau de journalisation Debug pour plus de détails (télémétrie sur stderr)"
        }
        MsgKey::DebugDesc => {
            "Activer le mode DEBUG complet avec télémétrie exhaustive sur stderr"
        }
        MsgKey::ConfigPathDesc => "Chemin vers un fichier de configuration TOML personnalisé",
        MsgKey::RulesPathDesc => {
            "Chemin vers un fichier de règles TOML personnalisé (~/.lexishield/rules.toml)"
        }
        MsgKey::LangDesc => "Langue d'affichage ('en', 'es', 'fr', 'de', 'pt', 'it', 'zh')",
        MsgKey::EngineInitialized => "Moteur central LexiShield initialisé avec succès",
        MsgKey::ScanningStarted => "Démarrage de l'analyse des fichiers cibles...",
        MsgKey::FilesProcessed => "Fichiers traités",
        MsgKey::EntitiesFound => "Entités détectées et protégées",
        MsgKey::SavedAndEncryptedMappings => "Mappages enregistrés et CHIFFRÉS à :",
        MsgKey::DeobfuscationSuccess => "Désanonymisation terminée avec succès",
        MsgKey::CorruptedFileError => "Le fichier est corrompu ou a une structure invalide",
        MsgKey::ClipboardWatching => "Surveillance du presse-papiers en temps réel...",
        MsgKey::ClipboardReplaced => "Contenu du presse-papiers assaini et mis à jour",
        MsgKey::ConfigResetSuccess => "Configuration par défaut restaurée avec succès",
    }
}

