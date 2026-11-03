use lexishield::detectors::custom::{CompiledCustomRule, CustomRule, CustomStrategy};
use lexishield::format_adapters::office_adapter::OfficeAdapter;
use lexishield::models::{DetectorType, Mapping};
use lexishield::validator::SyntaxValidator;
use std::io::{Cursor, Write};
use std::path::Path;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

#[test]
fn test_security_pandoc_argument_sanitization() {
    // 1. Rutas válidas normales deben ser aceptadas
    assert!(SyntaxValidator::validate_external_cli_path(Path::new("/tmp/documento.docx")).is_ok());
    assert!(SyntaxValidator::validate_external_cli_path(Path::new("informe_2026.md")).is_ok());

    // 2. Rutas vacías deben rechazarse
    assert!(SyntaxValidator::validate_external_cli_path(Path::new("")).is_err());

    // 3. Intento de inyección de flags directos (--filter, -o, etc.)
    assert!(SyntaxValidator::validate_external_cli_path(Path::new("--filter=evil.lua")).is_err());
    assert!(SyntaxValidator::validate_external_cli_path(Path::new("-o")).is_err());
    assert!(SyntaxValidator::validate_external_cli_path(Path::new("/tmp/-malicious.docx")).is_err());

    // 4. Intento de inyección con caracteres de control (newline, null byte)
    assert!(
        SyntaxValidator::validate_external_cli_path(Path::new("/tmp/doc\n--evil.docx")).is_err()
    );
    assert!(SyntaxValidator::validate_external_cli_path(Path::new("/tmp/doc\0.docx")).is_err());
}

#[test]
fn test_security_vault_poly1305_tamper_resistance() {
    let temp_dir = std::env::temp_dir().join("lexishield_sec_test_vault");
    let _ = std::fs::create_dir_all(&temp_dir);
    let vault_path = temp_dir.join("tamper_test.lexi");

    let mappings = vec![Mapping::new(
        "secreto_corporativo",
        "Token_001",
        DetectorType::GenericToken,
    )];
    let pwd = "SuperSecretPassword2026!";

    lexishield::save_mappings_auto(&vault_path, &mappings, Some(pwd)).unwrap();

    // Comprobar que la carga legítima funciona
    let loaded = lexishield::load_mappings_auto(&vault_path, Some(pwd)).unwrap();
    assert_eq!(loaded.len(), 1);
    assert_eq!(loaded[0].original, "secreto_corporativo");

    // 1. Contraseña incorrecta debe fallar inmediatamente
    assert!(lexishield::load_mappings_auto(&vault_path, Some("WrongPassword")).is_err());

    // 2. Manipulación de 1 bit en el payload cifrado (ciphertext / tag Poly1305)
    let orig_bytes = std::fs::read(&vault_path).unwrap();
    assert!(orig_bytes.len() > 35);

    let mut tampered_bytes = orig_bytes.clone();
    let last_idx = tampered_bytes.len() - 1;
    tampered_bytes[last_idx] ^= 0x01; // Flip 1 bit en el tag de autenticación Poly1305

    let tampered_path = temp_dir.join("tampered_payload.lexi");
    std::fs::write(&tampered_path, &tampered_bytes).unwrap();

    let res = lexishield::load_mappings_auto(&tampered_path, Some(pwd));
    assert!(
        res.is_err(),
        "Poly1305 AEAD debe rechazar cualquier alteración de bits en el archivo cifrado"
    );

    // 3. Manipulación en el Nonce (bytes 21..33)
    let mut tampered_nonce = orig_bytes.clone();
    tampered_nonce[25] ^= 0xFF;
    std::fs::write(&tampered_path, &tampered_nonce).unwrap();
    assert!(lexishield::load_mappings_auto(&tampered_path, Some(pwd)).is_err());

    let _ = std::fs::remove_dir_all(&temp_dir);
}

#[test]
fn test_security_office_zip_resource_inspection() {
    // Construir un archivo ZIP en memoria con contenido altamente repetitivo (simulando alto ratio de compresión)
    let mut zip_buf = Vec::new();
    {
        let cursor = Cursor::new(&mut zip_buf);
        let mut writer = ZipWriter::new(cursor);
        let options = SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);

        writer.start_file("word/document.xml", options).unwrap();
        // Escribir 1 MB de 'A's altamente comprimibles
        let payload = vec![b'A'; 1024 * 1024];
        writer.write_all(&payload).unwrap();
        writer.finish().unwrap();
    }

    let metrics = OfficeAdapter::inspect_zip_resources(&zip_buf).unwrap();
    assert_eq!(metrics.uncompressed_bytes, 1024 * 1024);
    assert!(metrics.compressed_bytes < 50 * 1024);
    assert!(metrics.compression_ratio > 20.0);
    // Para 1 MB total no debe saltar la alerta de bloqueo/advertencia de recursos del sistema
    assert!(!metrics.requires_warning);
}

#[test]
fn test_security_custom_regex_complexity_bounds() {
    // 1. Regla normal válida debe compilar sin problemas
    let valid_rule = CustomRule {
        name: "Ticket Interno".to_string(),
        pattern: r"(?i)\bTCK-\d{4,8}\b".to_string(),
        prefix: Some("TCK-".to_string()),
        strategy: CustomStrategy::RandomDigits,
        omitted: false,
    };
    assert!(CompiledCustomRule::try_from_rule(&valid_rule).is_ok());

    // 2. Regla con explosión de estados DFA (repetición masiva anidada que excede el límite de 2 MB)
    let abusive_rule = CustomRule {
        name: "Regla Abusiva".to_string(),
        pattern: r"[a-zA-Z0-9]{1000}{1000}".to_string(),
        prefix: None,
        strategy: CustomStrategy::Mask,
        omitted: false,
    };
    assert!(
        CompiledCustomRule::try_from_rule(&abusive_rule).is_err(),
        "El compilador de reglas debe rechazar expresiones regulares que excedan el límite de memoria segura"
    );
}

