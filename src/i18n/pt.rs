//! Mensagens em português para o LexiShield.

use super::MsgKey;

pub fn translate(key: MsgKey) -> &'static str {
    match key {
        MsgKey::CliAbout => {
            "Motor de anonimização e proteção semântica de alto desempenho para consultas de IA e sanitização de dados"
        }
        MsgKey::VerboseDesc => {
            "Ativar nível de log Debug para detalhes adicionais (telemetria em stderr)"
        }
        MsgKey::DebugDesc => "Ativar modo DEBUG completo com telemetria exaustiva em stderr",
        MsgKey::ConfigPathDesc => "Caminho para arquivo de configuração TOML personalizado",
        MsgKey::RulesPathDesc => {
            "Caminho para arquivo de regras TOML personalizado (~/.lexishield/rules.toml)"
        }
        MsgKey::LangDesc => "Idioma de exibição ('en', 'es', 'fr', 'de', 'pt', 'it', 'zh')",
        MsgKey::EngineInitialized => "Motor central do LexiShield inicializado com sucesso",
        MsgKey::ScanningStarted => "Iniciando varredura dos arquivos alvo...",
        MsgKey::FilesProcessed => "Ficheiros processados",
        MsgKey::EntitiesFound => "Entidades detetadas e protegidas",
        MsgKey::SavedAndEncryptedMappings => "Mapeamentos guardados e CIFRADOS em:",
        MsgKey::DeobfuscationSuccess => "Desanonimização concluída com sucesso",
        MsgKey::CorruptedFileError => "O ficheiro está corrompido ou tem uma estrutura inválida",
        MsgKey::ClipboardWatching => "A monitorizar a área de transferência em tempo real...",
        MsgKey::ClipboardReplaced => "Conteúdo da área de transferência sanitizado e atualizado",
        MsgKey::ConfigResetSuccess => "Configuração padrão restaurada com sucesso",
    }
}
