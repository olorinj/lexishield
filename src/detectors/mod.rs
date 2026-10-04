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
        let mut compiled = Vec::new();
        for r in custom_rules {
            match CompiledCustomRule::try_from_rule(r) {
                Ok(c) => compiled.push(c),
                Err(e) => {
                    log::warn!(
                        "Regla personalizada '{}' descartada por regex inválida: {}",
                        r.name,
                        e
                    );
                }
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
        }
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
            DetectorType::DomainFqdn => self.domain.generate_pseudonym(original),
            DetectorType::Hostname => self.hostname.generate_pseudonym(original),
            DetectorType::SpanishDniNie => self.dni_nie.generate_pseudonym(original),
            DetectorType::CreditCard => self.credit_card.generate_pseudonym(original),
            DetectorType::Telephone => self.telephone.generate_pseudonym(original),
            DetectorType::O365Subject => self.o365_subject.generate_pseudonym(original),
            DetectorType::O365OriginatingServer => self.o365_server.generate_pseudonym(original),
            DetectorType::AttachmentFileName => self.attachment.generate_pseudonym(original),
            DetectorType::GenericToken => {
                let id: u64 =
                    rand::Rng::gen_range(&mut rand::thread_rng(), 100_000_000..999_999_999);
                format!("ANON_{id}")
            }
            DetectorType::Custom(name) => {
                if let Some(rule) = self.custom_rules.iter().find(|r| &r.name == name) {
                    rule.generate_pseudonym(original)
                } else {
                    let id: u64 =
                        rand::Rng::gen_range(&mut rand::thread_rng(), 100_000_000..999_999_999);
                    format!("CUSTOM_{id}")
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
                    found_mappings.push(Mapping::new(candidate, pseudonym, dtype.clone()));
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
                    let mut pseudonym = self.generate_pseudonym_for(candidate, dtype);
                    let mut attempts = 0;
                    while seen_pseudonyms.contains(&pseudonym) && attempts < 100 {
                        pseudonym = self.generate_pseudonym_for(candidate, dtype);
                        attempts += 1;
                    }
                    seen_pseudonyms.insert(pseudonym.clone());
                    found_mappings.push(Mapping::new(candidate, pseudonym, dtype.clone()));
                }
            }
        }

        found_mappings
    }
}
