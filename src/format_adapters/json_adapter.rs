//! Adaptador estructurado para JSON con consciencia semántica y perfil Wazuh/SIEM.
//!
//! Garantiza que SOLO los valores de las cadenas sean ofuscados, manteniendo intactas
//! todas las claves (keys), tipos numéricos, booleanos, nulos y estructura de objetos/arrays.
//! Además, implementa reglas contextuales por clave/ruta para logs de seguridad SIEM (Wazuh).

use crate::detectors::network::HostnameDetector;
use crate::detectors::windows::WindowsSidDetector;
use crate::engine::replacer::apply_single_pass_replacements;
use crate::models::{DetectorType, Mapping, ObfuscationError};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Default)]
pub struct JsonAdapter;

impl JsonAdapter {
    pub fn new() -> Self {
        Self
    }

    /// Comprueba si el texto de entrada es un JSON sintácticamente válido.
    pub fn is_valid_json(text: &str) -> bool {
        let trimmed = text.trim();
        if (trimmed.starts_with('{') && trimmed.ends_with('}'))
            || (trimmed.starts_with('[') && trimmed.ends_with(']'))
        {
            serde_json::from_str::<Value>(trimmed).is_ok()
        } else {
            false
        }
    }

    /// Detecta si el JSON corresponde a una alerta o evento de Wazuh/SIEM.
    pub fn is_wazuh_format(value: &Value) -> bool {
        match value {
            Value::Object(map) => {
                if let Some(Value::String(idx)) = map.get("_index")
                    && (idx.starts_with("wazuh-alerts") || idx.starts_with("wazuh-archives"))
                {
                    return true;
                }
                if map.contains_key("agent") && map.contains_key("manager") {
                    return true;
                }
                if let Some(Value::Object(data)) = map.get("data")
                    && (data.contains_key("win") || data.contains_key("vulnerability"))
                {
                    return true;
                }
                false
            }
            Value::Array(arr) => arr.first().is_some_and(Self::is_wazuh_format),
            _ => false,
        }
    }

    /// Comprueba si una cadena representa un literal hexadecimal (ej: "0x80000000000000", "0x2aeea762").
    pub fn is_hex_literal(val: &str) -> bool {
        let s = val.trim();
        if let Some(rest) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
            !rest.is_empty() && rest.chars().all(|c| c.is_ascii_hexdigit())
        } else {
            false
        }
    }

    /// Comprueba si la ruta de claves pertenece al contexto de eventos de sistema o seguridad (SIEM, Windows EventLog, Wazuh, Sysmon).
    pub fn is_security_event_path(path: &[String]) -> bool {
        path.iter().any(|segment| {
            let s = segment.to_lowercase();
            matches!(
                s.as_str(),
                "eventdata"
                    | "event_data"
                    | "win"
                    | "system"
                    | "security"
                    | "audit"
                    | "wazuh"
                    | "agent"
                    | "manager"
                    | "sysmon"
                    | "winlog"
                    | "data"
            )
        })
    }

    /// Comprueba si una clave JSON está protegida en cualquier contexto.
    pub fn is_ignored_key(key: &str) -> bool {
        let key_lower = key.to_lowercase();
        matches!(
            key_lower.as_str(),
            "keywords"
                | "targetlogonid"
                | "subjectlogonid"
                | "logonid"
                | "eventid"
                | "processid"
                | "threadid"
                | "opcode"
                | "level"
                | "task"
                | "version"
                | "id"
        )
    }

    /// Comprueba si una ruta y su valor representan un campo técnico protegido en logs de seguridad.
    ///
    /// Evita falsos positivos en JSONs de negocio (ej. `{"keywords": "CEO secreto"}` NO se ignora,
    /// pero `{"eventdata": {"keywords": "0x80000000000000"}}` SÍ se ignora).
    pub fn is_protected_json_path(path: &[String], value_str: &str) -> bool {
        let current_key = path.last().map_or("", String::as_str);
        let key_lower = current_key.to_lowercase();
        let in_security_context = Self::is_security_event_path(path);
        let is_hex = Self::is_hex_literal(value_str);

        match key_lower.as_str() {
            // keywords y logonid en logs de eventos o máscaras hex (ej: 0x80000000000000, 0x2aeea762)
            "keywords" | "targetlogonid" | "subjectlogonid" | "logonid" => {
                in_security_context || is_hex
            }

            // Identificadores técnicos de eventos y procesos
            "eventid" | "processid" | "threadid" | "opcode" | "level" | "task" | "version" => {
                in_security_context || is_hex || value_str.chars().all(|c| c.is_ascii_digit())
            }

            _ => false,
        }
    }

    /// Extrae entidades estructuradas y respetando el perfil semántico de Wazuh/SIEM.
    pub fn extract_contextual_mappings(
        &self,
        value: &Value,
        hostname_detector: &HostnameDetector,
        sid_detector: &WindowsSidDetector,
    ) -> Vec<Mapping> {
        let mut mappings = Vec::new();
        let mut path_stack = Vec::new();
        self.traverse_and_extract(
            value,
            &mut path_stack,
            hostname_detector,
            sid_detector,
            &mut mappings,
        );
        mappings
    }

    fn traverse_and_extract(
        &self,
        val: &Value,
        path: &mut Vec<String>,
        hostname_detector: &HostnameDetector,
        sid_detector: &WindowsSidDetector,
        mappings: &mut Vec<Mapping>,
    ) {
        match val {
            Value::Object(map) => {
                for (k, v) in map {
                    path.push(k.clone());
                    self.traverse_and_extract(v, path, hostname_detector, sid_detector, mappings);
                    path.pop();
                }
            }
            Value::Array(arr) => {
                for item in arr {
                    self.traverse_and_extract(
                        item,
                        path,
                        hostname_detector,
                        sid_detector,
                        mappings,
                    );
                }
            }
            Value::String(s) => {
                let s_trim = s.trim();
                if s_trim.is_empty() {
                    return;
                }

                let current_key = path.last().map_or("", String::as_str);
                let current_key_lower = current_key.to_lowercase();
                let in_security = Self::is_security_event_path(path);
                let full_path_lower = path
                    .iter()
                    .map(|segment| segment.to_lowercase())
                    .collect::<Vec<_>>()
                    .join(".");

                // 1. Claves y rutas que NUNCA deben ofuscarse
                if Self::is_protected_json_path(path, s_trim) {
                    mappings.push(Mapping::new(s_trim, "=", DetectorType::GenericToken));
                    return;
                }

                // 2. InternetMessageId: transformar respetando la estructura y dominio corporativo
                if current_key_lower == "internetmessageid" {
                    let pseudonym = Self::anonymize_internet_message_id(s_trim);
                    mappings.push(Mapping::new(
                        s_trim,
                        pseudonym,
                        DetectorType::Custom("InternetMessageId".to_string()),
                    ));
                    return;
                }

                // 3. Claves forzadas: Hostnames / Servidores
                let is_agent_or_manager = full_path_lower.ends_with("agent.name")
                    || full_path_lower.ends_with("manager.name");
                let is_workstation_or_computer = current_key_lower == "workstationname"
                    || (current_key_lower == "computer" && in_security);

                if (is_agent_or_manager || is_workstation_or_computer)
                    && !mappings.iter().any(|m| m.original == s_trim)
                {
                    // Si contiene un FQDN con dominio (ej: HOST05.acme.local)
                    let pseudo = if s_trim.contains('.') {
                        let parts: Vec<&str> = s_trim.splitn(2, '.').collect();
                        let host_part = parts[0];
                        let domain_part = parts[1];
                        let host_pseudo = hostname_detector.generate_pseudonym(host_part);
                        format!("{host_pseudo}.{domain_part}")
                    } else {
                        hostname_detector.generate_pseudonym(s_trim)
                    };
                    mappings.push(Mapping::new(s_trim, pseudo, DetectorType::Hostname));
                    return;
                }

                // 4. Claves forzadas: Dominios Windows NetBIOS
                if (current_key_lower == "targetdomainname"
                    || (current_key_lower == "domain" && in_security))
                    && !mappings.iter().any(|m| m.original == s_trim)
                {
                    let pseudo = format!("DOMAIN_{:04X}", rand::random::<u16>());
                    mappings.push(Mapping::new(s_trim, pseudo, DetectorType::DomainFqdn));
                    return;
                }

                // 5. Claves forzadas: Nombres de usuario Windows
                if (current_key_lower == "targetusername" || current_key_lower == "subjectusername")
                    && !mappings.iter().any(|m| m.original == s_trim)
                {
                    let pseudo = format!("user_{:04x}", rand::random::<u16>());
                    mappings.push(Mapping::new(s_trim, pseudo, DetectorType::GenericToken));
                    return;
                }

                // 6. Claves forzadas: SIDs Windows
                if (current_key_lower == "targetusersid" || current_key_lower == "subjectusersid")
                    && !mappings.iter().any(|m| m.original == s_trim)
                {
                    if crate::stopwords::is_protected_sid(s_trim) {
                        mappings.push(Mapping::new(s_trim, "=", DetectorType::WindowsSid));
                    } else {
                        let pseudo = sid_detector.generate_pseudonym(s_trim);
                        mappings.push(Mapping::new(s_trim, pseudo, DetectorType::WindowsSid));
                    }
                }
            }
            Value::Null | Value::Bool(_) | Value::Number(_) => {}
        }
    }

    /// Anonimiza un InternetMessageId preservando el dominio base corporativo.
    /// Ejemplo: <GV4PR07MB75414463CD6E1D812AE9C6E93EB29C2@GV4PR07MB754144.eurprd07.service-2dcc9961.example.com>
    /// -> <MSGID_A1B2C3D4E5F6@host-anon.eurprd07.service-2dcc9961.example.com>
    pub fn anonymize_internet_message_id(raw_id: &str) -> String {
        let trimmed = raw_id.trim();
        let unwrapped = trimmed.trim_start_matches('<').trim_end_matches('>');
        let has_angles = trimmed.starts_with('<') && trimmed.ends_with('>');

        if let Some((_local, domain)) = unwrapped.split_once('@') {
            let rnd_id: u64 = rand::random();
            let new_local = format!("MSGID_{rnd_id:016X}");

            // Preservar la parte del dominio base (ej: *.service-*.example.com o últimos dos/tres segmentos)
            let domain_parts: Vec<&str> = domain.split('.').collect();
            let new_domain = if domain_parts.len() >= 3 {
                let base_len = if domain.contains(".example.com") {
                    3
                } else {
                    2
                };
                let split_idx = domain_parts.len().saturating_sub(base_len);
                let preserved_base = domain_parts[split_idx..].join(".");
                let internal_parts = &domain_parts[..split_idx];
                let mut anon_internals = Vec::new();
                for (i, _) in internal_parts.iter().enumerate() {
                    anon_internals.push(format!("sub{i}"));
                }
                if anon_internals.is_empty() {
                    preserved_base
                } else {
                    format!("{}.{preserved_base}", anon_internals.join("."))
                }
            } else {
                domain.to_string()
            };

            let res = format!("{new_local}@{new_domain}");
            if has_angles { format!("<{res}>") } else { res }
        } else {
            let rnd_id: u64 = rand::random();
            format!("<MSGID_{rnd_id:016X}>")
        }
    }

    /// Procesa y ofusca el contenido de un documento JSON respetando las claves.
    pub fn process_json(
        &self,
        text: &str,
        replacement_map: &HashMap<String, String>,
        strict_word_boundaries: bool,
    ) -> Result<(String, usize), ObfuscationError> {
        let mut parsed: Value = serde_json::from_str(text).map_err(ObfuscationError::JsonError)?;

        let mut total_replacements = 0;
        let mut path_stack = Vec::new();
        self.traverse_and_replace(
            &mut parsed,
            &mut path_stack,
            replacement_map,
            strict_word_boundaries,
            &mut total_replacements,
        );

        let output = if text.contains('\n') {
            serde_json::to_string_pretty(&parsed).map_err(ObfuscationError::JsonError)?
        } else {
            serde_json::to_string(&parsed).map_err(ObfuscationError::JsonError)?
        };

        Ok((output, total_replacements))
    }

    fn traverse_and_replace(
        &self,
        val: &mut Value,
        path: &mut Vec<String>,
        replacement_map: &HashMap<String, String>,
        strict_word_boundaries: bool,
        counter: &mut usize,
    ) {
        match val {
            Value::Object(map) => {
                // Las claves (keys) NUNCA se tocan, solo se recorren los valores
                for (key, value) in map.iter_mut() {
                    path.push(key.clone());
                    self.traverse_and_replace(
                        value,
                        path,
                        replacement_map,
                        strict_word_boundaries,
                        counter,
                    );
                    path.pop();
                }
            }
            Value::Array(arr) => {
                for item in arr.iter_mut() {
                    self.traverse_and_replace(
                        item,
                        path,
                        replacement_map,
                        strict_word_boundaries,
                        counter,
                    );
                }
            }
            Value::String(s) => {
                // Si la ruta y valor corresponden a un campo técnico protegido en logs de seguridad, omitir
                if Self::is_protected_json_path(path, s) {
                    return;
                }

                let (replaced, count) =
                    apply_single_pass_replacements(s, replacement_map, strict_word_boundaries);
                if count > 0 {
                    *s = replaced;
                    *counter += count;
                }
            }
            Value::Null | Value::Bool(_) | Value::Number(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::config::LexiConfig;
    use crate::engine::ObfuscatorEngine;
    use crate::models::FormatType;

    #[test]
    fn test_wazuh_event_obfuscation_and_preservation() {
        let json_input = r#"{
  "_index": "wazuh-alerts-4.x-2026.10.15",
  "agent": {
    "ip": "10.165.120.2",
    "name": "SRVF_002",
    "id": "007"
  },
  "manager": {
    "name": "wazuh"
  },
  "InternetMessageId": "<GV4PR07MB75414463CD6E1D812AE9C6E93EB29C2@GV4PR07MB754144.eurprd07.service-2dcc9961.example.com>",
  "data": {
    "win": {
      "eventdata": {
        "keywords": "0x80000000000000",
        "targetLogonId": "0x2aeea762",
        "targetDomainName": "ACME",
        "targetUserName": "visitas1",
        "targetUserSid": "S-1-5-21-390018920-242333331-881753101-2111",
        "workstationName": "COM00201",
        "nullSid": "S-1-0-0"
      },
      "system": {
        "computer": "service-4d975860.example.com"
      }
    }
  }
}"#;

        let config = LexiConfig::default();
        let mut engine = ObfuscatorEngine::new(config);

        // 1. Escanear y registrar entidades
        let mappings = engine
            .scan_and_register_mappings(json_input)
            .expect("Scan falló");
        assert!(
            !mappings.is_empty(),
            "Debe detectar mapeos en el JSON de Wazuh"
        );

        // 2. Ofuscar
        let (obfuscated, report) = engine
            .obfuscate_text(json_input, FormatType::Json)
            .expect("Ofuscación falló");

        assert!(report.replacements_applied > 0);

        // 3. Verificaciones de protección (NUNCA deben ofuscarse)
        assert!(
            obfuscated.contains("0x80000000000000"),
            "keywords no debe ofuscarse"
        );
        assert!(
            obfuscated.contains("0x2aeea762"),
            "targetLogonId no debe ofuscarse"
        );
        assert!(obfuscated.contains("S-1-0-0"), "S-1-0-0 no debe ofuscarse");

        // 4. Verificaciones de ofuscación forzada
        assert!(
            !obfuscated.contains("SRVF_002"),
            "agent.name debe ofuscarse"
        );
        assert!(
            !obfuscated.contains("\"wazuh\""),
            "manager.name debe ofuscarse"
        );
        assert!(
            !obfuscated.contains("\"visitas1\""),
            "targetUserName debe ofuscarse"
        );
        assert!(
            !obfuscated.contains("\"ACME\""),
            "targetDomainName debe ofuscarse"
        );
        assert!(
            !obfuscated.contains("\"COM00201\""),
            "workstationName debe ofuscarse"
        );
        assert!(
            !obfuscated.contains("S-1-5-21-390018920-242333331-881753101-2111"),
            "targetUserSid debe ofuscarse"
        );

        // 5. Verificación de InternetMessageId: debe anonimizar local y preservar base
        assert!(
            !obfuscated.contains("GV4PR07MB75414463CD6E1D812AE9C6E93EB29C2"),
            "Local part de InternetMessageId debe ofuscarse"
        );
        assert!(
            obfuscated.contains(".service-2dcc9961.example.com"),
            "Dominio base de InternetMessageId debe preservarse"
        );

        // 6. Desofuscación determinista 100% reversible
        let (restored, _) = engine
            .deobfuscate_text(&obfuscated, FormatType::Json)
            .expect("Desofuscación falló");

        let parsed_orig: serde_json::Value = serde_json::from_str(json_input).unwrap();
        let parsed_restored: serde_json::Value = serde_json::from_str(&restored).unwrap();
        assert_eq!(
            parsed_orig, parsed_restored,
            "La desofuscación debe restaurar exactamente el JSON original"
        );
    }

    #[test]
    fn test_generic_siem_event_obfuscation_and_preservation() {
        let json_input = r#"{
  "winlog": {
    "event_data": {
      "targetUserName": "administrador",
      "targetDomainName": "CORP",
      "workstationName": "PC-FINANZAS",
      "targetUserSid": "S-1-5-21-123456789-987654321-123456789-500",
      "targetLogonId": "0x3f12a9",
      "keywords": "0x80000000000000",
      "nullSid": "S-1-0-0"
    }
  }
}"#;

        let config = LexiConfig::default();
        let mut engine = ObfuscatorEngine::new(config);

        let mappings = engine
            .scan_and_register_mappings(json_input)
            .expect("Scan falló");
        assert!(!mappings.is_empty());

        let (obfuscated, report) = engine
            .obfuscate_text(json_input, FormatType::Json)
            .expect("Ofuscación falló");

        assert!(report.replacements_applied > 0);

        // Claves protegidas no deben ofuscarse
        assert!(obfuscated.contains("0x80000000000000"));
        assert!(obfuscated.contains("0x3f12a9"));
        assert!(obfuscated.contains("S-1-0-0"));

        // Campos forzados deben ofuscarse
        assert!(!obfuscated.contains("\"administrador\""));
        assert!(!obfuscated.contains("\"CORP\""));
        assert!(!obfuscated.contains("\"PC-FINANZAS\""));
        assert!(!obfuscated.contains("S-1-5-21-123456789-987654321-123456789-500"));

        // Desofuscación 100% reversible
        let (restored, _) = engine
            .deobfuscate_text(&obfuscated, FormatType::Json)
            .expect("Desofuscación falló");

        let parsed_orig: serde_json::Value = serde_json::from_str(json_input).unwrap();
        let parsed_restored: serde_json::Value = serde_json::from_str(&restored).unwrap();
        assert_eq!(parsed_orig, parsed_restored);
    }

    #[test]
    fn test_business_json_keywords_not_skipped() {
        let json_input = r#"{
  "article": {
    "title": "Informe trimestral",
    "keywords": "servidor 192.168.1.50 confidencial"
  }
}"#;

        let config = LexiConfig::default();
        let mut engine = ObfuscatorEngine::new(config);

        let mappings = engine
            .scan_and_register_mappings(json_input)
            .expect("Scan falló");
        assert!(!mappings.is_empty(), "Debe detectar la IP en keywords");

        let (obfuscated, report) = engine
            .obfuscate_text(json_input, FormatType::Json)
            .expect("Ofuscación falló");

        assert!(report.replacements_applied > 0);
        assert!(
            !obfuscated.contains("192.168.1.50"),
            "La IP en keywords de un JSON de negocio sí debe ofuscarse"
        );

        // Desofuscación 100% reversible
        let (restored, _) = engine
            .deobfuscate_text(&obfuscated, FormatType::Json)
            .expect("Desofuscación falló");

        let parsed_orig: serde_json::Value = serde_json::from_str(json_input).unwrap();
        let parsed_restored: serde_json::Value = serde_json::from_str(&restored).unwrap();
        assert_eq!(parsed_orig, parsed_restored);
    }
}
