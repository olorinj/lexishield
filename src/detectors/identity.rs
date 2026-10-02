//! Detectores de identidad personal: DNI/NIE con letra de control real, Tarjetas con Luhn y Teléfonos.

use crate::models::DetectorType;
use once_cell::sync::Lazy;
use rand::Rng;
use regex::Regex;

const DNI_LETTERS: &[u8] = b"TRWAGMYFPDXBNJZSQVHLCKE";

static DNI_NIE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(?:([XYZxyz])\s*(\d{7})|(\d{8}))\s*([A-Za-z])\b")
        .expect("Regex DNI/NIE inválida")
});

static CREDIT_CARD_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b(?:\d{4}[-\s]?){3}\d{4}\b|\b\d{13,19}\b").expect("Regex Credit Card inválida")
});

static PHONE_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?:\+34[\s.-]?)?(?:[6789]\d{2}[\s.-]?\d{3}[\s.-]?\d{3})\b")
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
        let cleaned: String = raw.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
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
            .map(|c| c.is_ascii_uppercase())
            .unwrap_or(true);
        let out_letter = if is_upper {
            letter
        } else {
            letter.to_ascii_lowercase()
        };
        format!("{:08}{}", num, out_letter)
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

    /// Algoritmo de Luhn (Módulo 10) para validación de tarjetas de crédito.
    pub fn is_luhn_valid(number: &str) -> bool {
        let digits: Vec<u32> = number.chars().filter_map(|c| c.to_digit(10)).collect();
        if digits.len() < 13 || digits.len() > 19 {
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

    pub fn find_matches<'a>(&self, text: &'a str) -> Vec<(usize, usize, &'a str)> {
        CREDIT_CARD_REGEX
            .find_iter(text)
            .filter(|m| Self::is_luhn_valid(m.as_str()))
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
            digits.iter().map(|d| d.to_string()).collect()
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

    /// Comprueba que la coincidencia no forme parte de un SID de Windows (regla estricta contra falsos positivos).
    fn is_false_positive_sid(full_text: &str, start: usize, end: usize) -> bool {
        let prefix_start = start.saturating_sub(10);
        let prefix = &full_text[prefix_start..start];
        if prefix.contains("S-1-") || prefix.contains("sid") || prefix.contains("Sid") {
            return true;
        }
        let suffix_end = (end + 10).min(full_text.len());
        let suffix = &full_text[end..suffix_end];
        if suffix.starts_with('-') {
            return true;
        }
        false
    }

    pub fn find_matches<'a>(&self, text: &'a str) -> Vec<(usize, usize, &'a str)> {
        PHONE_REGEX
            .find_iter(text)
            .filter(|m| !Self::is_false_positive_sid(text, m.start(), m.end()))
            .map(|m| (m.start(), m.end(), m.as_str()))
            .collect()
    }

    pub fn generate_pseudonym(&self, original: &str) -> String {
        let mut rng = rand::thread_rng();
        let p1: u32 = rng.gen_range(600..699);
        let p2: u32 = rng.gen_range(100..999);
        let p3: u32 = rng.gen_range(100..999);
        if original.starts_with("+34") {
            format!("+34 {} {} {}", p1, p2, p3)
        } else if original.contains('-') {
            format!("{}-{}-{}", p1, p2, p3)
        } else if original.contains(' ') {
            format!("{} {} {}", p1, p2, p3)
        } else {
            format!("{}{}{}", p1, p2, p3)
        }
    }
}
