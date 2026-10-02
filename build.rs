//! Script de construcción (build.rs) para incrustar recursos e iconos en binarios de Windows.

use std::path::PathBuf;

fn main() {
    let target = std::env::var("TARGET").unwrap_or_default();
    if target.contains("windows") {
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set(
            "FileDescription",
            "LexiShield - Motor de Ofuscación y Anonimización Segura",
        );
        res.set("ProductName", "LexiShield");
        res.set("OriginalFilename", "lexishield.exe");
        res.set("LegalCopyright", "Copyright (C) 2026");
        res.set("CompanyName", "Academia");
        res.set("FileVersion", "0.1.0.0");
        res.set("ProductVersion", "0.1.0.0");

        if target.contains("gnu") && !cfg!(target_os = "windows") {
            res.set_windres_path("x86_64-w64-mingw32-windres");
            res.set_ar_path("x86_64-w64-mingw32-ar");
        }

        if let Err(e) = res.compile() {
            panic!("Fallo al incrustar recursos de Windows: {}", e);
        }

        // Para GNU (MinGW), forzar al linker a incluir resource.o directamente
        // evitando que sea descartado por no contener símbolos referenciados
        if target.contains("gnu")
            && let Ok(out_dir) = std::env::var("OUT_DIR")
        {
            let res_obj = PathBuf::from(out_dir).join("resource.o");
            if res_obj.exists() {
                println!("cargo:rustc-link-arg={}", res_obj.display());
            }
        }
    }
}
