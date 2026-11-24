//! Módulo de aprendizaje local libre de PII (Zero-Knowledge) para calibración de detectores.
//!
//! Convierte cualquier muestra de entrada en un esqueleto morfológico irreversible (`A`, `a`, `Á`, `á`, `9`)
//! antes de persistir en disco (`detector_learning.json`), garantizando compatibilidad nativa en
//! Windows (`%USERPROFILE%\.lexishield\`), Linux y macOS (`~/.lexishield/`) y permitiendo compartir
//! propuestas de mejora en GitHub sin exponer jamás datos confidenciales ni rutas locales.

use crate::config::get_user_config_dir;
use crate::models::ObfuscationError;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

pub const DETECTOR_LEARNING_FILE_NAME: &str = "detector_learning.json";
pub const DETECTOR_LEARNING_SCHEMA_VERSION: u32 = 1;

const ALLOWED_HINTS: &[&str] = &[
    "backslash_domain_user",
    "machine_account_dollar",
    "upn_internal_domain",
    "email_at",
    "street_prefix",
    "plus_country_code",
    "ipv4_dots",
    "ipv6_colons",
    "dni_nie_pattern",
    "uuid_hyphens",
    "service_prefix",
    "all_caps",
    "multi_word",
];

const STREET_PREFIX_ALLOWLIST: &[&str] = &[
    "calle",
    "c/",
    "avda",
    "avda.",
    "av.",
    "avenida",
    "paseo",
    "plaza",
    "pza",
    "camino",
    "carretera",
    "ctra",
    "ronda",
    "travesia",
    "travesía",
    "glorieta",
    "street",
    "st",
    "st.",
    "avenue",
    "ave",
    "ave.",
    "road",
    "rd",
    "boulevard",
    "blvd",
    "lane",
    "ln",
    "drive",
    "dr",
    "way",
    "court",
    "ct",
];

const INTERNAL_DOMAIN_SUFFIXES: &[&str] = &[
    ".local",
    ".corp",
    ".internal",
    ".lan",
    ".intranet",
    ".priv",
    ".ad",
    ".home",
];

/// Regla morfológica aprendida a partir de confirmaciones o correcciones del usuario.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LearnedPatternRule {
    pub skeleton_compact: String,
    pub sample_skeleton: String,
    pub word_count: usize,
    #[serde(default)]
    pub hints: Vec<String>,
    pub auto_predicted: String,
    pub user_confirmed: String,
    pub was_correct: bool,
    pub confirmations: u32,
    pub corrections: u32,
}

/// Contenedor persistente y exportable de calibración de detectores (libre de datos reales y rutas locales).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DetectorLearningStore {
    pub schema_version: u32,
    pub app_version: String,
    pub os_family: String,
    #[serde(default)]
    pub rules: Vec<LearnedPatternRule>,
}

impl Default for DetectorLearningStore {
    fn default() -> Self {
        Self {
            schema_version: DETECTOR_LEARNING_SCHEMA_VERSION,
            app_version: env!("CARGO_PKG_VERSION").to_string(),
            os_family: current_os_family().to_string(),
            rules: Vec::new(),
        }
    }
}

/// Devuelve la familia del sistema operativo actual (`"windows"`, `"linux"`, `"macos"`, etc.).
pub fn current_os_family() -> &'static str {
    std::env::consts::OS
}

/// Obtiene la ruta multiplataforma del fichero de aprendizaje (`~/.lexishield/detector_learning.json`
/// en Linux/macOS o `%USERPROFILE%\.lexishield\detector_learning.json` en Windows).
pub fn get_detector_learning_path() -> Result<PathBuf, ObfuscationError> {
    let dir = get_user_config_dir()?;
    Ok(dir.join(DETECTOR_LEARNING_FILE_NAME))
}

fn mask_char(ch: char) -> char {
    if ch.is_ascii_uppercase() {
        'A'
    } else if ch.is_ascii_lowercase() {
        'a'
    } else if ch.is_ascii_digit() {
        '9'
    } else if ch.is_uppercase() {
        'Á'
    } else if ch.is_lowercase() || ch.is_alphabetic() {
        'á'
    } else if matches!(
        ch,
        '\\' | '@' | '.' | '-' | '_' | '/' | ':' | '+' | ',' | '$' | '(' | ')' | '[' | ']'
    ) {
        ch
    } else if ch.is_whitespace() {
        ' '
    } else {
        '#'
    }
}

fn collapse_skeleton(exact: &str) -> String {
    let mut compact = String::with_capacity(exact.len());
    let mut chars = exact.chars().peekable();

    while let Some(ch) = chars.next() {
        let collapsible = matches!(ch, 'A' | 'a' | 'Á' | 'á' | '9' | ' ');
        if !collapsible {
            compact.push(ch);
            continue;
        }

        let mut count = 1usize;
        while chars.peek() == Some(&ch) {
            let _ = chars.next();
            count += 1;
        }

        if ch == ' ' {
            compact.push(' ');
        } else if count >= 2 {
            compact.push(ch);
            compact.push('+');
        } else {
            compact.push(ch);
        }
    }

    compact
}

fn extract_safe_hints(trimmed: &str, word_count: usize) -> Vec<String> {
    let mut hints = Vec::new();
    let lower = trimmed.to_lowercase();

    if let Some((left, right)) = trimmed.split_once('\\')
        && !left.is_empty()
        && !right.is_empty()
        && !right.contains('\\')
    {
        hints.push("backslash_domain_user".to_string());
        if right.ends_with('$') {
            hints.push("machine_account_dollar".to_string());
        }
    }

    if let Some((_, domain)) = lower.split_once('@') {
        if INTERNAL_DOMAIN_SUFFIXES
            .iter()
            .any(|sfx| domain.ends_with(sfx))
        {
            hints.push("upn_internal_domain".to_string());
        } else if domain.contains('.') {
            hints.push("email_at".to_string());
        }
    }

    let first_token = lower.split_whitespace().next().unwrap_or("");
    if STREET_PREFIX_ALLOWLIST.contains(&first_token)
        || lower.starts_with("c/")
        || STREET_PREFIX_ALLOWLIST
            .iter()
            .any(|kw| lower.split_whitespace().any(|tok| tok == *kw))
    {
        hints.push("street_prefix".to_string());
    }

    if trimmed.starts_with('+') && trimmed.chars().nth(1).is_some_and(|c| c.is_ascii_digit()) {
        hints.push("plus_country_code".to_string());
    }

    if lower.starts_with("svc_")
        || lower.starts_with("svc-")
        || lower.starts_with("adm_")
        || lower.starts_with("adm-")
        || lower.starts_with("admin_")
        || lower.starts_with("admin-")
    {
        hints.push("service_prefix".to_string());
    }

    let has_alpha = trimmed.chars().any(char::is_alphabetic);
    if has_alpha
        && trimmed
            .chars()
            .filter(|c| c.is_alphabetic())
            .all(char::is_uppercase)
    {
        hints.push("all_caps".to_string());
    }

    if word_count >= 2 {
        hints.push("multi_word".to_string());
    }

    hints
}

/// Construye el esqueleto morfológico irreversible y las pistas seguras de lista blanca a partir de un valor en memoria.
pub fn build_structural_skeleton(raw: &str) -> (String, String, usize, Vec<String>) {
    let trimmed = raw.trim();
    let word_count = trimmed.split_whitespace().count();
    let sample_skeleton: String = trimmed.chars().take(120).map(mask_char).collect();
    let skeleton_compact = collapse_skeleton(&sample_skeleton);
    let hints = extract_safe_hints(trimmed, word_count);
    (sample_skeleton, skeleton_compact, word_count, hints)
}

/// Verifica defensivamente que una cadena contenga únicamente tokens de esqueleto permitidos.
pub fn is_valid_sanitized_skeleton(skeleton: &str) -> bool {
    if skeleton.is_empty() || skeleton.chars().count() > 160 {
        return false;
    }
    skeleton.chars().all(|ch| {
        matches!(
            ch,
            'A' | 'a'
                | 'Á'
                | 'á'
                | '9'
                | '+'
                | '\\'
                | '@'
                | '.'
                | '-'
                | '_'
                | '/'
                | ':'
                | ','
                | '$'
                | '('
                | ')'
                | '['
                | ']'
                | '#'
                | ' '
        )
    })
}

fn sanitize_rule(rule: &mut LearnedPatternRule) -> bool {
    if !is_valid_sanitized_skeleton(&rule.sample_skeleton)
        || !is_valid_sanitized_skeleton(&rule.skeleton_compact)
    {
        return false;
    }
    rule.hints.retain(|h| ALLOWED_HINTS.contains(&h.as_str()));
    rule.auto_predicted = rule.auto_predicted.trim().chars().take(48).collect();
    rule.user_confirmed = rule.user_confirmed.trim().chars().take(48).collect();
    !rule.auto_predicted.is_empty() && !rule.user_confirmed.is_empty()
}

fn write_learning_file_securely(path: &Path, data: &[u8]) -> Result<(), ObfuscationError> {
    if let Some(parent) = path.parent()
        && !parent.exists()
    {
        fs::create_dir_all(parent).map_err(ObfuscationError::IoError)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(parent, fs::Permissions::from_mode(0o700));
        }
    }

    fs::write(path, data).map_err(ObfuscationError::IoError)?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(0o600));
    }

    Ok(())
}

/// Carga el almacén de aprendizaje desde disco (o devuelve uno vacío si aún no existe).
pub fn load_learning_store_from_path(path: &Path) -> DetectorLearningStore {
    if !path.exists() {
        return DetectorLearningStore::default();
    }
    match fs::read_to_string(path) {
        Ok(content) => match serde_json::from_str::<DetectorLearningStore>(&content) {
            Ok(mut store) => {
                store.schema_version = DETECTOR_LEARNING_SCHEMA_VERSION;
                store.app_version = env!("CARGO_PKG_VERSION").to_string();
                store.os_family = current_os_family().to_string();
                store.rules.retain_mut(sanitize_rule);
                store
            }
            Err(e) => {
                log::warn!(
                    "No se pudo interpretar el archivo de aprendizaje {} ({e}); iniciando almacén limpio",
                    path.display()
                );
                DetectorLearningStore::default()
            }
        },
        Err(e) => {
            log::warn!(
                "No se pudo leer el archivo de aprendizaje {} ({e}); iniciando almacén limpio",
                path.display()
            );
            DetectorLearningStore::default()
        }
    }
}

/// Carga el almacén de aprendizaje del perfil del usuario actual.
pub fn load_learning_store() -> Result<(DetectorLearningStore, PathBuf), ObfuscationError> {
    let path = get_detector_learning_path()?;
    let store = load_learning_store_from_path(&path);
    Ok((store, path))
}

/// Guarda el almacén de aprendizaje aplicando sanitización estricta y permisos restringidos.
pub fn save_learning_store_to_path(
    path: &Path,
    store: &DetectorLearningStore,
) -> Result<(), ObfuscationError> {
    let mut clean_store = store.clone();
    clean_store.schema_version = DETECTOR_LEARNING_SCHEMA_VERSION;
    clean_store.app_version = env!("CARGO_PKG_VERSION").to_string();
    clean_store.os_family = current_os_family().to_string();
    clean_store.rules.retain_mut(sanitize_rule);

    let json = serde_json::to_string_pretty(&clean_store).map_err(|e| {
        ObfuscationError::ConfigError(format!("Error serializando detector_learning.json: {e}"))
    })?;
    write_learning_file_securely(path, json.as_bytes())
}

/// Registra una confirmación o corrección del usuario convirtiendo la muestra en memoria a esqueleto sin PII.
pub fn record_feedback_in_store(
    store: &mut DetectorLearningStore,
    raw_value: &str,
    auto_predicted: &str,
    user_confirmed: &str,
) -> Option<LearnedPatternRule> {
    let trimmed = raw_value.trim();
    let pred = auto_predicted.trim();
    let conf = user_confirmed.trim();
    if trimmed.is_empty() || pred.is_empty() || conf.is_empty() {
        return None;
    }

    let (sample_skeleton, skeleton_compact, word_count, hints) = build_structural_skeleton(trimmed);
    if sample_skeleton.is_empty() {
        return None;
    }

    let was_correct = pred.eq_ignore_ascii_case(conf);

    if let Some(existing) = store
        .rules
        .iter_mut()
        .find(|r| r.skeleton_compact == skeleton_compact && r.hints == hints)
    {
        existing.sample_skeleton.clone_from(&sample_skeleton);
        existing.word_count = word_count;
        existing.auto_predicted = pred.to_string();
        existing.user_confirmed = conf.to_string();
        existing.was_correct = was_correct;
        if was_correct {
            existing.confirmations = existing.confirmations.saturating_add(1);
        } else {
            existing.corrections = existing.corrections.saturating_add(1);
        }
        return Some(existing.clone());
    }

    let mut rule = LearnedPatternRule {
        skeleton_compact,
        sample_skeleton,
        word_count,
        hints,
        auto_predicted: pred.to_string(),
        user_confirmed: conf.to_string(),
        was_correct,
        confirmations: u32::from(was_correct),
        corrections: u32::from(!was_correct),
    };

    if !sanitize_rule(&mut rule) {
        return None;
    }

    store.rules.push(rule.clone());
    Some(rule)
}

/// Registra feedback sobre el archivo de usuario y devuelve el almacén actualizado y su ruta física.
pub fn record_user_detector_feedback(
    raw_value: &str,
    auto_predicted: &str,
    user_confirmed: &str,
) -> Result<(DetectorLearningStore, PathBuf), ObfuscationError> {
    let (mut store, path) = load_learning_store()?;
    let _ = record_feedback_in_store(&mut store, raw_value, auto_predicted, user_confirmed);
    save_learning_store_to_path(&path, &store)?;
    Ok((store, path))
}

/// Construye el JSON exacto (filtrado opcionalmente por `skeleton_compact`) listo para previsualizar, exportar o enviar a GitHub.
pub fn build_shareable_learning_json(
    store: &DetectorLearningStore,
    selected_skeletons: Option<&[String]>,
) -> Result<String, ObfuscationError> {
    let mut filtered = store.clone();
    filtered.schema_version = DETECTOR_LEARNING_SCHEMA_VERSION;
    filtered.app_version = env!("CARGO_PKG_VERSION").to_string();
    filtered.os_family = current_os_family().to_string();
    filtered.rules.retain_mut(sanitize_rule);

    if let Some(allowed) = selected_skeletons {
        filtered
            .rules
            .retain(|r| allowed.contains(&r.skeleton_compact));
    }

    serde_json::to_string_pretty(&filtered).map_err(|e| {
        ObfuscationError::ConfigError(format!("Error generando JSON compartible: {e}"))
    })
}

/// Exporta las reglas seleccionadas por el usuario a una ruta elegida en disco.
pub fn export_user_detector_learning(
    dest_path: &Path,
    selected_skeletons: Option<&[String]>,
) -> Result<PathBuf, ObfuscationError> {
    let (store, _) = load_learning_store()?;
    let json = build_shareable_learning_json(&store, selected_skeletons)?;
    write_learning_file_securely(dest_path, json.as_bytes())?;
    Ok(dest_path.to_path_buf())
}

/// Elimina las reglas aprendidas seleccionadas o vacía por completo el almacén local.
pub fn clear_user_detector_learning(
    skeleton_to_remove: Option<&str>,
) -> Result<(DetectorLearningStore, PathBuf), ObfuscationError> {
    let (mut store, path) = load_learning_store()?;
    if let Some(target) = skeleton_to_remove {
        store.rules.retain(|r| r.skeleton_compact != target);
    } else {
        store.rules.clear();
    }
    save_learning_store_to_path(&path, &store)?;
    Ok((store, path))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skeleton_never_leaks_pii_across_entities() {
        let samples = [
            ("Juan Pérez García", "Aaaa Aáaaa Aaaaáa"),
            (r"ACMECORP\carlos.gomez", r"AAAAAAAA\aaaaaa.aaaaa"),
            (r"ES-MADRID\WS042$", r"AA-AAAAAA\AA999$"),
            ("maria.lopez@empresa.local", "aaaaa.aaaaa@aaaaaaa.aaaaa"),
            ("Calle Alcalá 145, 2B", "Aaaaa Aaaaaá 999, 9A"),
            ("+34 612 34 56 78", "+99 999 99 99 99"),
        ];

        for (raw, expected_exact) in samples {
            let (exact, compact, _, hints) = build_structural_skeleton(raw);
            assert_eq!(exact, expected_exact);
            assert!(is_valid_sanitized_skeleton(&exact));
            assert!(is_valid_sanitized_skeleton(&compact));
            assert!(!exact.to_lowercase().contains("juan"));
            assert!(!exact.to_lowercase().contains("carlos"));
            assert!(!exact.to_lowercase().contains("acmecorp"));
            assert!(!exact.to_lowercase().contains("empresa"));
            assert!(!exact.to_lowercase().contains("alcala"));
            assert!(!exact.contains('6'));
            for h in &hints {
                assert!(ALLOWED_HINTS.contains(&h.as_str()));
            }
        }
    }

    #[test]
    fn test_domain_user_and_street_hints() {
        let (_, compact_ad, _, hints_ad) = build_structural_skeleton(r"CONTOSO\jperez");
        assert_eq!(compact_ad, r"A+\a+");
        assert!(hints_ad.contains(&"backslash_domain_user".to_string()));

        let (_, _, _, hints_mach) = build_structural_skeleton(r"CORP\SRV01$");
        assert!(hints_mach.contains(&"backslash_domain_user".to_string()));
        assert!(hints_mach.contains(&"machine_account_dollar".to_string()));

        let (_, _, _, hints_upn) = build_structural_skeleton("jdoe@interno.local");
        assert!(hints_upn.contains(&"upn_internal_domain".to_string()));

        let (_, _, _, hints_addr) = build_structural_skeleton("Calle Mayor 42");
        assert!(hints_addr.contains(&"street_prefix".to_string()));
        assert!(hints_addr.contains(&"multi_word".to_string()));
    }

    #[test]
    fn test_invalid_skeleton_rejected() {
        assert!(!is_valid_sanitized_skeleton(""));
        assert!(!is_valid_sanitized_skeleton("Juan Perez")); // 'J', 'u', 'n', 'P', 'e', 'r', 'z' disallowed!
        assert!(!is_valid_sanitized_skeleton("192.168.1.1")); // '1', '2', '6', '8' disallowed (only '9')!
        assert!(is_valid_sanitized_skeleton("999.999.9.9"));
    }

    #[test]
    fn test_store_roundtrip_and_shareable_json_has_no_paths() {
        let tmp_path = std::env::temp_dir().join(format!(
            "test_lexishield_learning_{}.json",
            rand::random::<u32>()
        ));
        let mut store = DetectorLearningStore::default();

        let r1 =
            record_feedback_in_store(&mut store, r"MIEMPRESA\carlos", "GenericToken", "Username");
        assert!(r1.is_some());
        let r2 = record_feedback_in_store(&mut store, r"OTRAEMP\marta", "GenericToken", "Username");
        assert!(r2.is_some());
        assert_eq!(store.rules.len(), 1);
        assert_eq!(store.rules[0].corrections, 2);

        save_learning_store_to_path(&tmp_path, &store).expect("Debe guardar aprendizaje");
        let loaded = load_learning_store_from_path(&tmp_path);
        let _ = fs::remove_file(&tmp_path);

        assert_eq!(loaded.rules.len(), 1);
        assert_eq!(loaded.rules[0].user_confirmed, "Username");

        let json = build_shareable_learning_json(&loaded, None).expect("JSON válido");
        assert!(!json.to_lowercase().contains("miempresa"));
        assert!(!json.to_lowercase().contains("carlos"));
        assert!(!json.contains("test_lexishield_learning"));
        assert!(json.contains("\"os_family\""));
    }
}
