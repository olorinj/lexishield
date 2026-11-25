//! Detección, decodificación recursiva (tipo `strings`), reensamblado multiparte y re-codificación
//! biyectiva de cargas codificadas legibles (Base64 UTF-8/UTF-16LE, Hexadecimal y URL-encoding).

use crate::stopwords::is_protected_word;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::LazyLock;

/// Profundidad máxima de decodificación recursiva ("cebolla").
pub const MAX_DECODE_DEPTH: usize = 2;

/// Modo de tratamiento de cargas codificadas legibles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default, Hash)]
#[serde(rename_all = "snake_case")]
pub enum EncodedPayloadMode {
    /// Desactivado: no inspecciona ni transforma bloques codificados.
    Off,
    /// Decodifica en memoria, protege/restaura el interior y vuelve a codificar en su formato original.
    #[default]
    Reencode,
    /// Expande los bloques codificados a texto claro legible respetando el escape estructural.
    ExpandClear,
}

impl EncodedPayloadMode {
    pub fn parse_opt(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "off" | "none" | "false" | "disabled" => Self::Off,
            "clear" | "expand" | "expand_clear" | "plain" => Self::ExpandClear,
            _ => Self::Reencode,
        }
    }
}

/// Variante exacta de codificación detectada para garantizar re-codificación determinista 1:1.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EncodingKind {
    Base64Std {
        padded: bool,
        utf16le: bool,
    },
    Base64Url {
        padded: bool,
        utf16le: bool,
    },
    Hex {
        has_0x_prefix: bool,
        uppercase: bool,
    },
    UrlPercent,
}

/// Resultado de inspección y expansión en claro para la interfaz gráfica (Studio) o pre-análisis.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DecodeInspectResult {
    pub expanded_text: String,
    pub merged_chunks_text: Option<String>,
    pub blocks_decoded: usize,
    pub multipart_groups_merged: usize,
}

static CANDIDATE_B64_HEX_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?:0[xX][0-9a-fA-F]{16,})|(?:[A-Za-z0-9+/]{4,}(?:={1,2})?)|(?:[A-Za-z0-9\-_]{16,}(?:={1,2})?)")
        .expect("Regex de candidatos Base64/Hex válida")
});

static CANDIDATE_URL_WITH_PCT_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?:[^\s,;|\x22'<>()\[\]{}]+%[0-9a-fA-F]{2}[^\s,;|\x22'<>()\[\]{}]*)|(?:0[xX][0-9a-fA-F]{16,})|(?:[A-Za-z0-9+/]{4,}(?:={1,2})?)|(?:[A-Za-z0-9\-_]{16,}(?:={1,2})?)",
    )
    .expect("Regex de candidatos codificados con URL-percent válida")
});

#[inline]
fn candidate_regex_for(input: &str) -> &'static Regex {
    if input.as_bytes().contains(&b'%') {
        &CANDIDATE_URL_WITH_PCT_REGEX
    } else {
        &CANDIDATE_B64_HEX_REGEX
    }
}

const B64_STD_ENC: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const B64_URL_ENC: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

fn b64_val(b: u8, url_safe: bool) -> Option<u8> {
    match b {
        b'A'..=b'Z' => Some(b - b'A'),
        b'a'..=b'z' => Some(b - b'a' + 26),
        b'0'..=b'9' => Some(b - b'0' + 52),
        b'+' if !url_safe => Some(62),
        b'/' if !url_safe => Some(63),
        b'-' if url_safe => Some(62),
        b'_' if url_safe => Some(63),
        _ => None,
    }
}

/// Decodifica Base64 verificando bits finales canónicos para que `encode(decode(x)) == x`.
pub fn decode_base64_canonical(input: &str, url_safe: bool) -> Option<(Vec<u8>, bool)> {
    let bytes = input.as_bytes();
    if bytes.len() < 4 {
        return None;
    }

    let eq_count = bytes.iter().rev().take_while(|&&b| b == b'=').count();
    if eq_count > 2 {
        return None;
    }
    let padded = eq_count > 0;
    if padded && !bytes.len().is_multiple_of(4) {
        return None;
    }

    let data = &bytes[..bytes.len() - eq_count];
    if data.contains(&b'=') {
        return None;
    }

    let rem = data.len() % 4;
    if rem == 1 {
        return None;
    }
    if padded && ((rem == 2 && eq_count != 2) || (rem == 3 && eq_count != 1) || rem == 0) {
        return None;
    }

    let mut out = Vec::with_capacity((data.len() * 3) / 4);
    let (chunks, remainder) = data.as_chunks::<4>();

    for chunk in chunks {
        let v0 = b64_val(chunk[0], url_safe)?;
        let v1 = b64_val(chunk[1], url_safe)?;
        let v2 = b64_val(chunk[2], url_safe)?;
        let v3 = b64_val(chunk[3], url_safe)?;

        out.push((v0 << 2) | (v1 >> 4));
        out.push((v1 << 4) | (v2 >> 2));
        out.push((v2 << 6) | v3);
    }

    if remainder.len() == 2 {
        let v0 = b64_val(remainder[0], url_safe)?;
        let v1 = b64_val(remainder[1], url_safe)?;
        if (v1 & 0x0F) != 0 {
            return None; // Bits sobrantes no canónicos
        }
        out.push((v0 << 2) | (v1 >> 4));
    } else if remainder.len() == 3 {
        let v0 = b64_val(remainder[0], url_safe)?;
        let v1 = b64_val(remainder[1], url_safe)?;
        let v2 = b64_val(remainder[2], url_safe)?;
        if (v2 & 0x03) != 0 {
            return None; // Bits sobrantes no canónicos
        }
        out.push((v0 << 2) | (v1 >> 4));
        out.push((v1 << 4) | (v2 >> 2));
    }

    Some((out, padded))
}

/// Codifica bytes en Base64 estándar o URL-safe con o sin relleno `=`.
pub fn encode_base64(bytes: &[u8], url_safe: bool, padded: bool) -> String {
    let table = if url_safe { B64_URL_ENC } else { B64_STD_ENC };
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);

    let (chunks, rem) = bytes.as_chunks::<3>();

    for c in chunks {
        let b0 = c[0];
        let b1 = c[1];
        let b2 = c[2];
        out.push(table[usize::from(b0 >> 2)] as char);
        out.push(table[usize::from(((b0 & 0x03) << 4) | (b1 >> 4))] as char);
        out.push(table[usize::from(((b1 & 0x0F) << 2) | (b2 >> 6))] as char);
        out.push(table[usize::from(b2 & 0x3F)] as char);
    }

    if rem.len() == 1 {
        let b0 = rem[0];
        out.push(table[usize::from(b0 >> 2)] as char);
        out.push(table[usize::from((b0 & 0x03) << 4)] as char);
        if padded {
            out.push_str("==");
        }
    } else if rem.len() == 2 {
        let b0 = rem[0];
        let b1 = rem[1];
        out.push(table[usize::from(b0 >> 2)] as char);
        out.push(table[usize::from(((b0 & 0x03) << 4) | (b1 >> 4))] as char);
        out.push(table[usize::from((b1 & 0x0F) << 2)] as char);
        if padded {
            out.push('=');
        }
    }

    out
}

fn decode_hex_bytes(hex_str: &str) -> Option<(Vec<u8>, bool, bool)> {
    let (raw, has_prefix) = if let Some(s) = hex_str
        .strip_prefix("0x")
        .or_else(|| hex_str.strip_prefix("0X"))
    {
        (s, true)
    } else {
        (hex_str, false)
    };

    if raw.len() < 16 || !raw.len().is_multiple_of(2) || !raw.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return None;
    }

    let has_upper = raw.bytes().any(|b| b.is_ascii_uppercase());
    let has_lower = raw.bytes().any(|b| b.is_ascii_lowercase());
    // Evitar confundir cadenas Base64 alfanuméricas que casualmente solo usen [0-9A-Fa-f] con mezcla caótica si no tienen 0x
    if !has_prefix && has_upper && has_lower {
        return None;
    }

    let mut out = Vec::with_capacity(raw.len() / 2);
    let bytes = raw.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let hi = (bytes[i] as char).to_digit(16)?;
        let lo = (bytes[i + 1] as char).to_digit(16)?;
        let byte = u8::try_from((hi << 4) | lo).ok()?;
        out.push(byte);
        i += 2;
    }

    Some((out, has_prefix, has_upper))
}

fn encode_hex_bytes(bytes: &[u8], has_0x_prefix: bool, uppercase: bool) -> String {
    const LOWER: &[u8; 16] = b"0123456789abcdef";
    const UPPER: &[u8; 16] = b"0123456789ABCDEF";
    let table = if uppercase { UPPER } else { LOWER };
    let mut out = String::with_capacity(bytes.len() * 2 + if has_0x_prefix { 2 } else { 0 });
    if has_0x_prefix {
        out.push_str("0x");
    }
    for &b in bytes {
        out.push(table[usize::from(b >> 4)] as char);
        out.push(table[usize::from(b & 0x0F)] as char);
    }
    out
}

fn decode_url_percent(input: &str) -> Option<String> {
    let bytes = input.as_bytes();
    let mut pct_count = 0usize;
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;

    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            let h1 = (bytes[i + 1] as char).to_digit(16);
            let h2 = (bytes[i + 2] as char).to_digit(16);
            if let (Some(hi), Some(lo)) = (h1, h2)
                && let Ok(val) = u8::try_from((hi << 4) | lo)
            {
                out.push(val);
                pct_count += 1;
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }

    if pct_count < 2 {
        return None;
    }

    let s = String::from_utf8(out).ok()?;
    if is_printable_human_text(&s, 70) {
        Some(s)
    } else {
        None
    }
}

fn encode_url_percent(input: &str) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(input.len() * 3 / 2);
    for &b in input.as_bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push('%');
            out.push(HEX[usize::from(b >> 4)] as char);
            out.push(HEX[usize::from(b & 0x0F)] as char);
        }
    }
    out
}

/// Codifica una cadena de texto claro según la variante exacta `EncodingKind`.
pub fn encode_with_kind(text: &str, kind: EncodingKind) -> String {
    match kind {
        EncodingKind::Base64Std { padded, utf16le } => {
            if utf16le {
                let bytes: Vec<u8> = text.encode_utf16().flat_map(u16::to_le_bytes).collect();
                encode_base64(&bytes, false, padded)
            } else {
                encode_base64(text.as_bytes(), false, padded)
            }
        }
        EncodingKind::Base64Url { padded, utf16le } => {
            if utf16le {
                let bytes: Vec<u8> = text.encode_utf16().flat_map(u16::to_le_bytes).collect();
                encode_base64(&bytes, true, padded)
            } else {
                encode_base64(text.as_bytes(), true, padded)
            }
        }
        EncodingKind::Hex {
            has_0x_prefix,
            uppercase,
        } => encode_hex_bytes(text.as_bytes(), has_0x_prefix, uppercase),
        EncodingKind::UrlPercent => encode_url_percent(text),
    }
}

/// Comprueba si una cadena decodificada es texto humano/estructurado legible (equivalente a `strings`).
/// `min_signal_pct` indica el porcentaje mínimo (0..=100) de caracteres alfanuméricos o delimitadores comunes.
pub fn is_printable_human_text(s: &str, min_signal_pct: usize) -> bool {
    if s.is_empty() {
        return false;
    }

    let mut total_chars = 0usize;
    let mut printable_chars = 0usize;
    let mut signal_chars = 0usize;
    let mut non_ws_chars = 0usize;

    for ch in s.chars() {
        if ch == '\0' || ch == '\u{FFFD}' {
            return false;
        }
        if ch.is_control() && !matches!(ch, '\n' | '\r' | '\t') {
            return false;
        }

        total_chars += 1;
        if !ch.is_control() || matches!(ch, '\n' | '\r' | '\t') {
            printable_chars += 1;
        }

        if !ch.is_whitespace() {
            non_ws_chars += 1;
            if ch.is_alphanumeric()
                || matches!(
                    ch,
                    '<' | '>'
                        | '/'
                        | '\\'
                        | ':'
                        | '='
                        | '"'
                        | '\''
                        | ','
                        | '.'
                        | ';'
                        | '|'
                        | '-'
                        | '_'
                        | '['
                        | ']'
                        | '{'
                        | '}'
                        | '('
                        | ')'
                        | '?'
                        | '&'
                        | '@'
                        | '#'
                        | '+'
                        | '*'
                        | '%'
                )
            {
                signal_chars += 1;
            }
        }
    }

    if total_chars == 0 || printable_chars * 100 < total_chars * 95 {
        return false;
    }

    if non_ws_chars > 0 && signal_chars * 100 < non_ws_chars * min_signal_pct {
        return false;
    }

    true
}

fn try_decode_utf8_or_utf16le(bytes: &[u8], min_signal_pct: usize) -> Option<(String, bool)> {
    if bytes.is_empty() {
        return None;
    }

    if let Ok(utf8_str) = std::str::from_utf8(bytes)
        && is_printable_human_text(utf8_str, min_signal_pct)
    {
        return Some((utf8_str.to_string(), false));
    }

    // Comprobar UTF-16LE (típico en Windows / PowerShell -EncodedCommand)
    if bytes.len() >= 8 && bytes.len().is_multiple_of(2) {
        let pair_count = bytes.len() / 2;
        let mut ascii_high_zeros = 0usize;
        let mut units = Vec::with_capacity(pair_count);

        for &pair in bytes.as_chunks::<2>().0 {
            let u = u16::from_le_bytes(pair);
            if u == 0 {
                return None;
            }
            if pair[1] == 0 && pair[0].is_ascii_graphic() {
                ascii_high_zeros += 1;
            }
            units.push(u);
        }

        if ascii_high_zeros * 100 >= pair_count * 50
            && let Ok(utf16_str) = String::from_utf16(&units)
            && is_printable_human_text(&utf16_str, 75)
        {
            return Some((utf16_str, true));
        }
    }

    None
}

fn looks_like_normal_word_or_identifier(token: &str) -> bool {
    if is_protected_word(token) {
        return true;
    }
    // Palabras puramente minúsculas ("ping", "meta", "done", "true", "null") o Capitalizadas ("Info", "Type", "Data")
    let mut chars = token.chars();
    let Some(first) = chars.next() else {
        return true;
    };
    let rest = chars.as_str();
    if first.is_ascii_lowercase() && rest.bytes().all(|b| b.is_ascii_lowercase()) {
        return true;
    }
    if first.is_ascii_uppercase()
        && !rest.is_empty()
        && rest.bytes().all(|b| b.is_ascii_lowercase())
    {
        return true;
    }
    // Números decimales puros
    if token.bytes().all(|b| b.is_ascii_digit()) {
        return true;
    }
    false
}

/// Intenta decodificar un token candidato individual.
/// `in_encoded_column`: true si el token pertenece a una columna CSV/TSV donde otras filas son inequívocamente codificadas.
pub fn try_decode_token(token: &str, in_encoded_column: bool) -> Option<(String, EncodingKind)> {
    let trimmed = token.trim();
    if trimmed.len() < 4 {
        return None;
    }

    // 1. URL-encoding (%XX)
    if trimmed.contains('%')
        && let Some(decoded) = decode_url_percent(trimmed)
    {
        return Some((decoded, EncodingKind::UrlPercent));
    }

    // 2. Hexadecimal (con o sin 0x, >= 16 hex chars)
    if let Some((hex_bytes, has_prefix, uppercase)) = decode_hex_bytes(trimmed)
        && let Ok(s) = std::str::from_utf8(&hex_bytes)
        && is_printable_human_text(s, 85)
        && s.chars().filter(char::is_ascii_alphanumeric).count() * 100
            >= s.chars().count().max(1) * 55
    {
        return Some((
            s.to_string(),
            EncodingKind::Hex {
                has_0x_prefix: has_prefix,
                uppercase,
            },
        ));
    }

    // 3. Base64 (Estándar o URL-safe)
    let has_url_chars = trimmed.contains('-') || trimmed.contains('_');
    let has_std_chars = trimmed.contains('+') || trimmed.contains('/');
    if has_url_chars && has_std_chars {
        return None;
    }

    // Evitar confundir claves API públicas conocidas que empiezan por AIza / ABQIAAAA / ghp_ / glpat- con Base64 de texto
    if trimmed.starts_with("AIza")
        || trimmed.starts_with("ABQIAAAA")
        || trimmed.starts_with("ghp_")
        || trimmed.starts_with("glpat-")
        || trimmed.starts_with("AKIA")
        || trimmed.starts_with("6Lc")
    {
        return None;
    }

    let url_safe = has_url_chars;
    let has_padding = trimmed.ends_with('=');

    if !has_padding {
        if url_safe {
            if trimmed.len() < 16 {
                return None;
            }
            // Evitar confundir identificadores con guiones/subrayados tipo snake_case o kebab-case
            let sep_count = trimmed.bytes().filter(|&b| b == b'-' || b == b'_').count();
            if sep_count * 6 >= trimmed.len() {
                return None;
            }
        } else {
            if !trimmed.len().is_multiple_of(4) {
                return None;
            }
            if looks_like_normal_word_or_identifier(trimmed) {
                return None;
            }
        }
    }

    let (raw_bytes, padded) = decode_base64_canonical(trimmed, url_safe)?;
    let min_signal = if has_padding || trimmed.len() >= 20 {
        75
    } else {
        95
    };

    let (decoded_str, utf16le) = try_decode_utf8_or_utf16le(&raw_bytes, min_signal)?;

    // Reglas estrictas contra falsos positivos en tokens cortos sin relleno '=' (4..16 chars, ej. "QUFB" -> "AAA")
    if !padded && trimmed.len() < 16 {
        let all_clean_alnum = decoded_str
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.' | '/' | '\\' | ':'));
        if !all_clean_alnum {
            return None;
        }
        if !in_encoded_column {
            // Fuera de una columna CSV codificada, exigir al menos 8 caracteres y mezcla de mayúsculas/dígitos/+ /
            if trimmed.len() < 8
                || !trimmed
                    .bytes()
                    .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'+' || b == b'/')
            {
                return None;
            }
        } else if !decoded_str.chars().all(|c| c.is_ascii_alphanumeric()) {
            return None;
        }
    } else if !padded && !in_encoded_column && !has_std_chars {
        // Para tokens de >= 16 chars sin '+', '/' ni '=', rechazar identificadores camelCase simples cuya "decodificación" tenga símbolos raros
        let weird_punct = decoded_str
            .chars()
            .filter(|c| {
                !c.is_alphanumeric()
                    && !c.is_whitespace()
                    && !matches!(
                        c,
                        '<' | '>'
                            | '/'
                            | '\\'
                            | ':'
                            | '='
                            | '"'
                            | '\''
                            | ','
                            | '.'
                            | ';'
                            | '|'
                            | '-'
                            | '_'
                    )
            })
            .count();
        if weird_punct * 10 > decoded_str.chars().count().max(1) {
            return None;
        }
    }

    let kind = if url_safe {
        EncodingKind::Base64Url { padded, utf16le }
    } else {
        EncodingKind::Base64Std {
            padded: padded || trimmed.len().is_multiple_of(4),
            utf16le,
        }
    };

    Some((decoded_str, kind))
}

/// Verifica si un token tiene codificación de alta confianza (para inferir contexto de columna CSV).
fn is_high_confidence_encoded(token: &str) -> bool {
    let t = token.trim();
    if (t.ends_with('=') && t.len() >= 8) || t.len() >= 24 {
        return try_decode_token(t, false).is_some();
    }
    false
}

/// Parsea una línea CSV básica respetando comillas RFC 4180 (una sola línea).
fn split_csv_row(line: &str, delimiter: char) -> Option<Vec<String>> {
    if !line.contains(delimiter) {
        return None;
    }
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut chars = line.chars().peekable();
    let mut in_quotes = false;

    while let Some(ch) = chars.next() {
        if in_quotes {
            if ch == '"' {
                if chars.peek() == Some(&'"') {
                    current.push('"');
                    let _ = chars.next();
                } else {
                    in_quotes = false;
                }
            } else {
                current.push(ch);
            }
        } else if ch == '"' && current.is_empty() {
            in_quotes = true;
        } else if ch == delimiter {
            fields.push(std::mem::take(&mut current));
        } else {
            current.push(ch);
        }
    }

    if in_quotes {
        return None;
    }
    fields.push(current);
    Some(fields)
}

/// Formatea un campo CSV según RFC 4180, entrecomillando si contiene comas, comillas o saltos de línea.
pub fn format_csv_cell(val: &str, delimiter: char) -> String {
    if val.contains(delimiter) || val.contains('"') || val.contains('\n') || val.contains('\r') {
        let escaped = val.replace('"', "\"\"");
        format!("\"{escaped}\"")
    } else {
        val.to_string()
    }
}

fn format_csv_row(fields: &[String], delimiter: char) -> String {
    let mut out = String::new();
    for (i, f) in fields.iter().enumerate() {
        if i > 0 {
            out.push(delimiter);
        }
        out.push_str(&format_csv_cell(f, delimiter));
    }
    out
}

/// Expande recursivamente (hasta `max_depth` niveles) los tokens codificados legibles dentro de un texto libre.
pub fn expand_encoded_in_str(
    input: &str,
    in_encoded_column: bool,
    depth: usize,
) -> (String, usize) {
    if depth >= MAX_DECODE_DEPTH || input.len() < 4 {
        return (input.to_string(), 0);
    }

    // Si todo el string de entrada (p. ej. una celda CSV) es un único bloque codificado
    let trimmed = input.trim();
    if !trimmed.contains(char::is_whitespace)
        && let Some((decoded_once, _)) = try_decode_token(trimmed, in_encoded_column)
    {
        let (decoded_nested, sub_count) = expand_encoded_in_str(&decoded_once, false, depth + 1);
        return (decoded_nested, 1 + sub_count);
    }

    let mut out = String::with_capacity(input.len());
    let mut last_end = 0usize;
    let mut decoded_blocks = 0usize;
    let bytes = input.as_bytes();

    for m in candidate_regex_for(input).find_iter(input) {
        let start = m.start();
        let end = m.end();

        // Respetar fronteras: no cortar en mitad de un token alfanumérico/ruta URL
        if start > 0 {
            let prev = bytes[start - 1];
            if prev.is_ascii_alphanumeric() || matches!(prev, b'_' | b'-' | b'/' | b'+') {
                continue;
            }
        }
        if end < bytes.len() {
            let next = bytes[end];
            if next.is_ascii_alphanumeric() || matches!(next, b'_' | b'-' | b'=') {
                continue;
            }
        }

        if let Some((decoded_once, _)) = try_decode_token(m.as_str(), false) {
            out.push_str(&input[last_end..start]);
            let (decoded_nested, sub_count) =
                expand_encoded_in_str(&decoded_once, false, depth + 1);
            out.push_str(&decoded_nested);
            last_end = end;
            decoded_blocks += 1 + sub_count;
        }
    }

    if decoded_blocks == 0 {
        return (input.to_string(), 0);
    }

    out.push_str(&input[last_end..]);
    (out, decoded_blocks)
}

/// Transforma recursivamente los bloques codificados aplicando `inner_transform` y re-codificando en su `EncodingKind` exacto.
pub fn transform_and_reencode_in_str<F>(
    input: &str,
    in_encoded_column: bool,
    depth: usize,
    inner_transform: &mut F,
) -> (String, usize, usize)
where
    F: FnMut(&str) -> (String, usize),
{
    if depth >= MAX_DECODE_DEPTH || input.len() < 4 {
        return (input.to_string(), 0, 0);
    }

    let trimmed = input.trim();
    if trimmed == input
        && !trimmed.contains(char::is_whitespace)
        && let Some((decoded_once, kind)) = try_decode_token(trimmed, in_encoded_column)
    {
        let (nested_done, sub_repl, sub_blocks) =
            transform_and_reencode_in_str(&decoded_once, false, depth + 1, inner_transform);
        let (transformed_inner, repl_count) = inner_transform(&nested_done);
        let total_repl = sub_repl + repl_count;
        let final_encoded = if transformed_inner == decoded_once {
            input.to_string()
        } else {
            encode_with_kind(&transformed_inner, kind)
        };
        return (final_encoded, total_repl, 1 + sub_blocks);
    }

    let mut out = String::with_capacity(input.len());
    let mut last_end = 0usize;
    let mut total_replacements = 0usize;
    let mut encoded_blocks = 0usize;
    let bytes = input.as_bytes();

    for m in candidate_regex_for(input).find_iter(input) {
        let start = m.start();
        let end = m.end();

        if start > 0 {
            let prev = bytes[start - 1];
            if prev.is_ascii_alphanumeric() || matches!(prev, b'_' | b'-' | b'/' | b'+') {
                continue;
            }
        }
        if end < bytes.len() {
            let next = bytes[end];
            if next.is_ascii_alphanumeric() || matches!(next, b'_' | b'-' | b'=') {
                continue;
            }
        }

        if let Some((decoded_once, kind)) = try_decode_token(m.as_str(), false) {
            out.push_str(&input[last_end..start]);
            let (nested_done, sub_repl, sub_blocks) =
                transform_and_reencode_in_str(&decoded_once, false, depth + 1, inner_transform);
            let (transformed_inner, repl_count) = inner_transform(&nested_done);
            total_replacements += sub_repl + repl_count;
            encoded_blocks += 1 + sub_blocks;

            if transformed_inner == decoded_once {
                out.push_str(m.as_str());
            } else {
                out.push_str(&encode_with_kind(&transformed_inner, kind));
            }
            last_end = end;
        }
    }

    if encoded_blocks == 0 {
        return (input.to_string(), 0, 0);
    }

    out.push_str(&input[last_end..]);
    (out, total_replacements, encoded_blocks)
}

#[derive(Debug, Clone)]
struct ParsedTabularCsv {
    delimiter: char,
    has_crlf: bool,
    trailing_newline: bool,
    header: Option<Vec<String>>,
    rows: Vec<Vec<String>>,
    encoded_cols: HashSet<usize>,
    /// Índices de columnas (group_col, idx_col, payload_col) cuando existen fragmentos multiparte (ej. id,kind,idx,v).
    multipart_spec: Option<(usize, usize, usize)>,
}

fn detect_tabular_csv(content: &str) -> Option<ParsedTabularCsv> {
    let has_crlf = content.contains("\r\n");
    let trailing_newline = content.ends_with('\n');
    let lines: Vec<&str> = content.lines().collect();
    if lines.len() < 2 {
        return None;
    }

    let delimiter = if lines[0].contains(',') {
        ','
    } else if lines[0].contains('\t') {
        '\t'
    } else if lines[0].contains(';') {
        ';'
    } else {
        return None;
    };

    let first_fields = split_csv_row(lines[0], delimiter)?;
    if first_fields.len() < 2 {
        return None;
    }
    let col_count = first_fields.len();

    let mut all_rows = Vec::with_capacity(lines.len());
    for l in &lines {
        if l.trim().is_empty() {
            continue;
        }
        let f = split_csv_row(l, delimiter)?;
        if f.len() != col_count {
            return None;
        }
        all_rows.push(f);
    }

    if all_rows.len() < 2 {
        return None;
    }

    // Determinar si la primera fila es cabecera (ningún número puro ni Base64 de alta confianza)
    let first_is_header = all_rows[0].iter().all(|cell| {
        let c = cell.trim();
        !c.is_empty() && !c.bytes().all(|b| b.is_ascii_digit()) && !is_high_confidence_encoded(c)
    });

    let (header, rows) = if first_is_header {
        let mut iter = all_rows.into_iter();
        let h = iter.next();
        (h, iter.collect::<Vec<_>>())
    } else {
        (None, all_rows)
    };

    if rows.is_empty() {
        return None;
    }

    let mut encoded_cols = HashSet::new();
    for col in 0..col_count {
        let high_conf = rows
            .iter()
            .filter(|r| is_high_confidence_encoded(&r[col]))
            .count();
        if high_conf >= 1 && (high_conf * 4 >= rows.len() || high_conf >= 2) {
            encoded_cols.insert(col);
        }
    }

    if encoded_cols.is_empty() {
        return None;
    }

    // Detectar estructura multiparte tipo (..., kind, idx, v) donde varias filas comparten `kind` y `idx` numérico
    let mut multipart_spec = None;
    if col_count >= 3 {
        let payload_col = col_count - 1;
        if encoded_cols.contains(&payload_col) {
            let idx_col = col_count - 2;
            let group_col = col_count - 3;
            let all_idx_numeric = rows
                .iter()
                .all(|r| r[idx_col].trim().parse::<u64>().is_ok());
            if all_idx_numeric {
                let mut groups: HashMap<&str, Vec<u64>> = HashMap::new();
                for r in &rows {
                    if let Ok(idx_val) = r[idx_col].trim().parse::<u64>() {
                        groups.entry(r[group_col].trim()).or_default().push(idx_val);
                    }
                }
                let has_multi_offset_group = groups.values().any(|offsets| {
                    if offsets.len() < 2 {
                        return false;
                    }
                    let unique: HashSet<u64> = offsets.iter().copied().collect();
                    unique.len() >= 2
                });
                if has_multi_offset_group {
                    multipart_spec = Some((group_col, idx_col, payload_col));
                }
            }
        }
    }

    Some(ParsedTabularCsv {
        delimiter,
        has_crlf,
        trailing_newline,
        header,
        rows,
        encoded_cols,
        multipart_spec,
    })
}

/// Comprueba si un grupo de fragmentos ordenados `(idx, decoded_text)` corresponde a cortes por offset de bytes/caracteres
/// de un mismo documento (ej. `idx = 0, 1400, 2800...`) frente a registros independientes numerados (`0, 1, 2...`).
fn is_contiguous_byte_offset_group(chunks: &[(u64, String)]) -> bool {
    if chunks.len() < 2 {
        return false;
    }
    let mut matches = 0usize;
    for pair in chunks.windows(2) {
        let expected_bytes = pair[0]
            .0
            .saturating_add(u64::try_from(pair[0].1.len()).unwrap_or(0));
        let expected_chars = pair[0]
            .0
            .saturating_add(u64::try_from(pair[0].1.chars().count()).unwrap_or(0));
        let next_idx = pair[1].0;
        if next_idx == expected_bytes || next_idx == expected_chars {
            matches += 1;
        }
    }
    matches * 2 >= chunks.len() - 1
}

/// Inspecciona un texto y devuelve tanto la expansión celda a celda en claro como (si procede)
/// la variante con fragmentos multiparte ordenados y unidos.
pub fn inspect_and_expand_encoded(content: &str) -> DecodeInspectResult {
    if content.len() < 4 {
        return DecodeInspectResult {
            expanded_text: content.to_string(),
            merged_chunks_text: None,
            blocks_decoded: 0,
            multipart_groups_merged: 0,
        };
    }

    // Ruta 1: Tabla CSV/TSV estructurada con columnas codificadas
    if let Some(csv) = detect_tabular_csv(content) {
        let eol = if csv.has_crlf { "\r\n" } else { "\n" };
        let mut blocks_decoded = 0usize;
        let mut decoded_rows = Vec::with_capacity(csv.rows.len());

        for r in &csv.rows {
            let mut new_row = r.clone();
            for &col in &csv.encoded_cols {
                let (dec, cnt) = expand_encoded_in_str(&r[col], true, 0);
                if cnt > 0 {
                    new_row[col] = dec;
                    blocks_decoded += cnt;
                }
            }
            decoded_rows.push(new_row);
        }

        if blocks_decoded == 0 {
            return DecodeInspectResult {
                expanded_text: content.to_string(),
                merged_chunks_text: None,
                blocks_decoded: 0,
                multipart_groups_merged: 0,
            };
        }

        let mut expanded_lines = Vec::with_capacity(decoded_rows.len() + 1);
        if let Some(ref h) = csv.header {
            expanded_lines.push(format_csv_row(h, csv.delimiter));
        }
        for r in &decoded_rows {
            expanded_lines.push(format_csv_row(r, csv.delimiter));
        }
        let mut expanded_text = expanded_lines.join(eol);
        if csv.trailing_newline {
            expanded_text.push_str(eol);
        }

        // Si existen grupos multiparte con offset numérico, construir también `merged_chunks_text`
        let mut merged_chunks_text = None;
        let mut multipart_groups_merged = 0usize;

        if let Some((group_col, idx_col, payload_col)) = csv.multipart_spec {
            // Agrupar filas que fueron decodificadas y pertenecen a grupos con >= 2 offsets distintos
            let mut group_indices: HashMap<String, Vec<(u64, usize)>> = HashMap::new();
            for (row_pos, orig_row) in csv.rows.iter().enumerate() {
                let was_decoded = decoded_rows[row_pos][payload_col] != orig_row[payload_col];
                if was_decoded && let Ok(idx_val) = orig_row[idx_col].trim().parse::<u64>() {
                    group_indices
                        .entry(orig_row[group_col].trim().to_string())
                        .or_default()
                        .push((idx_val, row_pos));
                }
            }

            let mergeable_groups: HashSet<String> = group_indices
                .iter()
                .filter_map(|(k, v)| {
                    let unique_offsets: HashSet<u64> = v.iter().map(|(off, _)| *off).collect();
                    if unique_offsets.len() >= 2 {
                        Some(k.clone())
                    } else {
                        None
                    }
                })
                .collect();

            if !mergeable_groups.is_empty() {
                let mut emitted_groups = HashSet::new();
                let mut merged_lines = Vec::new();
                if let Some(ref h) = csv.header {
                    merged_lines.push(format_csv_row(h, csv.delimiter));
                }

                for (row_pos, dec_row) in decoded_rows.iter().enumerate() {
                    let g_name = csv.rows[row_pos][group_col].trim();
                    let was_decoded = dec_row[payload_col] != csv.rows[row_pos][payload_col];
                    if was_decoded && mergeable_groups.contains(g_name) {
                        if emitted_groups.insert(g_name.to_string())
                            && let Some(entries) = group_indices.get(g_name)
                        {
                            let mut sorted = entries.clone();
                            sorted.sort_by_key(|&(off, pos)| (off, pos));

                            let chunk_pairs: Vec<(u64, String)> = sorted
                                .iter()
                                .map(|&(off, pos)| (off, decoded_rows[pos][payload_col].clone()))
                                .collect();

                            if is_contiguous_byte_offset_group(&chunk_pairs) {
                                // Flujo continuo partido por bytes (ej. web.config en zz_wc773x): unir en 1 fila
                                let mut base_row = decoded_rows[sorted[0].1].clone();
                                let mut full_payload = String::new();
                                for (_, chunk_str) in &chunk_pairs {
                                    full_payload.push_str(chunk_str);
                                }
                                base_row[idx_col] = sorted[0].0.to_string();
                                base_row[payload_col] = full_payload;
                                merged_lines.push(format_csv_row(&base_row, csv.delimiter));
                            } else {
                                // Registros numerados secuenciales (ej. [0], [1], [2] en zz_wc830x): emitir ordenados por idx
                                for &(_, pos) in &sorted {
                                    merged_lines
                                        .push(format_csv_row(&decoded_rows[pos], csv.delimiter));
                                }
                            }
                            multipart_groups_merged += 1;
                        }
                    } else {
                        merged_lines.push(format_csv_row(dec_row, csv.delimiter));
                    }
                }

                let mut m_text = merged_lines.join(eol);
                if csv.trailing_newline {
                    m_text.push_str(eol);
                }
                merged_chunks_text = Some(m_text);
            }
        }

        return DecodeInspectResult {
            expanded_text,
            merged_chunks_text,
            blocks_decoded,
            multipart_groups_merged,
        };
    }

    // Ruta 2: JSON válido (expandir valores de texto codificados preservando sintaxis JSON válida)
    if crate::format_adapters::json_adapter::JsonAdapter::is_valid_json(content)
        && let Ok(mut val) = serde_json::from_str::<serde_json::Value>(content)
    {
        let blocks_decoded = expand_json_value(&mut val);
        if blocks_decoded > 0
            && let Ok(pretty) = serde_json::to_string_pretty(&val)
        {
            return DecodeInspectResult {
                expanded_text: pretty,
                merged_chunks_text: None,
                blocks_decoded,
                multipart_groups_merged: 0,
            };
        }
    }

    // Ruta 3: Texto general / Logs / XML
    let (expanded_text, blocks_decoded) = expand_encoded_in_str(content, false, 0);
    DecodeInspectResult {
        expanded_text,
        merged_chunks_text: None,
        blocks_decoded,
        multipart_groups_merged: 0,
    }
}

/// Expande un contenido completo (incluyendo cargas codificadas Base64/Hex/URL y tramas SAP IDoc)
/// a texto plano legible sin aplicar ofuscación.
pub fn export_plain_decoded_text(
    content: &str,
    format: crate::models::FormatType,
    merge_chunks: bool,
) -> (String, usize, crate::models::FormatType) {
    use crate::format_adapters::idoc_adapter::{FIELD_SEP, IdocAdapter, RECORD_SEP, SEGMENT_SEP};
    use crate::models::FormatType;

    let actual_format = match format {
        FormatType::Auto => {
            crate::format_adapters::FormatOrchestrator::new().detect_format(content)
        }
        other => other,
    };

    if actual_format == FormatType::Idoc {
        let mut total_blocks = 0usize;
        let mut expanded_idoc = String::with_capacity(content.len());

        for (rec_idx, record) in content.split(RECORD_SEP).enumerate() {
            if rec_idx > 0 {
                expanded_idoc.push(RECORD_SEP);
            }
            for (seg_idx, segment) in record.split(SEGMENT_SEP).enumerate() {
                if seg_idx > 0 {
                    expanded_idoc.push(SEGMENT_SEP);
                }
                for (fld_idx, field) in segment.split(FIELD_SEP).enumerate() {
                    if fld_idx > 0 {
                        expanded_idoc.push(FIELD_SEP);
                    }
                    if field.is_empty() {
                        continue;
                    }
                    let inspected = inspect_and_expand_encoded(field);
                    total_blocks += inspected.blocks_decoded;
                    let val = if merge_chunks {
                        inspected
                            .merged_chunks_text
                            .unwrap_or(inspected.expanded_text)
                    } else {
                        inspected.expanded_text
                    };
                    expanded_idoc.push_str(&val);
                }
            }
        }

        let plain = IdocAdapter::format_plain_readable(&expanded_idoc);
        return (plain, total_blocks, actual_format);
    }

    let inspected = inspect_and_expand_encoded(content);
    let blocks_decoded = inspected.blocks_decoded;
    let plain = if merge_chunks {
        inspected
            .merged_chunks_text
            .unwrap_or(inspected.expanded_text)
    } else {
        inspected.expanded_text
    };
    (plain, blocks_decoded, actual_format)
}

fn expand_json_value(val: &mut serde_json::Value) -> usize {
    match val {
        serde_json::Value::String(s) => {
            let (dec, cnt) = expand_encoded_in_str(s, true, 0);
            if cnt > 0 {
                *s = dec;
            }
            cnt
        }
        serde_json::Value::Array(arr) => arr.iter_mut().map(expand_json_value).sum(),
        serde_json::Value::Object(map) => map.values_mut().map(expand_json_value).sum(),
        _ => 0,
    }
}

/// Extrae todos los fragmentos de texto decodificado (incluyendo flujos multiparte unidos sin escape CSV)
/// para que el escáner registre el 100% de entidades internas incluso si están codificadas o partidas entre dos filas CSV.
pub fn extract_decoded_views_for_scan(content: &str) -> Vec<String> {
    let inspected = inspect_and_expand_encoded(content);
    if inspected.blocks_decoded == 0 {
        return Vec::new();
    }
    let mut views = Vec::with_capacity(3);
    if let Some(merged) = inspected.merged_chunks_text {
        if merged.contains("\"\"") {
            views.push(merged.replace("\"\"", "\""));
        }
        views.push(merged);
    }
    if inspected.expanded_text.contains("\"\"") {
        views.push(inspected.expanded_text.replace("\"\"", "\""));
    }
    views.push(inspected.expanded_text);
    views
}

/// Aplica `inner_transform` sobre todas las cargas codificadas de `content` y las vuelve a codificar
/// preservando el formato exacto (incluyendo soporte biyectivo para flujos CSV multiparte por offset).
pub fn process_reencoded_payloads<F>(
    content: &str,
    reverse: bool,
    mut inner_transform: F,
) -> (String, usize, usize)
where
    F: FnMut(&str) -> (String, usize),
{
    if content.len() < 4 {
        return (content.to_string(), 0, 0);
    }

    // Ruta especializada para CSV tabular (preserva columnas, delimitadores, saltos de línea y soporta grupos por offset)
    if let Some(csv) = detect_tabular_csv(content) {
        let eol = if csv.has_crlf { "\r\n" } else { "\n" };
        let mut out_rows = csv.rows.clone();
        let mut total_replacements = 0usize;
        let mut encoded_blocks = 0usize;
        let mut handled_rows: HashSet<usize> = HashSet::new();

        // Si hay grupos multiparte continuos por offset de bytes (como `wc` en zz_wc773x), transformar el flujo unido
        // y volver a cortar por los offsets `idx` para proteger entidades partidas en frontera y restaurar 1:1.
        if let Some((group_col, idx_col, payload_col)) = csv.multipart_spec {
            let mut groups: BTreeMap<String, Vec<(u64, usize, String, EncodingKind)>> =
                BTreeMap::new();
            for (pos, row) in csv.rows.iter().enumerate() {
                if let Ok(off) = row[idx_col].trim().parse::<u64>()
                    && let Some((dec, kind)) = try_decode_token(row[payload_col].trim(), true)
                {
                    groups
                        .entry(row[group_col].trim().to_string())
                        .or_default()
                        .push((off, pos, dec, kind));
                }
            }

            for entries in groups.values_mut() {
                if entries.len() < 2 {
                    continue;
                }
                entries.sort_by_key(|(off, pos, _, _)| (*off, *pos));
                let pairs: Vec<(u64, String)> = entries
                    .iter()
                    .map(|(off, _, dec, _)| (*off, dec.clone()))
                    .collect();

                // En modo protección comprobamos offsets continuos; en modo restauración comprobamos que `idx` sea múltiplo de corte (> 1)
                let is_offset_stream = is_contiguous_byte_offset_group(&pairs)
                    || (reverse && pairs[0].0 == 0 && pairs[1].0 >= 64);

                if is_offset_stream {
                    let mut joined = String::new();
                    for (_, _, dec, _) in entries.iter() {
                        joined.push_str(dec);
                    }

                    let (nested_done, sub_repl, sub_blocks) =
                        transform_and_reencode_in_str(&joined, false, 1, &mut inner_transform);
                    let (transformed_stream, stream_repl) = inner_transform(&nested_done);
                    let group_repl = sub_repl + stream_repl;

                    if group_repl > 0 {
                        total_replacements += group_repl;
                        encoded_blocks += entries.len() + sub_blocks;

                        let slices = split_stream_for_chunks(&transformed_stream, entries, reverse);
                        for ((_, pos, _, kind), chunk_slice) in entries.iter().zip(slices.iter()) {
                            out_rows[*pos][payload_col] = encode_with_kind(chunk_slice, *kind);
                            handled_rows.insert(*pos);
                        }
                    } else {
                        for (_, pos, _, _) in entries.iter() {
                            handled_rows.insert(*pos);
                        }
                    }
                }
            }
        }

        for (pos, row) in csv.rows.iter().enumerate() {
            if handled_rows.contains(&pos) {
                continue;
            }
            for &col in &csv.encoded_cols {
                let (new_cell, repl, blk) =
                    transform_and_reencode_in_str(&row[col], true, 0, &mut inner_transform);
                if blk > 0 {
                    out_rows[pos][col] = new_cell;
                    total_replacements += repl;
                    encoded_blocks += blk;
                }
            }
        }

        if total_replacements == 0 {
            return (content.to_string(), 0, encoded_blocks);
        }

        let mut lines_out = Vec::with_capacity(out_rows.len() + 1);
        if let Some(ref h) = csv.header {
            lines_out.push(format_csv_row(h, csv.delimiter));
        }
        for r in &out_rows {
            lines_out.push(format_csv_row(r, csv.delimiter));
        }
        let mut final_csv = lines_out.join(eol);
        if csv.trailing_newline {
            final_csv.push_str(eol);
        }
        return (final_csv, total_replacements, encoded_blocks);
    }

    transform_and_reencode_in_str(content, false, 0, &mut inner_transform)
}

fn floor_char_boundary_safe(s: &str, mut idx: usize) -> usize {
    if idx >= s.len() {
        return s.len();
    }
    while idx > 0 && !s.is_char_boundary(idx) {
        idx -= 1;
    }
    idx
}

fn split_stream_for_chunks(
    stream: &str,
    entries: &[(u64, usize, String, EncodingKind)],
    reverse: bool,
) -> Vec<String> {
    let n = entries.len();
    if n == 0 {
        return Vec::new();
    }
    if n == 1 {
        return vec![stream.to_string()];
    }

    let mut result = Vec::with_capacity(n);
    if reverse {
        // En restauración, cortar exactamente en los offsets originales `idx` registrados en las filas
        for i in 0..n {
            let start_raw = usize::try_from(entries[i].0).unwrap_or(stream.len());
            let start = floor_char_boundary_safe(stream, start_raw);
            let end = if i + 1 < n {
                let end_raw = usize::try_from(entries[i + 1].0).unwrap_or(stream.len());
                floor_char_boundary_safe(stream, end_raw).max(start)
            } else {
                stream.len()
            };
            result.push(stream[start..end].to_string());
        }
    } else {
        // En protección, repartir proporcionalmente según el peso original de cada fragmento respetando UTF-8
        let orig_total: usize = entries
            .iter()
            .map(|(_, _, d, _)| d.len())
            .sum::<usize>()
            .max(1);
        let mut prev = 0usize;
        let mut acc_orig = 0usize;
        for (i, (_, _, dec, _)) in entries.iter().enumerate() {
            if i + 1 == n {
                result.push(stream[prev..].to_string());
            } else {
                acc_orig += dec.len();
                let target = (stream.len().saturating_mul(acc_orig)) / orig_total;
                let cut = floor_char_boundary_safe(stream, target).max(prev);
                result.push(stream[prev..cut].to_string());
                prev = cut;
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_csv_sample_decode_and_no_false_positives() {
        let sample = "id,kind,idx,v\n\
3,ping,0,UE9ORw==\n\
4,meta,0,YzpcaW5ldHB1Ylx3d3dyb290XGV4dHJhbmV0XHdlYi5jb25maWd8TEVOPTkwNzY=\n\
5,t1,0,QUFB\n\
8,t2,0,ABCDEFG\n";

        let res = inspect_and_expand_encoded(sample);
        assert_eq!(res.blocks_decoded, 3);
        assert!(res.expanded_text.contains("3,ping,0,PONG\n"));
        assert!(
            res.expanded_text
                .contains("4,meta,0,c:\\inetpub\\wwwroot\\extranet\\web.config|LEN=9076\n")
        );
        assert!(res.expanded_text.contains("5,t1,0,AAA\n"));
        assert!(res.expanded_text.contains("8,t2,0,ABCDEFG\n"));
    }

    #[test]
    fn test_binary_ciphervalue_is_preserved_untouched() {
        // CipherValue real (bytes binarios RSA/AES) no debe decodificarse a basura
        let cipher_b64 = "CF1ENzM0JT4u3EvtjmgRgOqt0Z/WbQOQZkyEN2pBC5UyWG7CO2hbCPK7ucsNEC/TgVV6TiwJBgoiOaCTYS1MkpJ1TOkNIPXs2GczQ0GuYpFAE2CH2fnY7/CTMpH1JmGBF+74DrC+ZFlAQwxFg4tqpa8gX943SDmz/Fjnr0fNq+hpPtLi0kGFzmRgjcop3/PLu+RrO4OS26Mr8G7Y7fO41KRFdsKXJvIWjWFOSaFhLBOPxIiv9MZcEreW7YqMjaybWLc/jXxuGyikuYi2Z8xf1P6G1diKJ7pjRgAzDEdeOQ7LzR1a5f1mOV6Sp5DaptIp7u85nOL9bSFHbZ8k55gnUg==";
        assert!(try_decode_token(cipher_b64, true).is_none());
    }

    #[test]
    fn test_powershell_utf16le_base64_roundtrip() {
        let cmd = "Invoke-WebRequest -Uri https://portal.interno.corp/api";
        let utf16_bytes: Vec<u8> = cmd.encode_utf16().flat_map(u16::to_le_bytes).collect();
        let encoded = encode_base64(&utf16_bytes, false, true);

        let (decoded, kind) = try_decode_token(&encoded, false).expect("Debe detectar UTF-16LE");
        assert_eq!(decoded, cmd);
        assert_eq!(
            kind,
            EncodingKind::Base64Std {
                padded: true,
                utf16le: true
            }
        );
        assert_eq!(encode_with_kind(&decoded, kind), encoded);
    }

    #[test]
    fn test_hex_and_nested_url_encoding() {
        let url_inner = "user%3Dadmin%40empresa.es%26host%3D10.20.30.40";
        let outer_b64 = encode_base64(url_inner.as_bytes(), false, true);
        let (expanded, count) = expand_encoded_in_str(&outer_b64, true, 0);
        assert_eq!(count, 2);
        assert_eq!(expanded, "user=admin@empresa.es&host=10.20.30.40");

        let hex_path = encode_hex_bytes(b"C:\\inetpub\\wwwroot\\extranet\\web.config", true, false);
        let (dec_hex, h_kind) = try_decode_token(&hex_path, false).expect("Debe detectar Hex");
        assert_eq!(dec_hex, "C:\\inetpub\\wwwroot\\extranet\\web.config");
        assert_eq!(encode_with_kind(&dec_hex, h_kind), hex_path);
    }

    #[test]
    fn test_common_words_never_false_positive_as_base64() {
        for word in [
            "Info", "Type", "Data", "User", "ping", "meta", "true", "null", "12345678",
        ] {
            assert!(
                try_decode_token(word, true).is_none(),
                "La palabra común '{word}' no debe decodificarse como Base64"
            );
        }
    }

    #[test]
    fn test_export_plain_decoded_text_idoc_and_base64() {
        let b64_payload = encode_base64(b"admin@empresa.es IP 10.20.30.40", false, true);
        let idoc = format!("EDI_DC40\x01{b64_payload}\x02E2EDK01\x01EUR\x03REC2\x01OK");
        let (plain, blocks, fmt) =
            export_plain_decoded_text(&idoc, crate::models::FormatType::Auto, false);
        assert_eq!(fmt, crate::models::FormatType::Idoc);
        assert_eq!(blocks, 1);
        assert_eq!(
            plain,
            "EDI_DC40 | admin@empresa.es IP 10.20.30.40\nE2EDK01 | EUR\n\nREC2 | OK"
        );
    }
}
