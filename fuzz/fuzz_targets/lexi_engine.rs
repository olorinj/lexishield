#![no_main]

use libfuzzer_sys::fuzz_target;
use lexishield::config::LexiConfig;
use lexishield::engine::ObfuscatorEngine;
use lexishield::models::FormatType;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        // Inicializar configuración y motor
        let config = LexiConfig::default();
        let mut engine = ObfuscatorEngine::new(config);

        // Prueba 1: Verificar que el escaneo no produzca pánicos
        let _ = engine.scan_and_register_mappings(s);

        // Prueba 2: Verificar que la ofuscación de diferentes formatos no produzca pánicos
        let _ = engine.obfuscate_text(s, FormatType::Auto);
        let _ = engine.obfuscate_text(s, FormatType::Json);
        let _ = engine.obfuscate_text(s, FormatType::Xml);
        
        // No verificamos el resultado, solo garantizamos que no haya pánicos (Crash/OOM)
    }
});
