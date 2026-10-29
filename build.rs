//! Script de construcción (build.rs) para incrustar recursos e iconos en binarios de Windows.

use std::path::PathBuf;

fn main() {
    println!("cargo:rerun-if-env-changed=DISABLE_WINRES");
    println!("cargo:rerun-if-env-changed=SKIP_WIN_RESOURCE");
    if std::env::var("DISABLE_WINRES").is_ok() || std::env::var("SKIP_WIN_RESOURCE").is_ok() {
        return;
    }

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
        res.set("CompanyName", "Julio C. Semper");
        let pkg_version =
            std::env::var("CARGO_PKG_VERSION").unwrap_or_else(|_| "2.0.0".to_string());
        let mut ver_parts: Vec<&str> = pkg_version
            .split('-')
            .next()
            .unwrap_or("2.0.0")
            .split('.')
            .collect();
        while ver_parts.len() < 4 {
            ver_parts.push("0");
        }
        let win_ver = ver_parts[..4].join(".");

        res.set("FileVersion", &win_ver);
        res.set("ProductVersion", &pkg_version);

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
