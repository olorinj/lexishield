use lexishield::config::LexiConfig;
use lexishield::detectors::identity::{CreditCardDetector, SpanishDniNieDetector};
use lexishield::engine::ObfuscatorEngine;
use lexishield::models::{DetectorType, FormatType, Mapping};

#[test]
fn test_mejora_1_json_keys_intact() {
    let config = LexiConfig::default();
    let mut engine = ObfuscatorEngine::new(config);

    let json_input = r#"{
  "ip_servidor": "192.168.1.100",
  "usuario": "administrador",
  "detalles": {
    "email": "contacto@empresa.com",
    "puerto": 8080,
    "activo": true
  }
}"#;

    // Registramos mapeos
    engine
        .manager
        .add_mapping(Mapping::new(
            "192.168.1.100",
            "192.0.2.1",
            DetectorType::IPv4,
        ))
        .unwrap();
    engine
        .manager
        .add_mapping(Mapping::new(
            "contacto@empresa.com",
            "user_99@example.com",
            DetectorType::Email,
        ))
        .unwrap();

    let (obfuscated, report) = engine.obfuscate_text(json_input, FormatType::Json).unwrap();

    // Las claves deben seguir intactas en el JSON
    assert!(obfuscated.contains("\"ip_servidor\":"));
    assert!(obfuscated.contains("\"usuario\":"));
    assert!(obfuscated.contains("\"email\":"));
    assert!(obfuscated.contains("\"puerto\": 8080"));
    assert!(obfuscated.contains("\"activo\": true"));

    // Los valores deben haberse ofuscado
    assert!(obfuscated.contains("192.0.2.1"));
    assert!(obfuscated.contains("user_99@example.com"));
    assert!(!obfuscated.contains("192.168.1.100"));
    assert!(!obfuscated.contains("contacto@empresa.com"));

    assert!(report.schema_intact);
    assert!(report.syntax_valid);
}

#[test]
fn test_mejora_1_xml_tags_intact() {
    let config = LexiConfig::default();
    let mut engine = ObfuscatorEngine::new(config);

    let xml_input = r#"<?xml version="1.0" encoding="UTF-8"?>
<configuration>
    <server name="produccion">
        <ip>10.0.0.5</ip>
        <admin>admin@corp.es</admin>
    </server>
</configuration>"#;

    engine
        .manager
        .add_mapping(Mapping::new("10.0.0.5", "192.0.2.5", DetectorType::IPv4))
        .unwrap();
    engine
        .manager
        .add_mapping(Mapping::new(
            "admin@corp.es",
            "anon@example.com",
            DetectorType::Email,
        ))
        .unwrap();

    let (obfuscated, report) = engine.obfuscate_text(xml_input, FormatType::Xml).unwrap();

    // Las etiquetas deben conservarse
    assert!(obfuscated.contains("<configuration>"));
    assert!(obfuscated.contains("<server name=\"produccion\">"));
    assert!(obfuscated.contains("<ip>192.0.2.5</ip>"));
    assert!(obfuscated.contains("<admin>anon@example.com</admin>"));
    assert!(!obfuscated.contains("10.0.0.5"));

    assert!(report.schema_intact);
    assert!(report.syntax_valid);
}

#[test]
fn test_mejora_2_short_tokens_and_boundaries() {
    let config = LexiConfig::default();
    let mut engine = ObfuscatorEngine::new(config);

    // Mapeo corto "cat" -> no debe reemplazar dentro de "certificate" o "application"
    engine
        .manager
        .add_mapping(Mapping::new("cat", "DOG", DetectorType::GenericToken))
        .unwrap();

    let text = "The application uses a secure certificate for the cat server.";
    let (obfuscated, _) = engine.obfuscate_text(text, FormatType::Plaintext).unwrap();

    assert!(obfuscated.contains("application"));
    assert!(obfuscated.contains("certificate"));
    assert!(obfuscated.contains("for the DOG server."));
}

#[test]
fn test_mejora_4_bidirectional_roundtrip_injective() {
    let config = LexiConfig::default();
    let mut engine = ObfuscatorEngine::new(config);

    let original_json = r#"{
  "uuid": "e2f1837a-751a-4c28-98e3-057bfd589d81",
  "sid": "S-1-5-21-397955417-626881126-188441444-500",
  "email": "juan.perez@empresa.com"
}"#;

    // Escaneo y registro automático
    let detected_count = engine.scan_and_register_mappings(original_json).unwrap();
    assert_eq!(detected_count, 3);

    // Ofuscar
    let (obfuscated, _) = engine
        .obfuscate_text(original_json, FormatType::Json)
        .unwrap();
    assert!(!obfuscated.contains("e2f1837a-751a-4c28-98e3-057bfd589d81"));
    assert!(!obfuscated.contains("S-1-5-21-397955417-626881126-188441444-500"));
    assert!(!obfuscated.contains("juan.perez@empresa.com"));

    // Desofuscar (proceso inverso 100% fiel)
    let (restored, _) = engine
        .deobfuscate_text(&obfuscated, FormatType::Json)
        .unwrap();
    assert!(restored.contains("e2f1837a-751a-4c28-98e3-057bfd589d81"));
    assert!(restored.contains("S-1-5-21-397955417-626881126-188441444-500"));
    assert!(restored.contains("juan.perez@empresa.com"));
}

#[test]
fn test_mejora_5_semantically_valid_checksums() {
    let detector = SpanishDniNieDetector::new();
    // Validar cálculo de letra de DNI (Módulo 23)
    let letter = SpanishDniNieDetector::calculate_control_letter(12345678);
    assert_eq!(letter, 'Z');
    assert!(SpanishDniNieDetector::is_valid_dni_nie("12345678Z"));

    let generated_dni = detector.generate_pseudonym("12345678Z");
    assert!(
        SpanishDniNieDetector::is_valid_dni_nie(&generated_dni),
        "El DNI generado {} debe tener letra de control válida",
        generated_dni
    );

    // Validar Luhn en tarjeta de crédito
    let cc_detector = CreditCardDetector::new();
    let generated_cc = cc_detector.generate_pseudonym("4532-1234-5678-9010");
    assert!(
        CreditCardDetector::is_luhn_valid(&generated_cc),
        "La tarjeta generada {} debe cumplir el algoritmo de Luhn",
        generated_cc
    );
}

#[test]
fn test_omitted_mapping_with_equals_sign() {
    let config = LexiConfig::default();
    let mut engine = ObfuscatorEngine::new(config);

    // Regla de comodín "=": debe ser descartado y no sustituir nada por "="
    let mapping_omitted = Mapping::new("skip_me", "=", DetectorType::GenericToken);
    engine.add_mapping(mapping_omitted).unwrap();

    let text = "El valor skip_me debe permanecer inalterado.";
    let (result, report) = engine.obfuscate_text(text, FormatType::Plaintext).unwrap();

    assert_eq!(result, text);
    assert_eq!(report.replacements_applied, 0);
}
