//! Katalog komunikatów w języku polskim dla LexiShield.

use super::MsgKey;

pub fn translate(key: MsgKey) -> &'static str {
    match key {
        MsgKey::CliAbout => {
            "Wysokowydajny silnik ochrony semantycznej i anonimizacji dla zapytań SI oraz sanityzacji danych"
        }
        MsgKey::VerboseDesc => {
            "Włącz poziom logowania Debug dla dodatkowych szczegółów (telemetria na stderr)"
        }
        MsgKey::DebugDesc => "Włącz pełny tryb DEBUG z wyczerpującą telemetrią na stderr",
        MsgKey::ConfigPathDesc => "Ścieżka do niestandardowego pliku konfiguracyjnego TOML",
        MsgKey::RulesPathDesc => {
            "Ścieżka do niestandardowego pliku reguł TOML (~/.lexishield/rules.toml)"
        }
        MsgKey::LangDesc => {
            "Język wyświetlania ('en', 'es', 'fr', 'de', 'pt', 'it', 'zh', 'ja', 'ru', 'pl', 'nl', 'ko')"
        }
        MsgKey::EngineInitialized => "Główny silnik LexiShield został pomyślnie zainicjowany",
        MsgKey::ScanningStarted => "Rozpoczynanie skanowania plików docelowych...",
        MsgKey::FilesProcessed => "Przetworzone pliki",
        MsgKey::EntitiesFound => "Wykryte i chronione jednostki",
        MsgKey::SavedAndEncryptedMappings => "Mapowania zapisane i ZASZYFROWANE w:",
        MsgKey::DeobfuscationSuccess => "Deanonymizacja zakończona pomyślnie",
        MsgKey::CorruptedFileError => "Plik jest uszkodzony lub ma nieprawidłową strukturę",
        MsgKey::ClipboardWatching => "Monitorowanie schowka w czasie rzeczywistym...",
        MsgKey::ClipboardReplaced => "Zawartość schowka została oczyszczona i zaktualizowana",
        MsgKey::ConfigResetSuccess => "Pomyślnie przywrócono konfigurację domyślną",
    }
}
