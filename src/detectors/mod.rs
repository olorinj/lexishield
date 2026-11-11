//! Registro y orquestador de detectores de patrones de LexiShield.

pub mod custom;
pub mod guid;
pub mod identity;
pub mod network;
pub mod o365;
pub mod windows;

use crate::models::{DetectorType, Mapping};
use crate::stopwords::is_protected_word;
use custom::{CompiledCustomRule, CustomRule};
use guid::GuidDetector;
use identity::{CreditCardDetector, SpanishDniNieDetector, TelephoneDetector};
use network::{DomainDetector, EmailDetector, HostnameDetector, IPv4Detector, IPv6Detector};
use o365::{AttachmentFileNameDetector, O365OriginatingServerDetector, O365SubjectDetector};
use std::collections::HashSet;
use windows::{WindowsHexIdDetector, WindowsLogonIdDetector, WindowsSidDetector};

/// Coincidencia extraída por un detector antes de ser añadida a la tabla de mapeo.
#[derive(Debug, Clone)]
pub struct RawMatch {
    pub start: usize,
    pub end: usize,
    pub original: String,
    pub detector_type: DetectorType,
}

/// Registro unificado de detectores.
pub struct DetectorRegistry {
    pub guid: GuidDetector,
    pub sid: WindowsSidDetector,
    pub logon_id: WindowsLogonIdDetector,
    pub hex_id: WindowsHexIdDetector,
    pub ipv4: IPv4Detector,
    pub ipv6: IPv6Detector,
    pub email: EmailDetector,
    pub domain: DomainDetector,
    pub hostname: HostnameDetector,
    pub dni_nie: SpanishDniNieDetector,
    pub credit_card: CreditCardDetector,
    pub telephone: TelephoneDetector,
    pub o365_subject: O365SubjectDetector,
    pub o365_server: O365OriginatingServerDetector,
    pub attachment: AttachmentFileNameDetector,
    pub custom_rules: Vec<CompiledCustomRule>,
    pub ignored_domains: HashSet<String>,
}

impl Default for DetectorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl DetectorRegistry {
    pub fn new() -> Self {
        Self::with_custom_rules(&[])
    }

    pub fn with_custom_rules(custom_rules: &[CustomRule]) -> Self {
        Self::with_custom_rules_and_ignored_domains(custom_rules, &[])
    }

    pub fn with_custom_rules_and_ignored_domains(
        custom_rules: &[CustomRule],
        ignored_domains: &[String],
    ) -> Self {
        let mut compiled = Vec::new();
        for r in custom_rules {
            match CompiledCustomRule::try_from_rule(r) {
                Ok(c) => {
                    log::debug!(
                        "Regla personalizada compilada con éxito: '{}' [{}]",
                        r.name,
                        r.pattern
                    );
                    compiled.push(c);
                }
                Err(e) => {
                    log::warn!(
                        "Regla personalizada '{}' descartada por regex inválida: {}",
                        r.name,
                        e
                    );
                }
            }
        }

        let mut ignored_set = HashSet::with_capacity(ignored_domains.len());
        for d in ignored_domains {
            let clean = d.trim().trim_end_matches('.').to_lowercase();
            if !clean.is_empty() {
                ignored_set.insert(clean);
            }
        }

        Self {
            guid: GuidDetector::new(),
            sid: WindowsSidDetector::new(),
            logon_id: WindowsLogonIdDetector::new(),
            hex_id: WindowsHexIdDetector::new(),
            ipv4: IPv4Detector::new(),
            ipv6: IPv6Detector::new(),
            email: EmailDetector::new(),
            domain: DomainDetector::new(),
            hostname: HostnameDetector::new(),
            dni_nie: SpanishDniNieDetector::new(),
            credit_card: CreditCardDetector::new(),
            telephone: TelephoneDetector::new(),
            o365_subject: O365SubjectDetector::new(),
            o365_server: O365OriginatingServerDetector::new(),
            attachment: AttachmentFileNameDetector::new(),
            custom_rules: compiled,
            ignored_domains: ignored_set,
        }
    }

    /// Comprueba si un dominio o FQDN está explícitamente ignorado por el usuario o es un proveedor conocido.
    pub fn is_ignored_domain(&self, candidate: &str) -> bool {
        if crate::stopwords::is_known_vendor_domain(candidate) {
            return true;
        }

        let lower = candidate.trim().trim_end_matches('.').to_lowercase();
        let clean = lower
            .strip_prefix("http://")
            .or_else(|| lower.strip_prefix("https://"))
            .unwrap_or(&lower);
        let domain_part = clean.split(['/', ':', '?', '#']).next().unwrap_or(clean);
        let domain_no_www = domain_part.strip_prefix("www.").unwrap_or(domain_part);

        if self.ignored_domains.contains(domain_part)
            || self.ignored_domains.contains(domain_no_www)
        {
            return true;
        }

        for d in &self.ignored_domains {
            let d_clean = d.strip_prefix("www.").unwrap_or(d);
            if domain_part.ends_with(&format!(".{d_clean}")) {
                return true;
            }
        }
        false
    }

    /// Genera un seudónimo apropiado según el tipo de detector.
    pub fn generate_pseudonym_for(&self, original: &str, dtype: &DetectorType) -> String {
        match dtype {
            DetectorType::GuidUuid => self.guid.generate_pseudonym(original),
            DetectorType::WindowsSid => self.sid.generate_pseudonym(original),
            DetectorType::WindowsLogonId => self.logon_id.generate_pseudonym(original),
            DetectorType::WindowsHexId => self.hex_id.generate_pseudonym(original),
            DetectorType::IPv4 => self.ipv4.generate_pseudonym(original),
            DetectorType::IPv6 => self.ipv6.generate_pseudonym(original),
            DetectorType::Email => self.email.generate_pseudonym(original),
            DetectorType::DomainFqdn => self
                .domain
                .generate_pseudonym_with_ipv4(original, Some(&self.ipv4)),
            DetectorType::Hostname => self.hostname.generate_pseudonym(original),
            DetectorType::SpanishDniNie => self.dni_nie.generate_pseudonym(original),
            DetectorType::CreditCard => self.credit_card.generate_pseudonym(original),
            DetectorType::Telephone => self.telephone.generate_pseudonym(original),
            DetectorType::O365Subject => self.o365_subject.generate_pseudonym(original),
            DetectorType::O365OriginatingServer => self.o365_server.generate_pseudonym(original),
            DetectorType::AttachmentFileName => self.attachment.generate_pseudonym(original),
            DetectorType::GenericToken => {
                let id: u32 =
                    rand::Rng::gen_range(&mut rand::thread_rng(), 0x1000_0000..0xFFFF_FFFF);
                format!("ANON_{id:08X}")
            }
            DetectorType::Custom(name) => {
                if let Some(rule) = self.custom_rules.iter().find(|r| &r.name == name) {
                    rule.generate_pseudonym(original)
                } else {
                    let id: u32 =
                        rand::Rng::gen_range(&mut rand::thread_rng(), 0x1000_0000..0xFFFF_FFFF);
                    format!("CUSTOM_{id:08X}")
                }
            }
        }
    }

    /// Escanea el texto aplicando primero reglas personalizadas y luego los detectores en el orden de prioridad.
    pub fn scan_text(
        &self,
        text: &str,
        priority_order: &[DetectorType],
        min_token_len: usize,
        mut on_found: Option<&mut dyn FnMut(&Mapping)>,
    ) -> Vec<Mapping> {
        let mut found_mappings: Vec<Mapping> = Vec::new();
        let mut seen_originals: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        let mut seen_pseudonyms: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        let mut claimed_spans: Vec<(usize, usize)> = Vec::new();

        // 1. Ejecutar primero las reglas personalizadas del usuario (máxima prioridad)
        for custom_rule in &self.custom_rules {
            let matches = custom_rule.find_matches(text);
            let dtype = custom_rule.detector_type();

            for (start, end, matched_str) in matches {
                let idx = claimed_spans.partition_point(|&(s, _)| s < end);
                let overlaps = idx > 0 && claimed_spans[idx - 1].1 > start;
                if overlaps {
                    continue;
                }

                let candidate = matched_str.trim();
                if candidate.is_empty() {
                    continue;
                }

                let ins_idx = claimed_spans.partition_point(|&(s, _)| s < start);
                claimed_spans.insert(ins_idx, (start, end));

                if !seen_originals.contains(candidate) {
                    seen_originals.insert(candidate.to_string());
                    let mut pseudonym = custom_rule.generate_pseudonym(candidate);
                    if pseudonym != "=" {
                        let mut attempts = 0;
                        while seen_pseudonyms.contains(&pseudonym) && attempts < 100 {
                            pseudonym = custom_rule.generate_pseudonym(candidate);
                            attempts += 1;
                        }
                        seen_pseudonyms.insert(pseudonym.clone());
                    }
                    let m = Mapping::new(candidate, pseudonym, dtype.clone());
                    if let Some(ref mut cb) = on_found {
                        cb(&m);
                    }
                    found_mappings.push(m);
                }
            }
        }

        // 2. Ejecutar detectores estándar según el orden de prioridad
        for dtype in priority_order {
            let matches: Vec<(usize, usize, &str)> = match dtype {
                DetectorType::GuidUuid => self.guid.find_matches(text),
                DetectorType::WindowsSid => self.sid.find_matches(text),
                DetectorType::WindowsLogonId => self.logon_id.find_matches(text),
                DetectorType::WindowsHexId => self.hex_id.find_matches(text),
                DetectorType::IPv4 => self.ipv4.find_matches(text),
                DetectorType::IPv6 => self.ipv6.find_matches(text),
                DetectorType::Email => self.email.find_matches(text),
                DetectorType::DomainFqdn => self.domain.find_matches(text),
                DetectorType::Hostname => self.hostname.find_matches(text),
                DetectorType::SpanishDniNie => self.dni_nie.find_matches(text),
                DetectorType::CreditCard => self.credit_card.find_matches(text),
                DetectorType::Telephone => self.telephone.find_matches(text),
                DetectorType::O365Subject => self.o365_subject.find_matches(text),
                DetectorType::O365OriginatingServer => self.o365_server.find_matches(text),
                DetectorType::AttachmentFileName => self.attachment.find_matches(text),
                DetectorType::GenericToken | DetectorType::Custom(_) => Vec::new(),
            };

            for (start, end, matched_str) in matches {
                let idx = claimed_spans.partition_point(|&(s, _)| s < end);
                let overlaps = idx > 0 && claimed_spans[idx - 1].1 > start;
                if overlaps {
                    continue;
                }

                let candidate = matched_str.trim();

                // Salvaguardas: Longitud mínima y palabras protegidas
                if candidate.len() < min_token_len && matches!(dtype, DetectorType::GenericToken) {
                    continue;
                }

                if is_protected_word(candidate) {
                    log::debug!(
                        "Token descartado por coincidir con palabra protegida: {candidate}"
                    );
                    continue;
                }

                let ins_idx = claimed_spans.partition_point(|&(s, _)| s < start);
                claimed_spans.insert(ins_idx, (start, end));

                if !seen_originals.contains(candidate) {
                    seen_originals.insert(candidate.to_string());
                    let pseudonym = if matches!(dtype, DetectorType::DomainFqdn)
                        && self.is_ignored_domain(candidate)
                    {
                        "=".to_string()
                    } else {
                        let mut p = self.generate_pseudonym_for(candidate, dtype);
                        let mut attempts = 0;
                        while seen_pseudonyms.contains(&p) && attempts < 100 {
                            p = self.generate_pseudonym_for(candidate, dtype);
                            attempts += 1;
                        }
                        p
                    };
                    if !pseudonym.is_empty() && pseudonym != "=" {
                        seen_pseudonyms.insert(pseudonym.clone());
                    }
                    log::debug!(
                        "Entidad detectada [{dtype}]: '{candidate}' -> Seudónimo: '{pseudonym}'"
                    );
                    let m = Mapping::new(candidate, pseudonym, dtype.clone());
                    if let Some(ref mut cb) = on_found {
                        cb(&m);
                    }
                    found_mappings.push(m);
                }
            }
        }

        log::debug!(
            "Escaneo de texto finalizado ({} caracteres). Entidades únicas registradas: {}",
            text.len(),
            found_mappings.len()
        );

        found_mappings
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ignored_domains_assigned_equals_pseudonym() {
        let registry = DetectorRegistry::new();
        let priority = vec![DetectorType::DomainFqdn];

        let text = "Consulta thehackernews.com, www.first.org, httpd.apache.org, www.incibe.es, wazuh.com, github.com, owasp.org, nvd.nist.gov, www.rfc-editor.org y www.cve.org junto con dominio.corporativo.privado.com";
        let mappings = registry.scan_text(text, &priority, 4, None);

        // Los dominios públicos conocidos deben estar marcados con "=" (omitidos)
        for domain in [
            "thehackernews.com",
            "www.first.org",
            "httpd.apache.org",
            "www.incibe.es",
            "wazuh.com",
            "github.com",
            "owasp.org",
            "nvd.nist.gov",
            "www.rfc-editor.org",
            "www.cve.org",
        ] {
            let m = mappings.iter().find(|m| m.original == domain);
            assert!(
                m.is_some(),
                "El dominio {domain} debería haber sido detectado"
            );
            let m_val = m.unwrap();
            assert_eq!(
                m_val.pseudonym, "=",
                "El dominio {domain} debería tener seudónimo '=' para omisión"
            );
            assert!(
                m_val.omitted,
                "El dominio {domain} debería tener flag omitted=true"
            );
        }

        // El dominio privado sí debe tener un seudónimo generado y no estar omitido
        let priv_m = mappings
            .iter()
            .find(|m| m.original == "dominio.corporativo.privado.com");
        assert!(priv_m.is_some());
        assert_ne!(priv_m.unwrap().pseudonym, "=");
        assert!(!priv_m.unwrap().omitted);
    }

    #[test]
    fn test_custom_user_ignored_domains() {
        let custom_domains = vec![
            "customportal.org".to_string(),
            "internallink.es".to_string(),
        ];
        let registry =
            DetectorRegistry::with_custom_rules_and_ignored_domains(&[], &custom_domains);
        let priority = vec![DetectorType::DomainFqdn];

        let text = "Enlaces a sub.customportal.org y otro.es";
        let mappings = registry.scan_text(text, &priority, 4, None);

        let custom_m = mappings
            .iter()
            .find(|m| m.original == "sub.customportal.org");
        assert!(custom_m.is_some());
        assert_eq!(custom_m.unwrap().pseudonym, "=");
        assert!(custom_m.unwrap().omitted);

        let other_m = mappings.iter().find(|m| m.original == "otro.es");
        assert!(other_m.is_some());
        assert_ne!(other_m.unwrap().pseudonym, "=");
    }
}
