//! LexiShield 简体中文消息目录。

use super::MsgKey;

pub fn translate(key: MsgKey) -> &'static str {
    match key {
        MsgKey::CliAbout => "用于人工智能查询与数据净化的高性能语义保护与脱敏引擎",
        MsgKey::VerboseDesc => "启用 Debug 调试日志级别以获取更详细信息（输出至 stderr）",
        MsgKey::DebugDesc => "启用包含详尽遥测输出的完整 DEBUG 模式（输出至 stderr）",
        MsgKey::ConfigPathDesc => "自定义 TOML 配置文件的路径",
        MsgKey::RulesPathDesc => "自定义 TOML 规则文件的路径 (~/.lexishield/rules.toml)",
        MsgKey::LangDesc => "界面与输出显示语言 ('en', 'es', 'fr', 'de', 'pt', 'it', 'zh')",
        MsgKey::EngineInitialized => "LexiShield 核心引擎初始化成功",
        MsgKey::ScanningStarted => "正在启动目标文件扫描...",
        MsgKey::FilesProcessed => "已处理文件数量",
        MsgKey::EntitiesFound => "已检测并保护的敏感实体",
        MsgKey::SavedAndEncryptedMappings => "映射表已安全加密保存至：",
        MsgKey::DeobfuscationSuccess => "反向还原脱敏成功完成",
        MsgKey::CorruptedFileError => "文件已损坏或结构不合规",
        MsgKey::ClipboardWatching => "正在实时监听系统剪贴板...",
        MsgKey::ClipboardReplaced => "剪贴板敏感数据已净化并完成替换",
        MsgKey::ConfigResetSuccess => "已成功恢复系统默认配置",
    }
}
