//! Каталог сообщений на русском языке для LexiShield.

use super::MsgKey;

pub fn translate(key: MsgKey) -> &'static str {
    match key {
        MsgKey::CliAbout => {
            "Высокопроизводительный движок семантической защиты и анонимизации для ИИ-запросов и очистки данных"
        }
        MsgKey::VerboseDesc => {
            "Включить уровень логирования Debug для подробностей (телеметрия в stderr)"
        }
        MsgKey::DebugDesc => {
            "Включить полный режим DEBUG с исчерпывающей телеметрией в stderr"
        }
        MsgKey::ConfigPathDesc => "Путь к пользовательскому файлу конфигурации TOML",
        MsgKey::RulesPathDesc => {
            "Путь к пользовательскому файлу правил TOML (~/.lexishield/rules.toml)"
        }
        MsgKey::LangDesc => "Язык интерфейса ('en', 'es', 'fr', 'de', 'pt', 'it', 'zh', 'ja', 'ru', 'pl', 'nl', 'ko')",
        MsgKey::EngineInitialized => "Основной движок LexiShield успешно инициализирован",
        MsgKey::ScanningStarted => "Запуск сканирования целевых файлов...",
        MsgKey::FilesProcessed => "Обработано файлов",
        MsgKey::EntitiesFound => "Обнаружено и защищено сущностей",
        MsgKey::SavedAndEncryptedMappings => "Сопоставления сохранены и ЗАШИФРОВАНЫ в:",
        MsgKey::DeobfuscationSuccess => "Деанонимизация успешно завершена",
        MsgKey::CorruptedFileError => "Файл поврежден или имеет неверную структуру",
        MsgKey::ClipboardWatching => "Мониторинг буфера обмена в реальном времени...",
        MsgKey::ClipboardReplaced => "Содержимое буфера обмена очищено и обновлено",
        MsgKey::ConfigResetSuccess => "Конфигурация по умолчанию успешно восстановлена",
    }
}

