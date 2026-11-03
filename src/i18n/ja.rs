//! 日本語メッセージカタログ (Japanese messages for LexiShield).

use super::MsgKey;

pub fn translate(key: MsgKey) -> &'static str {
    match key {
        MsgKey::CliAbout => {
            "AIクエリおよびデータサニタイズのための高性能セマンティック保護・匿名化エンジン"
        }
        MsgKey::VerboseDesc => {
            "Debugログレベルを有効化して詳細を出力します（stderrへテレメトリ出力）"
        }
        MsgKey::DebugDesc => {
            "完全なDEBUGモードを有効化し、網羅的なテレメトリをstderrに出力します"
        }
        MsgKey::ConfigPathDesc => "カスタムTOML設定ファイルへのパス",
        MsgKey::RulesPathDesc => {
            "カスタムTOMLルールファイルへのパス (~/.lexishield/rules.toml)"
        }
        MsgKey::LangDesc => "表示言語 ('en', 'es', 'fr', 'de', 'pt', 'it', 'zh', 'ja', 'ru', 'pl', 'nl', 'ko')",
        MsgKey::EngineInitialized => "LexiShield コアエンジンが正常に初期化されました",
        MsgKey::ScanningStarted => "対象ファイルのスキャンを開始しています...",
        MsgKey::FilesProcessed => "処理されたファイル数",
        MsgKey::EntitiesFound => "検出され保護された機密エンティティ",
        MsgKey::SavedAndEncryptedMappings => "マッピングが安全に暗号化され保存されました:",
        MsgKey::DeobfuscationSuccess => "復元・逆匿名化が正常に完了しました",
        MsgKey::CorruptedFileError => "ファイルが破損しているか、無効な構造です",
        MsgKey::ClipboardWatching => "クリップボードをリアルタイムで監視中...",
        MsgKey::ClipboardReplaced => "クリップボードの内容がサニタイズされ更新されました",
        MsgKey::ConfigResetSuccess => "デフォルト設定が正常に復元されました",
    }
}

