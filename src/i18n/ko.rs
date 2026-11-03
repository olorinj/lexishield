//! 한국어 메시지 카탈로그 (Korean messages for LexiShield).

use super::MsgKey;

pub fn translate(key: MsgKey) -> &'static str {
    match key {
        MsgKey::CliAbout => {
            "AI 쿼리 및 데이터 정제를 위한 고성능 의미론적 보호 및 익명화 엔진"
        }
        MsgKey::VerboseDesc => {
            "세부 정보를 위한 Debug 로그 레벨 활성화 (stderr로 원격 측정 출력)"
        }
        MsgKey::DebugDesc => {
            "stderr로 전체 원격 측정을 출력하는 완전한 DEBUG 모드 활성화"
        }
        MsgKey::ConfigPathDesc => "사용자 지정 TOML 구성 파일 경로",
        MsgKey::RulesPathDesc => {
            "사용자 지정 TOML 규칙 파일 경로 (~/.lexishield/rules.toml)"
        }
        MsgKey::LangDesc => "표시 언어 ('en', 'es', 'fr', 'de', 'pt', 'it', 'zh', 'ja', 'ru', 'pl', 'nl', 'ko')",
        MsgKey::EngineInitialized => "LexiShield 코어 엔진이 성공적으로 초기화되었습니다",
        MsgKey::ScanningStarted => "대상 파일 검색 시작 중...",
        MsgKey::FilesProcessed => "처리된 파일 수",
        MsgKey::EntitiesFound => "탐지 및 보호된 민감한 개체 수",
        MsgKey::SavedAndEncryptedMappings => "매핑이 안전하게 암호화되어 저장되었습니다:",
        MsgKey::DeobfuscationSuccess => "역익명화(복원) 작업이 성공적으로 완료되었습니다",
        MsgKey::CorruptedFileError => "파일이 손상되었거나 형식이 올바르지 않습니다",
        MsgKey::ClipboardWatching => "실시간으로 클립보드를 감시하는 중...",
        MsgKey::ClipboardReplaced => "클립보드 내용이 정제되어 업데이트되었습니다",
        MsgKey::ConfigResetSuccess => "기본 구성 설정이 성공적으로 복원되었습니다",
    }
}

