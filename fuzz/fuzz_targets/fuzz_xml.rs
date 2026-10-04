#![no_main]

use libfuzzer_sys::fuzz_target;
use lexishield::format_adapters::XmlAdapter;
use std::collections::HashMap;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let adapter = XmlAdapter::new(true);
        let mut replacements = HashMap::new();
        replacements.insert("admin".to_string(), "user_anon".to_string());
        replacements.insert("192.168.1.1".to_string(), "10.0.0.1".to_string());
        replacements.insert("test@example.com".to_string(), "anon@domain.com".to_string());

        // Asegurar que procesar cualquier XML corrupto o malformado no cause pánicos ni crashes
        let _ = adapter.process(s, &replacements, true);
    }
});

