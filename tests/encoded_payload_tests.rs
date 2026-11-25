//! Pruebas de integración extremo a extremo (Roundtrip Biyectivo y Expansión en Claro)
//! para cargas codificadas legibles (Base64 UTF-8/UTF-16LE, Hex, URL-encoding y CSV multiparte).

use lexishield::config::LexiConfig;
use lexishield::encoded_content::{
    EncodedPayloadMode, decode_base64_canonical, encode_base64, inspect_and_expand_encoded,
};
use lexishield::engine::ObfuscatorEngine;
use lexishield::models::FormatType;

#[test]
fn test_csv_base64_reencode_bijective_roundtrip() {
    let original_csv = "id,kind,idx,v\n\
3,ping,0,UE9ORw==\n\
4,meta,0,YzpcaW5ldHB1Ylx3d3dyb290XGV4dHJhbmV0XHdlYi5jb25maWd8TEVOPTkwNzY=\n\
5,t1,0,QUFB\n\
8,t2,0,ABCDEFG\n\
34,p,7,WzddIGNscnNhcG9sby5nbHMtc3BhaW4uZXM6MTAuMjM0LjIyMC44MixFQzJBTUFaLUE4SE5WRDM6RUMyQU1BWi1BOEhOVkQzLGpldHdlYjpDOlxpbmV0cHViXHd3d3Jvb3RcZXh0cmFuZXRcLGpldHdpbjpDOlxXaW5kb3dzXCxsb29wYms6bG9jYWxob3N0\n";

    let mut cfg = LexiConfig::default();
    cfg.general.encoded_payload_mode = EncodedPayloadMode::Reencode;
    let mut engine = ObfuscatorEngine::new(cfg);

    let mappings = engine
        .scan_and_register_mappings(original_csv)
        .expect("Escaneo exitoso");
    assert!(
        !mappings.is_empty(),
        "Debe detectar entidades dentro del Base64"
    );

    // Verificar que se detectaron dominio, IP, ruta Windows y hostname EC2AMAZ
    assert!(mappings.iter().any(|m| m.original == "10.234.220.82"));
    assert!(
        mappings
            .iter()
            .any(|m| m.original == "clrsapolo.gls-spain.es")
    );
    assert!(mappings.iter().any(|m| m.original == "EC2AMAZ-A8HNVD3"));
    assert!(
        mappings
            .iter()
            .any(|m| m.original == r"c:\inetpub\wwwroot\extranet\web.config")
    );

    let (protected_csv, prot_report) = engine
        .obfuscate_text(original_csv, FormatType::Auto)
        .expect("Protección exitosa");

    assert!(prot_report.replacements_applied >= 4);
    assert!(prot_report.encoded_blocks_processed >= 2);
    // Las filas sin secretos (ping / QUFB / ABCDEFG) se mantienen idénticas
    assert!(protected_csv.contains("3,ping,0,UE9ORw==\n"));
    assert!(protected_csv.contains("5,t1,0,QUFB\n"));
    assert!(protected_csv.contains("8,t2,0,ABCDEFG\n"));
    // Las filas con secretos cambiaron su Base64 y al decodificarlas NO contienen datos reales
    assert!(
        !protected_csv.contains("YzpcaW5ldHB1Ylx3d3dyb290XGV4dHJhbmV0XHdlYi5jb25maWd8TEVOPTkwNzY=")
    );

    for line in protected_csv.lines().skip(1) {
        let cell = line.rsplit(',').next().unwrap_or("");
        if let Some((bytes, _)) = decode_base64_canonical(cell, false)
            && let Ok(dec) = std::str::from_utf8(&bytes)
        {
            assert!(!dec.contains("10.234.220.82"));
            assert!(!dec.contains("gls-spain.es"));
            assert!(!dec.contains("EC2AMAZ-A8HNVD3"));
            assert!(!dec.to_lowercase().contains("inetpub\\wwwroot\\extranet"));
        }
    }

    // Restauración biyectiva 100% exacta al CSV original
    let (restored_csv, rest_report) = engine
        .deobfuscate_text(&protected_csv, FormatType::Auto)
        .expect("Restauración exitosa");
    assert!(rest_report.replacements_applied >= 4);
    assert_eq!(restored_csv, original_csv);
}

#[test]
fn test_multipart_chunked_web_config_expand_merge_and_reencode_roundtrip() {
    // Simulamos fragmentos desordenados por idx de un web.config con CipherValue binario, Redis AWS, vocweb01, claves API y dominio partido
    let full_xml = concat!(
        "<?xml version=\"1.0\"?>\r\n<configuration>\r\n",
        "  <CipherValue>CF1ENzM0JT4u3EvtjmgRgOqt0Z/WbQOQZkyEN2pBC5UyWG7CO2hbCPK7ucsNEC/TgVV6TiwJBgoiOaCTYS1MkpJ1TOkNIPXs2GczQ0GuYpFAE2CH2fnY7/CTMpH1JmGB</CipherValue>\r\n",
        "  <machineKey validationKey=\"DCAAB540C066886AD394F11FA840DDF5D590A37E\" decryptionKey=\"69DD216F87406DA8B84F024559AA2396\" />\r\n",
        "  <sessionState><add host=\"session-extranet.qe61rf.0001.eus2.cache.amazonaws.com:6379\" /></sessionState>\r\n",
        "  <appSettings>\r\n",
        "    <add key=\"www.asmred.com\" value=\"http://vocweb01:20655/EtiquetacionService.svc\" />\r\n",
        "    <add key=\"ApiKeyMaps\" value=\"AIzaSyCU3zyGJ0Z-2cu02DMOO5NtylFzMz-MLrc\" />\r\n",
        "    <add key=\"SecretKeyCaptcha\" value=\"6Lc51E4eAAAAAEIRH2wAW5hwwkbhppiNRSpT2fxI\" />\r\n",
        "  </appSettings>\r\n</configuration>\r\n"
    );

    // Partimos justo en mitad de "www.asmred.com" para demostrar protección transversal entre fragmentos
    let split_target = "www.asm";
    let split_pos = full_xml.find(split_target).expect("Existe www.asm") + split_target.len();
    let chunk0 = &full_xml[..split_pos];
    let chunk1 = &full_xml[split_pos..];

    let b64_0 = encode_base64(chunk0.as_bytes(), false, true);
    let b64_1 = encode_base64(chunk1.as_bytes(), false, true);

    // Emitimos desordenado: primero chunk1 (idx=split_pos) y luego chunk0 (idx=0), como en zz_wc773x
    let chunked_csv =
        format!("id,kind,idx,v\n3,ping,0,UE9ORw==\n15,wc,{split_pos},{b64_1}\n14,wc,0,{b64_0}\n");

    // 1. Verificar inspección y unión ordenada en claro para Studio
    let inspected = inspect_and_expand_encoded(&chunked_csv);
    assert_eq!(inspected.blocks_decoded, 3);
    assert_eq!(inspected.multipart_groups_merged, 1);
    let merged = inspected
        .merged_chunks_text
        .expect("Debe generar texto con fragmentos unidos");
    assert!(merged.contains("www.asmred.com"));
    assert!(merged.contains("CF1ENzM0JT4u3EvtjmgRgOqt0Z/WbQOQZkyEN2pBC5UyWG7CO2hbCPK7ucsNEC"));

    // 2. Verificar modo ExpandClear + merge_encoded_chunks
    let mut clear_cfg = LexiConfig::default();
    clear_cfg.general.encoded_payload_mode = EncodedPayloadMode::ExpandClear;
    clear_cfg.general.merge_encoded_chunks = true;
    let mut clear_engine = ObfuscatorEngine::new(clear_cfg);
    clear_engine
        .scan_and_register_mappings(&chunked_csv)
        .expect("Scan en modo claro exitoso");
    let (clear_protected, _) = clear_engine
        .obfuscate_text(&chunked_csv, FormatType::Auto)
        .expect("Protección en modo claro exitosa");

    assert!(!clear_protected.contains("www.asmred.com"));
    assert!(!clear_protected.contains("session-extranet.qe61rf"));
    assert!(!clear_protected.contains("vocweb01"));
    assert!(!clear_protected.contains("AIzaSyCU3zyGJ0Z-2cu02DMOO5NtylFzMz-MLrc"));
    assert!(!clear_protected.contains("6Lc51E4eAAAAAEIRH2wAW5hwwkbhppiNRSpT2fxI"));
    assert!(!clear_protected.contains("DCAAB540C066886AD394F11FA840DDF5D590A37E"));
    // CipherValue binario permanece intacto en claro
    assert!(
        clear_protected.contains("CF1ENzM0JT4u3EvtjmgRgOqt0Z/WbQOQZkyEN2pBC5UyWG7CO2hbCPK7ucsNEC")
    );

    // 3. Verificar modo Reencode biyectivo (incluso con www.asmred.com partido entre dos filas)
    let mut re_cfg = LexiConfig::default();
    re_cfg.general.encoded_payload_mode = EncodedPayloadMode::Reencode;
    let mut re_engine = ObfuscatorEngine::new(re_cfg);
    re_engine
        .scan_and_register_mappings(&chunked_csv)
        .expect("Scan reencode exitoso");
    let (re_protected, _) = re_engine
        .obfuscate_text(&chunked_csv, FormatType::Auto)
        .expect("Protección reencode exitosa");
    assert_ne!(re_protected, chunked_csv);

    let (re_restored, _) = re_engine
        .deobfuscate_text(&re_protected, FormatType::Auto)
        .expect("Restauración reencode exitosa");
    assert_eq!(re_restored, chunked_csv);
}
