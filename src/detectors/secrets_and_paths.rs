//! Detectores semánticos para claves API / secretos criptográficos (`SecretKey`)
//! y rutas de sistema de archivos Windows / Linux / Web (`FilePath`).

use crate::models::DetectorType;
use rand::Rng;
use regex::Regex;
use std::sync::LazyLock;

// 1. Claves API con prefijo estructural inequívoco
static PREFIXED_API_KEY_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"\b(?:AIza[0-9A-Za-z\-_]{35}|ABQIAAAA[0-9A-Za-z\-_]{40,120}|6Lc[0-9A-Za-z\-_]{35,42}|AKIA[0-9A-Z]{16}|ghp_[0-9A-Za-z]{36,}|glpat-[0-9A-Za-z\-_]{20,})\b",
    )
    .expect("Regex de claves API con prefijo válida")
});

// 2. Atributos criptográficos y secretos explícitos en XML/JSON/KVP/CSV-RFC4180 (machineKey, SecretKey*, ApiKey*, etc.)
static NAMED_SECRET_VALUE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?i)\b(?:validationKey|decryptionKey|SecretKey[A-Za-z0-9_]*|ApiKey[A-Za-z0-9_]*|clientSecret|accessKey)\s*(?:=|:|(?:"{1,2}|\\")\s*value\s*=\s*)(?:"{1,2}|\\")([A-Za-z0-9+/=\-_]{16,256})(?:"{1,2}|\\")"#,
    )
    .expect("Regex de secretos por nombre de clave válida")
});

// 3. Pares <add key="...Secret...|...ApiKey..." value="..."/> en web.config / app.config (incluyendo comillas CSV "" o JSON \")
static XML_ADD_KEY_SECRET_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?i)<add\s+[^>]*\bkey\s*=\s*(?:"{1,2}|\\")(?:[^"\\]*(?:secret|apikey|api_key|token|password|passwd|pwd)[^"\\]*)(?:"{1,2}|\\")\s+[^>]*\bvalue\s*=\s*(?:"{1,2}|\\")([^"\s\\]{12,256})(?:"{1,2}|\\")"#,
    )
    .expect("Regex de <add key=secret value=...> válida")
});

// 4. Rutas Windows con unidad (C:\...) o UNC (\\server\share\...)
static WINDOWS_PATH_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)\b[a-z]:\\(?:[^\\/:*?"<>|\r\n\t,;'"`]+\\)*[^\\/:*?"<>|\r\n\t,;'"`]*"#)
        .expect("Regex de rutas Windows válida")
});

// 5. Rutas Unix de aplicación/usuario (/var/www/..., /home/..., /opt/..., /srv/..., /etc/...) y rutas web relativas (/extranet/error.aspx)
static UNIX_AND_WEB_PATH_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r#"(?:^|[\s"'=:(,;])((?:/(?:var|home|opt|srv|etc|usr/local)(?:/[A-Za-z0-9._\-]+)+/?)|(?:/[A-Za-z0-9._\-]{2,}(?:/[A-Za-z0-9._\-]+)*\.(?:aspx|asmx|svc|ashx|config|jsp|php|env|ini|pem|key|sql|bak)))\b"#,
    )
    .expect("Regex de rutas Unix/Web válida")
});

/// Rutas genéricas puras del sistema operativo que no revelan topología de aplicación ni identidad.
fn is_generic_os_path(path: &str) -> bool {
    let norm = path
        .trim()
        .trim_end_matches(['\\', '/'])
        .to_ascii_lowercase();
    matches!(
        norm.as_str(),
        "c:" | "c:\\windows"
            | "c:\\windows\\system32"
            | "c:\\windows\\syswow64"
            | "c:\\program files"
            | "c:\\program files (x86)"
            | "/etc"
            | "/var"
            | "/var/log"
            | "/usr/local"
            | "/usr/local/bin"
    )
}

#[derive(Default)]
pub struct SecretKeyDetector;

impl SecretKeyDetector {
    pub fn new() -> Self {
        Self
    }

    pub fn detector_type(&self) -> DetectorType {
        DetectorType::SecretKey
    }

    pub fn find_matches<'a>(&self, text: &'a str) -> Vec<(usize, usize, &'a str)> {
        let mut matches = Vec::new();

        for m in PREFIXED_API_KEY_REGEX.find_iter(text) {
            matches.push((m.start(), m.end(), m.as_str()));
        }

        for cap in NAMED_SECRET_VALUE_REGEX.captures_iter(text) {
            if let Some(m) = cap.get(1) {
                matches.push((m.start(), m.end(), m.as_str()));
            }
        }

        for cap in XML_ADD_KEY_SECRET_REGEX.captures_iter(text) {
            if let Some(m) = cap.get(1) {
                matches.push((m.start(), m.end(), m.as_str()));
            }
        }

        dedup_non_overlapping(matches)
    }

    /// Genera una clave sintética preservando el prefijo estructural, longitud exacta y alfabeto (hex vs base64url).
    pub fn generate_pseudonym(&self, original: &str) -> String {
        let mut rng = rand::thread_rng();
        let preserved_prefix = [
            "ABQIAAAA", "AIzaSy", "AIza", "6Lc", "AKIA", "ghp_", "glpat-",
        ]
        .into_iter()
        .find(|p| original.starts_with(p))
        .unwrap_or("");

        let rest = &original[preserved_prefix.len()..];
        let is_pure_hex = !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_hexdigit());
        let has_upper_hex = rest.bytes().any(|b| b.is_ascii_uppercase());

        let mut out = String::with_capacity(original.len());
        out.push_str(preserved_prefix);

        if is_pure_hex {
            const HEX_UPPER: &[u8; 16] = b"0123456789ABCDEF";
            const HEX_LOWER: &[u8; 16] = b"0123456789abcdef";
            let table = if has_upper_hex { HEX_UPPER } else { HEX_LOWER };
            for _ in 0..rest.len() {
                out.push(table[rng.gen_range(0..16)] as char);
            }
        } else {
            const ALNUM: &[u8; 62] =
                b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
            for ch in rest.chars() {
                if ch == '-' || ch == '_' || ch == '=' {
                    out.push(ch);
                } else if ch.is_ascii_uppercase() && preserved_prefix == "AKIA" {
                    out.push(b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"[rng.gen_range(0..36)] as char);
                } else {
                    out.push(ALNUM[rng.gen_range(0..ALNUM.len())] as char);
                }
            }
        }

        out
    }
}

#[derive(Default)]
pub struct FilePathDetector;

impl FilePathDetector {
    pub fn new() -> Self {
        Self
    }

    pub fn detector_type(&self) -> DetectorType {
        DetectorType::FilePath
    }

    pub fn find_matches<'a>(&self, text: &'a str) -> Vec<(usize, usize, &'a str)> {
        let mut matches = Vec::new();

        for m in WINDOWS_PATH_REGEX.find_iter(text) {
            let raw = m.as_str();
            if raw.len() >= 4 && !is_generic_os_path(raw) {
                matches.push((m.start(), m.end(), raw));
            }
        }

        for cap in UNIX_AND_WEB_PATH_REGEX.captures_iter(text) {
            if let Some(m) = cap.get(1) {
                let raw = m.as_str();
                // Descartar rutas precedidas por '://' (parte de una URL completa) o '</' (cierre XML)
                let start = m.start();
                if start > 0 {
                    let prefix = &text[..start];
                    if prefix.ends_with(":/")
                        || prefix.ends_with("://")
                        || prefix.ends_with('<')
                        || prefix
                            .chars()
                            .last()
                            .is_some_and(|c| c.is_ascii_alphanumeric())
                    {
                        continue;
                    }
                }
                if !is_generic_os_path(raw) {
                    matches.push((start, m.end(), raw));
                }
            }
        }

        dedup_non_overlapping(matches)
    }

    /// Genera una ruta sintética coherente preservando el estilo (Windows vs Unix), barra final y extensión.
    pub fn generate_pseudonym(&self, original: &str) -> String {
        let mut rng = rand::thread_rng();
        let id: u16 = rng.gen_range(0x1000..0xFFFF);

        if original.contains('\\') {
            let drive = &original[..2.min(original.len())];
            let trailing_slash = original.ends_with('\\');
            let trimmed = original.trim_end_matches('\\');
            let last_seg = trimmed.rsplit('\\').next().unwrap_or("");

            if !trailing_slash
                && let Some((_, ext)) = last_seg.rsplit_once('.')
                && ext.len() <= 10
                && ext.chars().all(|c| c.is_ascii_alphanumeric())
            {
                format!("{drive}\\app_path_{id:04x}\\file_{id:04x}.{ext}")
            } else if trailing_slash {
                format!("{drive}\\app_path_{id:04x}\\")
            } else {
                format!("{drive}\\app_path_{id:04x}")
            }
        } else {
            let trailing_slash = original.ends_with('/');
            let trimmed = original.trim_end_matches('/');
            let last_seg = trimmed.rsplit('/').next().unwrap_or("");

            if !trailing_slash
                && let Some((_, ext)) = last_seg.rsplit_once('.')
                && ext.len() <= 10
                && ext.chars().all(|c| c.is_ascii_alphanumeric())
            {
                format!("/app_path_{id:04x}/file_{id:04x}.{ext}")
            } else if trailing_slash {
                format!("/app_path_{id:04x}/")
            } else {
                format!("/app_path_{id:04x}")
            }
        }
    }
}

fn dedup_non_overlapping<'a>(
    mut matches: Vec<(usize, usize, &'a str)>,
) -> Vec<(usize, usize, &'a str)> {
    matches.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| b.1.cmp(&a.1)));
    let mut filtered: Vec<(usize, usize, &'a str)> = Vec::with_capacity(matches.len());
    for (start, end, val) in matches {
        if let Some(last) = filtered.last()
            && start < last.1
        {
            continue;
        }
        filtered.push((start, end, val));
    }
    filtered
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_key_detector_web_config_patterns() {
        let det = SecretKeyDetector::new();
        let xml = r#"
            <machineKey validationKey="DCAAB540C066886AD394F11FA840DDF5D590A37E" decryptionKey="69DD216F87406DA8B84F024559AA2396" />
            <add key="SecretKeyCaptcha" value="6Lc51E4eAAAAAEIRH2wAW5hwwkbhppiNRSpT2fxI"/>
            <add key="ApiKeyMaps" value="AIzaSyCU3zyGJ0Z-2cu02DMOO5NtylFzMz-MLrc"/>
            <add key="localhost" value="http://maps.google.com/maps?key=ABQIAAAAf989o3j_Zz6fvBG2siXRbBQrNl6sxnwmvdHK8_o7G0MfNm2b1RRbCTB9CMeSzRBXEj53rm8DRxXoHQ"/>
        "#;
        let found: Vec<&str> = det.find_matches(xml).into_iter().map(|m| m.2).collect();
        assert!(found.contains(&"DCAAB540C066886AD394F11FA840DDF5D590A37E"));
        assert!(found.contains(&"69DD216F87406DA8B84F024559AA2396"));
        assert!(found.contains(&"6Lc51E4eAAAAAEIRH2wAW5hwwkbhppiNRSpT2fxI"));
        assert!(found.contains(&"AIzaSyCU3zyGJ0Z-2cu02DMOO5NtylFzMz-MLrc"));
        assert!(found.iter().any(|k| k.starts_with("ABQIAAAA")));

        let p_hex = det.generate_pseudonym("DCAAB540C066886AD394F11FA840DDF5D590A37E");
        assert_eq!(p_hex.len(), 40);
        assert!(p_hex.bytes().all(|b| b.is_ascii_hexdigit()));

        let p_g = det.generate_pseudonym("AIzaSyCU3zyGJ0Z-2cu02DMOO5NtylFzMz-MLrc");
        assert!(p_g.starts_with("AIzaSy"));
        assert_eq!(p_g.len(), 39);
    }

    #[test]
    fn test_file_path_detector_windows_and_web() {
        let det = FilePathDetector::new();
        let sample = r#"meta=c:\inetpub\wwwroot\extranet\web.config|LEN=9076,jetwin:C:\Windows\,redirect="/extranet/error.aspx""#;
        let found: Vec<&str> = det.find_matches(sample).into_iter().map(|m| m.2).collect();
        assert!(found.contains(&r"c:\inetpub\wwwroot\extranet\web.config"));
        assert!(found.contains(&"/extranet/error.aspx"));
        assert!(!found.contains(&r"C:\Windows\"));

        let win_pseudo = det.generate_pseudonym(r"c:\inetpub\wwwroot\extranet\web.config");
        assert!(win_pseudo.starts_with(r"c:\app_path_"));
        assert!(win_pseudo.ends_with(".config"));
    }
}
