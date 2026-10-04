//! Detectores de identidad personal: DNI/NIE con letra de control real, Tarjetas con Luhn y Teléfonos.

use crate::models::DetectorType;
use rand::Rng;
use regex::Regex;
use std::sync::LazyLock;

const DNI_LETTERS: &[u8] = b"TRWAGMYFPDXBNJZSQVHLCKE";

static DNI_NIE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:([XYZxyz])\s*(\d{7})|(\d{8}))\s*([A-Za-z])\b")
        .expect("Regex DNI/NIE inválida")
});

static CREDIT_CARD_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(?:\d{4}[-\s]?){3}\d{4}\b|\b\d{13,19}\b").expect("Regex Credit Card inválida")
});

static PHONE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(?:\+34[\s.-]?)?[6789]\d{2}[\s.-]?\d{3}[\s.-]?\d{3}\b")
        .expect("Regex Phone inválida")
});

#[derive(Default)]
pub struct SpanishDniNieDetector;

impl SpanishDniNieDetector {
    pub fn new() -> Self {
        Self
    }

    pub fn detector_type(&self) -> DetectorType {
        DetectorType::SpanishDniNie
    }

    /// Calcula la letra de control oficial del Ministerio del Interior para un número de DNI o NIE.
    pub fn calculate_control_letter(num: u32) -> char {
        let idx = (num % 23) as usize;
        DNI_LETTERS[idx] as char
    }

    /// Valida si un DNI/NIE tiene un checksum matemáticamente correcto.
    pub fn is_valid_dni_nie(raw: &str) -> bool {
        let cleaned: String = raw.chars().filter(char::is_ascii_alphanumeric).collect();
        if cleaned.len() != 9 {
            return false;
        }

        let first = cleaned.chars().next().unwrap_or(' ');
        let last = cleaned.chars().last().unwrap_or(' ').to_ascii_uppercase();

        let num_str: String = match first.to_ascii_uppercase() {
            'X' => format!("0{}", &cleaned[1..8]),
            'Y' => format!("1{}", &cleaned[1..8]),
            'Z' => format!("2{}", &cleaned[1..8]),
            _ if first.is_ascii_digit() => cleaned[0..8].to_string(),
            _ => return false,
        };

        if let Ok(num) = num_str.parse::<u32>() {
            Self::calculate_control_letter(num) == last
        } else {
            false
        }
    }

    pub fn find_matches<'a>(&self, text: &'a str) -> Vec<(usize, usize, &'a str)> {
        DNI_NIE_REGEX
            .find_iter(text)
            .filter(|m| Self::is_valid_dni_nie(m.as_str()))
            .map(|m| (m.start(), m.end(), m.as_str()))
            .collect()
    }

    /// Genera un DNI sintético válido con letra de control matemáticamente correcta.
    pub fn generate_pseudonym(&self, original: &str) -> String {
        let mut rng = rand::thread_rng();
        let num: u32 = rng.gen_range(10_000_000..99_999_999);
        let letter = Self::calculate_control_letter(num);
        let is_upper = original
            .chars()
            .last()
            .is_none_or(|c| c.is_ascii_uppercase());
        let out_letter = if is_upper {
            letter
        } else {
            letter.to_ascii_lowercase()
        };
        format!("{num:08}{out_letter}")
    }
}

#[derive(Default)]
pub struct CreditCardDetector;

impl CreditCardDetector {
    pub fn new() -> Self {
        Self
    }

    pub fn detector_type(&self) -> DetectorType {
        DetectorType::CreditCard
    }

    /// Valida si un prefijo y longitud corresponden a una red de tarjetas conocida (IIN/BIN).
    pub fn is_valid_iin(digits: &[u32]) -> bool {
        let len = digits.len();
        if !(13..=19).contains(&len) {
            return false;
        }

        let first = digits[0];
        let first2 = if len >= 2 {
            digits[0] * 10 + digits[1]
        } else {
            0
        };
        let first4 = if len >= 4 {
            digits[0] * 1000 + digits[1] * 100 + digits[2] * 10 + digits[3]
        } else {
            0
        };
        let first6 = if len >= 6 {
            digits[0] * 100_000
                + digits[1] * 10_000
                + digits[2] * 1000
                + digits[3] * 100
                + digits[4] * 10
                + digits[5]
        } else {
            0
        };

        // Visa: 4 (longitud 13, 16, 19)
        if first == 4 && (len == 13 || len == 16 || len == 19) {
            return true;
        }

        // Mastercard: 51..=55, 2221..=2720 (longitud 16)
        if len == 16 {
            if (51..=55).contains(&first2) {
                return true;
            }
            if (2221..=2720).contains(&first4) {
                return true;
            }
        }

        // American Express: 34, 37 (longitud 15)
        if (first2 == 34 || first2 == 37) && len == 15 {
            return true;
        }

        // Discover: 6011, 644..=649, 65, 622126..=622925 (longitud 16, 19)
        if len == 16 || len == 19 {
            if first4 == 6011 || (644..=649).contains(&first4) || first2 == 65 {
                return true;
            }
            if (622126..=622925).contains(&first6) {
                return true;
            }
        }

        // Diners Club / Carte Blanche: 300..=305, 36, 38 (longitud 14..=19)
        if (300..=305).contains(&first4) || first2 == 36 || first2 == 38 {
            return true;
        }

        // JCB: 3528..=3589 (longitud 16..=19)
        if (3528..=3589).contains(&first4) && len >= 16 {
            return true;
        }

        // Maestro: 50, 56..=58, 6 (longitud 12..=19)
        if first2 == 50 || (56..=58).contains(&first2) {
            return true;
        }

        // UnionPay: 62 (longitud 16..=19)
        if first2 == 62 && len >= 16 {
            return true;
        }

        // Mir: 2200..=2204 (longitud 16)
        if (2200..=2204).contains(&first4) && len == 16 {
            return true;
        }

        false
    }

    /// Algoritmo de Luhn (Módulo 10) para validación de tarjetas de crédito con comprobación de IIN.
    pub fn is_luhn_valid(number: &str) -> bool {
        let digits: Vec<u32> = number.chars().filter_map(|c| c.to_digit(10)).collect();
        if !(13..=19).contains(&digits.len()) {
            return false;
        }

        // Descartar secuencias homogéneas (como 0000000000000000)
        if digits.iter().all(|&d| d == digits[0]) {
            return false;
        }

        if !Self::is_valid_iin(&digits) {
            return false;
        }

        let mut sum = 0;
        let mut alternate = false;
        for &digit in digits.iter().rev() {
            if alternate {
                let d = digit * 2;
                sum += if d > 9 { d - 9 } else { d };
            } else {
                sum += digit;
            }
            alternate = !alternate;
        }
        sum % 10 == 0
    }

    /// Comprueba que la coincidencia no sea parte de un hash, nombre de archivo o identificador contiguo.
    pub fn is_isolated_credit_card(text: &str, start: usize, end: usize) -> bool {
        // 1. Carácter inmediatamente anterior
        if start > 0 {
            let prev_char = text[..start].chars().last().unwrap_or(' ');
            if prev_char.is_ascii_alphanumeric() || prev_char == '_' {
                return false;
            }
            // Si está precedido por delimitadores comunes de rutas/hashes (-, ., :, /, \)
            if ['-', '.', ':', '/', '\\'].contains(&prev_char) && start > 1 {
                let before_delim = text[..start - prev_char.len_utf8()]
                    .chars()
                    .last()
                    .unwrap_or(' ');
                if before_delim.is_ascii_alphanumeric() {
                    return false;
                }
            }
        }

        // 2. Carácter inmediatamente posterior
        if end < text.len() {
            let next_char = text[end..].chars().next().unwrap_or(' ');
            if next_char.is_ascii_alphanumeric() || next_char == '_' {
                return false;
            }
            // Si está seguido por delimitadores comunes de rutas/hashes (-, ., :, /, \)
            if ['-', '.', ':', '/', '\\'].contains(&next_char)
                && end + next_char.len_utf8() < text.len()
            {
                let after_delim = text[end + next_char.len_utf8()..]
                    .chars()
                    .next()
                    .unwrap_or(' ');
                if after_delim.is_ascii_alphanumeric() {
                    return false;
                }
            }
        }

        // 3. Inspeccionar el token delimitado circundante
        let token_start = text[..start]
            .rfind(|c: char| {
                c.is_whitespace()
                    || [
                        '"', '\'', '<', '>', '(', ')', '{', '}', '[', ']', ',', ';', '\r', '\n',
                    ]
                    .contains(&c)
            })
            .map_or(0, |idx| idx + 1);
        let token_end = text[end..]
            .find(|c: char| {
                c.is_whitespace()
                    || [
                        '"', '\'', '<', '>', '(', ')', '{', '}', '[', ']', ',', ';', '\r', '\n',
                    ]
                    .contains(&c)
            })
            .map_or(text.len(), |idx| end + idx);

        let surrounding = &text[token_start..token_end];
        // Si el token circundante contiene letras alfabéticas o de hash, no es una tarjeta real
        if surrounding.chars().any(|c| c.is_ascii_alphabetic()) {
            return false;
        }

        // 4. Comprobar si el contexto previo indica un hash o checksum conocido
        let ctx_start = {
            let mut idx = start.saturating_sub(40);
            while idx > 0 && !text.is_char_boundary(idx) {
                idx -= 1;
            }
            idx
        };
        let ctx_prefix = text[ctx_start..start].to_lowercase();
        if ctx_prefix.contains("sha")
            || ctx_prefix.contains("md5")
            || ctx_prefix.contains("hash")
            || ctx_prefix.contains("checksum")
            || ctx_prefix.contains("digest")
            || ctx_prefix.contains("etag")
            || ctx_prefix.contains("commit")
        {
            return false;
        }

        true
    }

    pub fn find_matches<'a>(&self, text: &'a str) -> Vec<(usize, usize, &'a str)> {
        CREDIT_CARD_REGEX
            .find_iter(text)
            .filter(|m| {
                Self::is_luhn_valid(m.as_str())
                    && Self::is_isolated_credit_card(text, m.start(), m.end())
            })
            .map(|m| (m.start(), m.end(), m.as_str()))
            .collect()
    }

    /// Genera un número de tarjeta de prueba sintético de 16 dígitos que cumple con el algoritmo de Luhn.
    pub fn generate_pseudonym(&self, original: &str) -> String {
        let mut rng = rand::thread_rng();
        let mut digits: Vec<u32> = Vec::with_capacity(16);
        // Prefijo Visa de pruebas (4)
        digits.push(4);
        for _ in 0..14 {
            digits.push(rng.gen_range(0..10));
        }

        // Calcular dígito de control de Luhn
        let mut sum = 0;
        for (i, &digit) in digits.iter().enumerate() {
            if i % 2 == 0 {
                let d = digit * 2;
                sum += if d > 9 { d - 9 } else { d };
            } else {
                sum += digit;
            }
        }
        let check_digit = (10 - (sum % 10)) % 10;
        digits.push(check_digit);

        if original.contains('-') {
            format!(
                "{}{}{}{}-{}{}{}{}-{}{}{}{}-{}{}{}{}",
                digits[0],
                digits[1],
                digits[2],
                digits[3],
                digits[4],
                digits[5],
                digits[6],
                digits[7],
                digits[8],
                digits[9],
                digits[10],
                digits[11],
                digits[12],
                digits[13],
                digits[14],
                digits[15]
            )
        } else if original.contains(' ') {
            format!(
                "{}{}{}{} {}{}{}{} {}{}{}{} {}{}{}{}",
                digits[0],
                digits[1],
                digits[2],
                digits[3],
                digits[4],
                digits[5],
                digits[6],
                digits[7],
                digits[8],
                digits[9],
                digits[10],
                digits[11],
                digits[12],
                digits[13],
                digits[14],
                digits[15]
            )
        } else {
            digits
                .iter()
                .map(std::string::ToString::to_string)
                .collect()
        }
    }
}

#[derive(Default)]
pub struct TelephoneDetector;

impl TelephoneDetector {
    pub fn new() -> Self {
        Self
    }

    pub fn detector_type(&self) -> DetectorType {
        DetectorType::Telephone
    }

    /// Comprueba que la coincidencia no forme parte de un SID de Windows ni sea un falso positivo común en JSON (como IDs o timestamps).
    fn is_false_positive_context(full_text: &str, start: usize, end: usize) -> bool {
        // Regla 1: Descartar si es parte de un SID de Windows
        let prefix_start = {
            let mut idx = start.saturating_sub(10);
            while idx > 0 && !full_text.is_char_boundary(idx) {
                idx -= 1;
            }
            idx
        };
        let prefix = &full_text[prefix_start..start].to_lowercase();
        if prefix.contains("s-1-") || prefix.contains("sid") {
            return true;
        }
        let suffix_end = {
            let mut idx = (end + 10).min(full_text.len());
            while idx < full_text.len() && !full_text.is_char_boundary(idx) {
                idx += 1;
            }
            idx
        };
        let suffix = &full_text[end..suffix_end];
        if suffix.starts_with('-') {
            return true;
        }

        // Regla 2: Descartar si el contexto indica un campo numérico (ID, Timestamp) en estructurados como JSON/YAML
        let json_prefix_start = {
            let mut idx = start.saturating_sub(30);
            while idx > 0 && !full_text.is_char_boundary(idx) {
                idx -= 1;
            }
            idx
        };
        let json_prefix = &full_text[json_prefix_start..start].to_lowercase();
        if json_prefix.contains("id\"")
            || json_prefix.contains("id'")
            || json_prefix.contains("id:")
            || json_prefix.contains("id=")
            || json_prefix.contains("timestamp")
            || json_prefix.contains("date")
            || json_prefix.contains("time")
            || json_prefix.contains("created")
            || json_prefix.contains("updated")
            || json_prefix.contains("size")
            || json_prefix.contains("length")
            || json_prefix.contains("count")
        {
            return true;
        }

        // Falso positivo si es una fracción decimal (ej. .944430319)
        if json_prefix.ends_with('.') {
            return true;
        }

        false
    }

    pub fn find_matches<'a>(&self, text: &'a str) -> Vec<(usize, usize, &'a str)> {
        PHONE_REGEX
            .find_iter(text)
            .filter(|m| !Self::is_false_positive_context(text, m.start(), m.end()))
            .map(|m| (m.start(), m.end(), m.as_str()))
            .collect()
    }

    /// Genera un número de teléfono sintético respetando exactamente el formato, espacios y delimitadores del original.
    pub fn generate_pseudonym(&self, original: &str) -> String {
        let mut rng = rand::thread_rng();

        // 1. Identificar si tiene prefijo +34
        let (prefix, body) = if let Some(stripped) = original.strip_prefix("+34") {
            ("+34", stripped)
        } else {
            ("", original)
        };

        // 2. Determinar primer dígito del cuerpo (mantener tipo móvil/fijo)
        let first_digit_orig = body.chars().find(char::is_ascii_digit).unwrap_or('6');
        let first_digit = match first_digit_orig {
            '6' | '7' => {
                if rng.gen_bool(0.5) {
                    '6'
                } else {
                    '7'
                }
            }
            '8' | '9' => {
                if rng.gen_bool(0.5) {
                    '8'
                } else {
                    '9'
                }
            }
            _ => '6',
        };

        let mut new_digits = Vec::with_capacity(9);
        new_digits.push(first_digit);
        for _ in 1..9 {
            let d: u32 = rng.gen_range(0..=9);
            new_digits.push(char::from_digit(d, 10).unwrap_or('0'));
        }

        let mut digit_iter = new_digits.into_iter();
        let mut result = String::with_capacity(original.len());
        result.push_str(prefix);

        for c in body.chars() {
            if c.is_ascii_digit() {
                if let Some(next_digit) = digit_iter.next() {
                    result.push(next_digit);
                } else {
                    result.push(c);
                }
            } else {
                result.push(c);
            }
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spanish_dni_nie_detection_and_validation() {
        let detector = SpanishDniNieDetector::new();
        // 12345678Z (8 % 23 = 14 => Z)
        let matches =
            detector.find_matches("El DNI 12345678Z y NIE X1234567L son válidos pero 00000000A no");
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].2, "12345678Z");
        assert_eq!(matches[1].2, "X1234567L");

        let pseudo = detector.generate_pseudonym("12345678Z");
        let matches_pseudo = detector.find_matches(&pseudo);
        assert_eq!(
            matches_pseudo.len(),
            1,
            "El seudónimo generado debe ser un DNI válido"
        );
    }

    #[test]
    fn test_credit_card_luhn_and_heuristic() {
        let detector = CreditCardDetector::new();
        // Visa de prueba válida de 16 dígitos con Luhn válido
        let matches = detector.find_matches("Tarjeta Visa 4532-1234-5678-9014 activa");
        assert_eq!(matches.len(), 1);
        let pseudo = detector.generate_pseudonym("4532-1234-5678-9014");
        assert_eq!(pseudo.len(), 19); // Mantiene espaciado y guiones
    }

    #[test]
    fn test_telephone_detector_format_preservation() {
        let detector = TelephoneDetector::new();
        let matches = detector.find_matches("Llamar al +34 694 185 579 o al 912345678");
        assert_eq!(matches.len(), 2);

        let pseudo_intl = detector.generate_pseudonym("+34 694 185 579");
        assert!(pseudo_intl.starts_with("+34 "));
        assert_eq!(pseudo_intl.len(), "+34 694 185 579".len());
    }
}
