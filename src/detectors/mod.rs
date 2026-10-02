//! Registro y orquestador de detectores de patrones de LexiShield.

pub mod guid;
pub mod identity;
pub mod network;
pub mod o365;
pub mod windows;

use crate::models::{DetectorType, Mapping};
use crate::stopwords::is_protected_word;
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
}

impl Default for DetectorRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl DetectorRegistry {
    pub fn new() -> Self {
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
        }
    }

    /// Genera un seudónimo apropiado según el tipo de detector.
    pub fn generate_pseudonym_for(&self, original: &str, dtype: DetectorType) -> String {
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
                let id: u32 = rand::random::<u32>() % 9000 + 1000;
                format!("ANON_{}", id)
            }
        }
    }

    /// Escanea el texto aplicando los detectores en el orden estricto de prioridad configurado.
    ///
    /// Aplica **Mejora 2**: Validación de longitud mínima y exclusión de palabras protegidas.
    pub fn scan_text(
        &self,
        text: &str,
        priority_order: &[DetectorType],
        min_token_len: usize,
    ) -> Vec<Mapping> {
        let mut found_mappings: Vec<Mapping> = Vec::new();
        let mut seen_originals: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        let mut claimed_spans: Vec<(usize, usize)> = Vec::new();

        for &dtype in priority_order {
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
                DetectorType::GenericToken => Vec::new(),
            };

            for (start, end, matched_str) in matches {
                // Verificar si el rango se solapa con un patrón más prioritario ya capturado
                let overlaps = claimed_spans.iter().any(|&(s, e)| {
                    (start >= s && start < e) || (end > s && end <= e) || (start <= s && end >= e)
                });
                if overlaps {
                    continue;
                }

                let candidate = matched_str.trim();

                // Salvaguardas de la Mejora 2: Longitud mínima y palabras protegidas
                if candidate.len() < min_token_len {
                    // Permitir solo si es un tipo estructurado de alta fidelidad
                    if matches!(dtype, DetectorType::GenericToken) {
                        continue;
                    }
                }

                if is_protected_word(candidate) {
                    log::debug!(
                        "Token descartado por coincidir con palabra protegida: {}",
                        candidate
                    );
                    continue;
                }

                claimed_spans.push((start, end));

                if !seen_originals.contains(candidate) {
                    seen_originals.insert(candidate.to_string());
                    let pseudonym = self.generate_pseudonym_for(candidate, dtype);
                    found_mappings.push(Mapping::new(candidate, pseudonym, dtype));
                }
            }
        }

        found_mappings
    }
}
