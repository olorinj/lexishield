//! Detectores de redes, direcciones IP, dominios, hostnames y correos electrónicos.

use crate::models::DetectorType;
use rand::Rng;
use regex::Regex;
use std::net::Ipv4Addr;
use std::sync::LazyLock;

static IPV4_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b(?:[0-9]{1,3}\\?\.){3}[0-9]{1,3}\b").expect("Regex IPv4 inválida")
});

static IPV6_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\b(?:[0-9a-f]{1,4}:){7}[0-9a-f]{1,4}\b|\b(?:[0-9a-f]{1,4}:){1,7}:[0-9a-f]{1,4}\b",
    )
    .expect("Regex IPv6 inválida")
});

static EMAIL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b[a-z0-9._%+-]+@[a-z0-9.-]+\.[a-z]{2,}\b").expect("Regex Email inválida")
});

static DOMAIN_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?\.)+(?:com|org|net|edu|gov|io|es|eu|local|internal|corp)\b")
        .expect("Regex Domain inválida")
});

static HOSTNAME_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:host|srv|server|dc|ws|node|app)-?[a-z0-9]{2,10}\b")
        .expect("Regex Hostname inválida")
});

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

#[derive(Default)]
pub struct IPv4Detector {
    subnet_map: RefCell<HashMap<String, String>>,
    used_subnets: RefCell<HashSet<String>>,
}

impl IPv4Detector {
    pub fn new() -> Self {
        Self {
            subnet_map: RefCell::new(HashMap::new()),
            used_subnets: RefCell::new(HashSet::new()),
        }
    }

    pub fn detector_type(&self) -> DetectorType {
        DetectorType::IPv4
    }

    pub fn find_matches<'a>(&self, text: &'a str) -> Vec<(usize, usize, &'a str)> {
        IPV4_REGEX
            .find_iter(text)
            .filter_map(|m| {
                let clean_str = m.as_str().replace('\\', "");
                if clean_str.parse::<Ipv4Addr>().is_ok() {
                    Some((m.start(), m.end(), m.as_str()))
                } else {
                    None
                }
            })
            .collect()
    }

    fn is_private(ip: Ipv4Addr) -> bool {
        let octets = ip.octets();
        octets[0] == 10
            || (octets[0] == 172 && (16..=31).contains(&octets[1]))
            || (octets[0] == 192 && octets[1] == 168)
    }

    /// Genera una dirección IP. Si es privada, mantiene coherencia en la subred /24.
    pub fn generate_pseudonym(&self, original: &str) -> String {
        let mut rng = rand::thread_rng();
        let clean_original = original.replace('\\', "");
        let has_escapes = original.contains('\\');

        let result = if let Ok(ip) = clean_original.parse::<Ipv4Addr>() {
            let octets = ip.octets();
            let is_priv = Self::is_private(ip);
            let subnet_key = format!("{}.{}.{}", octets[0], octets[1], octets[2]);

            let mut map = self.subnet_map.borrow_mut();
            let mut used = self.used_subnets.borrow_mut();

            // Registrar subred original para que no se asigne como seudónimo a otra
            used.insert(subnet_key.clone());

            let new_subnet = map.entry(subnet_key).or_insert_with(|| {
                loop {
                    let cand = if is_priv {
                        // Subredes privadas en 10.x.y.0/24 (65.536 subredes disponibles)
                        let b: u8 = rng.gen_range(0..=255);
                        let c: u8 = rng.gen_range(0..=255);
                        format!("10.{b}.{c}")
                    } else {
                        // Subredes públicas seguras en espacios no reservados (millones de subredes disponibles)
                        let a: u8 = rng.gen_range(11..=220);
                        if a == 127 || a == 169 || a == 172 || a == 192 {
                            continue;
                        }
                        let b: u8 = rng.gen_range(1..=254);
                        let c: u8 = rng.gen_range(1..=254);
                        format!("{a}.{b}.{c}")
                    };

                    if !used.contains(&cand) {
                        used.insert(cand.clone());
                        break cand;
                    }
                }
            });

            format!("{}.{}", new_subnet, octets[3])
        } else {
            let a: u8 = rng.gen_range(11..=220);
            let b: u8 = rng.gen_range(1..=254);
            let c: u8 = rng.gen_range(1..=254);
            let d: u8 = rng.gen_range(1..=254);
            format!("{a}.{b}.{c}.{d}")
        };

        if has_escapes {
            result.replace('.', "\\.")
        } else {
            result
        }
    }
}

#[derive(Default)]
pub struct IPv6Detector;

impl IPv6Detector {
    pub fn new() -> Self {
        Self
    }

    pub fn detector_type(&self) -> DetectorType {
        DetectorType::IPv6
    }

    pub fn find_matches<'a>(&self, text: &'a str) -> Vec<(usize, usize, &'a str)> {
        IPV6_REGEX
            .find_iter(text)
            .map(|m| (m.start(), m.end(), m.as_str()))
            .collect()
    }

    /// Genera una dirección IPv6 en el prefijo reservado de documentación RFC 3849 (2001:db8::/32).
    pub fn generate_pseudonym(&self, _original: &str) -> String {
        let mut rng = rand::thread_rng();
        let a: u16 = rng.gen_range(0x1000..0xFFFF);
        let b: u16 = rng.gen_range(0x1000..0xFFFF);
        format!("2001:db8:85a3::{a:x}:{b:x}")
    }
}

#[derive(Default)]
pub struct EmailDetector;

impl EmailDetector {
    pub fn new() -> Self {
        Self
    }

    pub fn detector_type(&self) -> DetectorType {
        DetectorType::Email
    }

    pub fn find_matches<'a>(&self, text: &'a str) -> Vec<(usize, usize, &'a str)> {
        EMAIL_REGEX
            .find_iter(text)
            .map(|m| (m.start(), m.end(), m.as_str()))
            .collect()
    }

    /// Genera un correo seguro bajo el dominio reservado RFC 2606 (example.com).
    pub fn generate_pseudonym(&self, _original: &str) -> String {
        let mut rng = rand::thread_rng();
        let id: u64 = rng.gen_range(100_000..999_999_999);
        format!("user_{id}@example.com")
    }
}

#[derive(Default)]
pub struct DomainDetector;

impl DomainDetector {
    pub fn new() -> Self {
        Self
    }

    pub fn detector_type(&self) -> DetectorType {
        DetectorType::DomainFqdn
    }

    pub fn find_matches<'a>(&self, text: &'a str) -> Vec<(usize, usize, &'a str)> {
        DOMAIN_REGEX
            .find_iter(text)
            .map(|m| (m.start(), m.end(), m.as_str()))
            .collect()
    }

    /// Genera un dominio RFC 2606 seguro (.example.com).
    pub fn generate_pseudonym(&self, _original: &str) -> String {
        let mut rng = rand::thread_rng();
        let id: u64 = rng.gen_range(100_000..999_999_999);
        format!("service{id}.example.com")
    }
}

#[derive(Default)]
pub struct HostnameDetector;

impl HostnameDetector {
    pub fn new() -> Self {
        Self
    }

    pub fn detector_type(&self) -> DetectorType {
        DetectorType::Hostname
    }

    pub fn find_matches<'a>(&self, text: &'a str) -> Vec<(usize, usize, &'a str)> {
        HOSTNAME_REGEX
            .find_iter(text)
            .map(|m| (m.start(), m.end(), m.as_str()))
            .collect()
    }

    pub fn generate_pseudonym(&self, original: &str) -> String {
        let mut rng = rand::thread_rng();
        let id: u64 = rng.gen_range(100_000..999_999_999);
        let prefix = if original.to_lowercase().starts_with("srv") {
            "srv"
        } else {
            "host"
        };
        format!("{prefix}-anon-{id}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipv4_detection_and_generation() {
        let detector = IPv4Detector::new();
        let matches = detector
            .find_matches("Servidor en 192.168.1.50 y router en 10.0.0.1 y falso 999.999.999.999");
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].2, "192.168.1.50");
        assert_eq!(matches[1].2, "10.0.0.1");

        let pseudo = detector.generate_pseudonym("192.168.1.50");
        assert!(pseudo.parse::<Ipv4Addr>().is_ok());
    }

    #[test]
    fn test_ipv6_detection_and_generation() {
        let detector = IPv6Detector::new();
        let matches =
            detector.find_matches("Direccion 2001:0db8:85a3:0000:0000:8a2e:0370:7334 activa");
        assert_eq!(matches.len(), 1);
        let pseudo = detector.generate_pseudonym(matches[0].2);
        assert!(pseudo.contains(':'));
    }

    #[test]
    #[allow(clippy::case_sensitive_file_extension_comparisons)]
    fn test_email_detection_and_generation() {
        let detector = EmailDetector::new();
        let matches = detector.find_matches("Escribir a admin.soporte@dominio.com o user@test.org");
        assert_eq!(matches.len(), 2);
        let pseudo = detector.generate_pseudonym("admin.soporte@dominio.com");
        assert!(pseudo.contains('@'));
        assert!(pseudo.ends_with(".com") || pseudo.ends_with(".org") || pseudo.ends_with(".net"));
    }

    #[test]
    fn test_domain_detection_and_generation() {
        let detector = DomainDetector::new();
        let matches = detector.find_matches("Visita portal.interno.corp o web.empresa.es");
        assert_eq!(matches.len(), 2);
        let pseudo = detector.generate_pseudonym("portal.interno.corp");
        assert!(pseudo.contains("example.com"));
    }

    #[test]
    fn test_hostname_detection_and_generation() {
        let detector = HostnameDetector::new();
        let matches = detector.find_matches("Servidores srv-prod01 y host-db02");
        assert_eq!(matches.len(), 2);
        let pseudo = detector.generate_pseudonym("srv-prod01");
        assert!(pseudo.starts_with("srv-anon-"));
    }
}
