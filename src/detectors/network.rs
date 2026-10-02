//! Detectores de redes, direcciones IP, dominios, hostnames y correos electrónicos.

use crate::models::DetectorType;
use once_cell::sync::Lazy;
use rand::Rng;
use regex::Regex;
use std::net::Ipv4Addr;

static IPV4_REGEX: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b(?:[0-9]{1,3}\.){3}[0-9]{1,3}\b").expect("Regex IPv4 inválida"));

static IPV6_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(
        r"(?i)\b(?:[0-9a-f]{1,4}:){7}[0-9a-f]{1,4}\b|\b(?:[0-9a-f]{1,4}:){1,7}:[0-9a-f]{1,4}\b",
    )
    .expect("Regex IPv6 inválida")
});

static EMAIL_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b[a-z0-9._%+-]+@[a-z0-9.-]+\.[a-z]{2,}\b").expect("Regex Email inválida")
});

static DOMAIN_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(?:[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?\.)+(?:com|org|net|edu|gov|io|es|eu|local|internal|corp)\b")
        .expect("Regex Domain inválida")
});

static HOSTNAME_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)\b(?:host|srv|server|dc|ws|node|app)-?[a-z0-9]{2,10}\b")
        .expect("Regex Hostname inválida")
});

#[derive(Default)]
pub struct IPv4Detector;

impl IPv4Detector {
    pub fn new() -> Self {
        Self
    }

    pub fn detector_type(&self) -> DetectorType {
        DetectorType::IPv4
    }

    pub fn find_matches<'a>(&self, text: &'a str) -> Vec<(usize, usize, &'a str)> {
        IPV4_REGEX
            .find_iter(text)
            .filter_map(|m| {
                if m.as_str().parse::<Ipv4Addr>().is_ok() {
                    Some((m.start(), m.end(), m.as_str()))
                } else {
                    None
                }
            })
            .collect()
    }

    /// Genera una dirección IP en el rango reservado de pruebas de la IETF / RFC 5737 (TEST-NET-1: 192.0.2.0/24).
    pub fn generate_pseudonym(&self, _original: &str) -> String {
        let mut rng = rand::thread_rng();
        let octet: u8 = rng.gen_range(1..254);
        format!("192.0.2.{}", octet)
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
        format!("2001:db8:85a3::{:x}:{:x}", a, b)
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
        let id: u32 = rng.gen_range(1000..9999);
        format!("user_{}@example.com", id)
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
        let id: u32 = rng.gen_range(100..999);
        format!("service{}.example.com", id)
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
        let id: u32 = rng.gen_range(100..999);
        let prefix = if original.to_lowercase().starts_with("srv") {
            "srv"
        } else {
            "host"
        };
        format!("{}-anon-{}", prefix, id)
    }
}
