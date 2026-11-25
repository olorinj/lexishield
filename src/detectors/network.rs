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
    Regex::new(r"(?i)\b(?:[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?\.)+(?:com|org|net|edu|gov|io|es|eu|local|internal|corp|lan|home|priv|intranet)\b")
        .expect("Regex Domain inválida")
});

static HOSTNAME_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\b(?:(?:host|srv|server|dc|ws|node|app)(?:-[a-z0-9]{2,12}|[a-z0-9]{0,8}\d[a-z0-9]{0,6})|(?:ec2amaz|win|desktop|laptop)-[a-z0-9]{5,15}|[a-z]{2,10}(?:web|sql|db|app|srv|dc)\d{1,4})\b",
    )
    .expect("Regex Hostname inválida")
});

static URL_INTERNAL_HOST_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\bhttps?://([a-z][a-z0-9\-]{2,30})(?::\d{2,5}|/)")
        .expect("Regex URL Internal Hostname inválida")
});

static RIMA_PTR_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^([0-9]{1,3})\.red-([0-9]{1,3})-([0-9]{1,3})-([0-9]{1,3})\.((?:[a-z0-9-]+\.)*rima-tde\.net)$")
        .expect("Regex RIMA PTR inválida")
});

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

/// Comprueba si una dirección IPv4 corresponde a una máscara de subred contigua (/8../32) o a la ruta por defecto (0.0.0.0).
#[must_use]
pub fn is_ipv4_netmask(ip: Ipv4Addr) -> bool {
    let raw = u32::from(ip);
    let k = raw.leading_ones();
    if k == 0 {
        raw == 0
    } else if (8..=32).contains(&k) {
        if k == 32 {
            raw == u32::MAX
        } else {
            raw == (!0u32) << (32 - k)
        }
    } else {
        false
    }
}

pub struct IPv4Detector {
    subnet_map: RefCell<HashMap<String, String>>,
    used_subnets: RefCell<HashSet<String>>,
    pub ignore_netmasks: bool,
}

impl Default for IPv4Detector {
    fn default() -> Self {
        Self::new()
    }
}

impl IPv4Detector {
    pub fn new() -> Self {
        Self {
            subnet_map: RefCell::new(HashMap::new()),
            used_subnets: RefCell::new(HashSet::new()),
            ignore_netmasks: true,
        }
    }

    pub fn with_ignore_netmasks(ignore_netmasks: bool) -> Self {
        Self {
            subnet_map: RefCell::new(HashMap::new()),
            used_subnets: RefCell::new(HashSet::new()),
            ignore_netmasks,
        }
    }

    pub fn detector_type(&self) -> DetectorType {
        DetectorType::IPv4
    }

    pub fn find_matches<'a>(&self, text: &'a str) -> Vec<(usize, usize, &'a str)> {
        IPV4_REGEX
            .find_iter(text)
            .filter_map(|m| {
                // Verificar que no esté precedida por un punto (ej. OIDs como iso.3.6.1.2 o prefijo de versión)
                if m.start() > 0 {
                    let prefix = &text[..m.start()];
                    if prefix.ends_with('.') || prefix.ends_with("\\.") {
                        return None;
                    }
                }

                // Verificar que no esté seguida por un punto y otro segmento alfanumérico o punto (ej. 3.6.1.2.1)
                let suffix = &text[m.end()..];
                let after_dot = if let Some(r) = suffix.strip_prefix("\\.") {
                    Some(r)
                } else {
                    suffix.strip_prefix('.')
                };

                if after_dot.is_some_and(|rest| {
                    rest.chars()
                        .next()
                        .is_some_and(|c| c.is_ascii_alphanumeric() || c == '.')
                }) {
                    return None;
                }

                let raw = m.as_str();
                let parsed = if raw.as_bytes().contains(&b'\\') {
                    raw.replace('\\', "").parse::<Ipv4Addr>()
                } else {
                    raw.parse::<Ipv4Addr>()
                };
                if let Ok(ip) = parsed {
                    if self.ignore_netmasks && is_ipv4_netmask(ip) {
                        return None;
                    }
                    Some((m.start(), m.end(), raw))
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
                        // Preservar la misma clase / espacio privado:
                        if octets[0] == 192 && octets[1] == 168 {
                            // Subredes en 192.168.x.0/24 (256 subredes privadas)
                            let c: u8 = rng.gen_range(0..=255);
                            format!("192.168.{c}")
                        } else if octets[0] == 172 && (16..=31).contains(&octets[1]) {
                            // Subredes en 172.(16..31).x.0/24
                            let b: u8 = rng.gen_range(16..=31);
                            let c: u8 = rng.gen_range(0..=255);
                            format!("172.{b}.{c}")
                        } else {
                            // Subredes en 10.x.y.0/24 (65.536 subredes privadas)
                            let b: u8 = rng.gen_range(0..=255);
                            let c: u8 = rng.gen_range(0..=255);
                            format!("10.{b}.{c}")
                        }
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
        let id: u32 = rng.gen_range(0x1000_0000..0xFFFF_FFFF);
        format!("user_{id:08x}@example.com")
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

    /// Extrae la IP subyacente y el sufijo de dominio de un registro PTR de RIMA si coincide con el patrón.
    pub fn extract_rima_ptr_ip(hostname: &str) -> Option<(String, String)> {
        let caps = RIMA_PTR_REGEX.captures(hostname)?;
        let d: u8 = caps.get(1)?.as_str().parse().ok()?;
        let a: u8 = caps.get(2)?.as_str().parse().ok()?;
        let b: u8 = caps.get(3)?.as_str().parse().ok()?;
        let c: u8 = caps.get(4)?.as_str().parse().ok()?;
        let suffix = caps.get(5)?.as_str().to_string();
        Some((format!("{a}.{b}.{c}.{d}"), suffix))
    }

    /// Reconstruye el hostname PTR de RIMA a partir de una dirección IPv4 protegida y el sufijo de dominio.
    pub fn reconstruct_rima_ptr(pseudo_ip: &str, suffix: &str) -> Option<String> {
        let clean = pseudo_ip.replace('\\', "");
        let ip: Ipv4Addr = clean.parse().ok()?;
        let octets = ip.octets();
        Some(format!(
            "{}.red-{}-{}-{}.{suffix}",
            octets[3], octets[0], octets[1], octets[2]
        ))
    }

    /// Genera un seudónimo de dominio con soporte de coherencia IPv4 para registros PTR de RIMA.
    pub fn generate_pseudonym_with_ipv4(
        &self,
        original: &str,
        ipv4_detector: Option<&IPv4Detector>,
    ) -> String {
        if let Some((ip, suffix)) = Self::extract_rima_ptr_ip(original) {
            let pseudo_ip = if let Some(det) = ipv4_detector {
                det.generate_pseudonym(&ip)
            } else {
                let det = IPv4Detector::new();
                det.generate_pseudonym(&ip)
            };
            if let Some(ptr) = Self::reconstruct_rima_ptr(&pseudo_ip, &suffix) {
                return ptr;
            }
        }

        let orig_lower = original.to_lowercase();
        // Si el dominio ya termina en .service-[a-f0-9]+.example.com, anonimizar los prefijos y preservar el sufijo
        if let Some(pos) = orig_lower.find(".service-") {
            let suffix = &original[pos..];
            if suffix.ends_with(".example.com") {
                let mut rng = rand::thread_rng();
                let rnd_prefix: u32 = rng.gen_range(0x1000..0xFFFF);
                return format!("sub-{rnd_prefix:04x}{suffix}");
            }
        }

        let mut rng = rand::thread_rng();
        let id: u32 = rng.gen_range(0x1000_0000..0xFFFF_FFFF);
        format!("service-{id:08x}.example.com")
    }

    /// Genera un dominio RFC 2606 seguro (.example.com), preservando el sufijo .service-*.example.com si ya está presente.
    pub fn generate_pseudonym(&self, original: &str) -> String {
        self.generate_pseudonym_with_ipv4(original, None)
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
        let mut matches: Vec<(usize, usize, &'a str)> = HOSTNAME_REGEX
            .find_iter(text)
            .map(|m| (m.start(), m.end(), m.as_str()))
            .collect();

        for cap in URL_INTERNAL_HOST_REGEX.captures_iter(text) {
            if let Some(m) = cap.get(1) {
                let host = m.as_str();
                if !host.eq_ignore_ascii_case("localhost") && !host.contains('.') {
                    matches.push((m.start(), m.end(), host));
                }
            }
        }

        matches.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| b.1.cmp(&a.1)));
        let mut dedup: Vec<(usize, usize, &'a str)> = Vec::with_capacity(matches.len());
        for (s, e, val) in matches {
            if let Some(last) = dedup.last()
                && s < last.1
            {
                continue;
            }
            dedup.push((s, e, val));
        }
        dedup
    }

    pub fn generate_pseudonym(&self, original: &str) -> String {
        let mut rng = rand::thread_rng();
        let id: u32 = rng.gen_range(0x1000_0000..0xFFFF_FFFF);
        let lower = original.to_ascii_lowercase();
        let prefix = if lower.starts_with("srv") {
            "srv"
        } else if lower.starts_with("ec2amaz-") {
            return format!("EC2AMAZ-{:07X}", id & 0x0FFF_FFFF);
        } else if lower.starts_with("win-") {
            return format!("WIN-{id:08X}");
        } else {
            "host"
        };
        format!("{prefix}-anon-{id:08x}")
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

        // Probar coincidencia con delimitadores y escapes regex (^10\.155\.69\.21$)
        let regex_matches = detector.find_matches(r"Regla: ^10\.155\.69\.21$ activa");
        assert_eq!(regex_matches.len(), 1);
        assert_eq!(regex_matches[0].2, r"10\.155\.69\.21");
        let pseudo_escaped = detector.generate_pseudonym(regex_matches[0].2);
        assert!(pseudo_escaped.contains(r"\."));
    }

    #[test]
    fn test_ipv4_no_false_positives_on_snmp_oids() {
        let detector = IPv4Detector::new();
        let snmp_text = r"
iso.3.6.1.2.1.1.2.0 = OID: iso.3.6.1.4.1.8072.3.2.10
iso.3.6.1.2.1.1.3.0 = Timeticks: (63469694) 7 days, 8:18:16.94
iso.3.6.1.2.1.1.9.1.2.1 = OID: iso.3.6.1.6.3.11.3.1.1
1.3.6.1.4.1.8072.3.2.10 = OID: 1.3.6.1.2.1.92
Servidor legítimo en 192.168.1.55 con gateway 10.0.0.1.
";
        let matches = detector.find_matches(snmp_text);
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].2, "192.168.1.55");
        assert_eq!(matches[1].2, "10.0.0.1");
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

    #[test]
    fn test_rima_ptr_hostname_coherence() {
        let (extracted_ip, suffix) =
            DomainDetector::extract_rima_ptr_ip("245.red-83-48-73.staticip.rima-tde.net")
                .expect("Debería extraer IP de registro PTR de RIMA");
        assert_eq!(extracted_ip, "83.48.73.245");
        assert_eq!(suffix, "staticip.rima-tde.net");

        let ipv4_det = IPv4Detector::new();
        let domain_det = DomainDetector::new();

        let pseudo_ip = ipv4_det.generate_pseudonym(&extracted_ip);
        let pseudo_ptr = domain_det.generate_pseudonym_with_ipv4(
            "245.red-83-48-73.staticip.rima-tde.net",
            Some(&ipv4_det),
        );

        let ip_parts: Vec<&str> = pseudo_ip.split('.').collect();
        assert_eq!(ip_parts.len(), 4);
        let expected_ptr = format!(
            "{}.red-{}-{}-{}.staticip.rima-tde.net",
            ip_parts[3], ip_parts[0], ip_parts[1], ip_parts[2]
        );
        assert_eq!(pseudo_ptr, expected_ptr);
    }

    #[test]
    fn test_is_ipv4_netmask() {
        use std::str::FromStr;

        // Máscaras estándar contiguas
        assert!(is_ipv4_netmask(Ipv4Addr::from_str("0.0.0.0").unwrap()));
        assert!(is_ipv4_netmask(Ipv4Addr::from_str("255.0.0.0").unwrap()));
        assert!(is_ipv4_netmask(Ipv4Addr::from_str("255.128.0.0").unwrap()));
        assert!(is_ipv4_netmask(Ipv4Addr::from_str("255.255.0.0").unwrap()));
        assert!(is_ipv4_netmask(
            Ipv4Addr::from_str("255.255.255.0").unwrap()
        ));
        assert!(is_ipv4_netmask(
            Ipv4Addr::from_str("255.255.255.128").unwrap()
        ));
        assert!(is_ipv4_netmask(
            Ipv4Addr::from_str("255.255.255.192").unwrap()
        ));
        assert!(is_ipv4_netmask(
            Ipv4Addr::from_str("255.255.255.224").unwrap()
        ));
        assert!(is_ipv4_netmask(
            Ipv4Addr::from_str("255.255.255.240").unwrap()
        ));
        assert!(is_ipv4_netmask(
            Ipv4Addr::from_str("255.255.255.248").unwrap()
        ));
        assert!(is_ipv4_netmask(
            Ipv4Addr::from_str("255.255.255.252").unwrap()
        ));
        assert!(is_ipv4_netmask(
            Ipv4Addr::from_str("255.255.255.254").unwrap()
        ));
        assert!(is_ipv4_netmask(
            Ipv4Addr::from_str("255.255.255.255").unwrap()
        ));

        // IPs normales de host (NO deben considerarse máscaras de red)
        assert!(!is_ipv4_netmask(Ipv4Addr::from_str("192.168.1.1").unwrap()));
        assert!(!is_ipv4_netmask(Ipv4Addr::from_str("10.0.0.1").unwrap()));
        assert!(!is_ipv4_netmask(Ipv4Addr::from_str("172.16.5.4").unwrap()));
        assert!(!is_ipv4_netmask(Ipv4Addr::from_str("8.8.8.8").unwrap()));
        assert!(!is_ipv4_netmask(
            Ipv4Addr::from_str("255.255.255.1").unwrap()
        ));
        assert!(!is_ipv4_netmask(Ipv4Addr::from_str("128.0.0.1").unwrap()));
    }

    #[test]
    fn test_ipv4_detector_ignores_netmasks() {
        let detector_ignoring = IPv4Detector::with_ignore_netmasks(true);
        let sample = "IP: 192.168.1.50 Mask: 255.255.255.0 GW: 192.168.1.1 Default: 0.0.0.0";
        let matches = detector_ignoring.find_matches(sample);

        let matched_ips: Vec<&str> = matches.iter().map(|m| m.2).collect();
        assert_eq!(matched_ips, vec!["192.168.1.50", "192.168.1.1"]);

        // Con ignore_netmasks desactivado, sí debe detectar 255.255.255.0 y 0.0.0.0
        let detector_all = IPv4Detector::with_ignore_netmasks(false);
        let matches_all = detector_all.find_matches(sample);
        let matched_all_ips: Vec<&str> = matches_all.iter().map(|m| m.2).collect();
        assert_eq!(
            matched_all_ips,
            vec!["192.168.1.50", "255.255.255.0", "192.168.1.1", "0.0.0.0"]
        );
    }
}
