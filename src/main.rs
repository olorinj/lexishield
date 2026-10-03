//! Interfaz de Línea de Comandos (CLI) de LexiShield en Rust.

use clap::{Parser, Subcommand, ValueEnum};
use lexishield::clipboard::{
    WatchDirection, get_clipboard_text, set_clipboard_text, watch_clipboard_loop,
};
use lexishield::config::{LexiConfig, load_config};
use lexishield::engine::ObfuscatorEngine;
use lexishield::logger::init_logger;
use lexishield::models::{DetectorType, FormatType, Mapping};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "lexishield")]
#[command(about = "Motor de anonimización y ofuscación semántica de alto rendimiento", long_about = None)]
#[command(version = "0.1.0")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(ValueEnum, Clone, Copy, Debug)]
enum CliFormat {
    Auto,
    Json,
    Xml,
    Log,
    Text,
}

impl From<CliFormat> for FormatType {
    fn from(f: CliFormat) -> Self {
        match f {
            CliFormat::Auto => FormatType::Auto,
            CliFormat::Json => FormatType::Json,
            CliFormat::Xml => FormatType::Xml,
            CliFormat::Log => FormatType::Log,
            CliFormat::Text => FormatType::Plaintext,
        }
    }
}

#[derive(ValueEnum, Clone, Copy, Debug)]
enum CliDirection {
    Obfuscate,
    Deobfuscate,
}

impl From<CliDirection> for WatchDirection {
    fn from(d: CliDirection) -> Self {
        match d {
            CliDirection::Obfuscate => WatchDirection::Obfuscate,
            CliDirection::Deobfuscate => WatchDirection::Deobfuscate,
        }
    }
}

#[derive(Subcommand)]
enum DictCommands {
    /// Lista los mapeos contenidos en un archivo de mapeos (JSON o cifrado .lexi).
    List {
        /// Ruta al archivo de mapeos. Si se omite, usa el default.
        #[arg(short, long)]
        mappings: Option<PathBuf>,

        /// Contraseña si el archivo de mapeos está cifrado.
        #[arg(short, long)]
        password: Option<String>,

        /// Solicitar la contraseña de forma interactiva y oculta.
        #[arg(long)]
        ask_password: bool,
    },
    /// Cambia la contraseña de un archivo de mapeos cifrado (Rekey).
    Rekey {
        /// Ruta al archivo de mapeos. Si se omite, usa el default.
        #[arg(short, long)]
        mappings: Option<PathBuf>,

        /// Contraseña original (actual).
        #[arg(short, long)]
        password: Option<String>,

        /// Nueva contraseña (si se omite, se preguntará interactivamente 2 veces).
        #[arg(short = 'n', long)]
        new_password: Option<String>,
    },
    /// Añade un nuevo mapeo manual al archivo de mapeos.
    Add {
        /// Valor original sensible.
        #[arg(short, long)]
        original: String,

        /// Seudónimo ofuscado.
        #[arg(short, long)]
        pseudonym: String,

        /// Ruta al archivo de mapeos. Si se omite, usa el default.
        #[arg(short, long)]
        mappings: Option<PathBuf>,

        /// Contraseña para cifrar o actualizar el archivo.
        #[arg(short, long)]
        password: Option<String>,

        /// Solicitar la contraseña de forma interactiva y oculta.
        #[arg(long)]
        ask_password: bool,
    },
    /// Elimina un mapeo específico de la bóveda (un par original-seudónimo).
    #[command(alias = "del")]
    Remove {
        /// Valor original del mapeo a eliminar.
        #[arg(short, long)]
        original: String,

        /// Ruta al archivo de mapeos. Si se omite, usa el default.
        #[arg(short, long)]
        mappings: Option<PathBuf>,

        /// Contraseña si el archivo de mapeos está cifrado.
        #[arg(short, long)]
        password: Option<String>,

        /// Solicitar la contraseña de forma interactiva y oculta.
        #[arg(long)]
        ask_password: bool,
    },
    /// Vacía o elimina todos los mapeos del archivo.
    Clear {
        /// Ruta al archivo de mapeos. Si se omite, usa el default.
        #[arg(short, long)]
        mappings: Option<PathBuf>,

        /// Contraseña si se desea mantener cifrado el archivo vacío.
        #[arg(short, long)]
        password: Option<String>,

        /// Solicitar la contraseña de forma interactiva y oculta.
        #[arg(long)]
        ask_password: bool,
    },
    /// Elimina permanentemente un archivo de mapeos (¡CUIDADO: Irreversible!).
    #[command(alias = "rm")]
    Delete {
        /// Ruta al archivo de mapeos. Si se omite, usa el default.
        #[arg(short, long)]
        mappings: Option<PathBuf>,

        /// Confirma la eliminación sin preguntar.
        #[arg(short = 'y', long = "yes")]
        yes: bool,
    },
}

#[derive(Subcommand)]
enum Commands {
    /// Ofusca un archivo, texto directo o portapapeles.
    Obfuscate {
        /// Archivo de entrada a procesar.
        #[arg(short, long)]
        input: Option<PathBuf>,

        /// Archivo de salida donde guardar el resultado.
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Texto directo a procesar (si no se especifica archivo).
        #[arg(short, long)]
        text: Option<String>,

        /// Procesar el contenido actual del portapapeles (ejecución atómica).
        #[arg(short, long)]
        clipboard: bool,

        /// Formato estructural (auto, json, xml, log, text).
        #[arg(short, long, value_enum, default_value_t = CliFormat::Auto)]
        format: CliFormat,

        /// Ruta opcional para guardar la tabla de mapeos generada (soporta JSON o cifrado).
        #[arg(short, long)]
        save_mappings: Option<PathBuf>,

        /// Contraseña para cifrar la tabla de mapeos guardada (Argon2 + ChaCha20-Poly1305).
        #[arg(short, long)]
        password: Option<String>,

        /// Solicitar la contraseña de cifrado interactivamente.
        #[arg(long)]
        ask_password: bool,

        /// No guardar los mapeos generados (ignora el guardado por defecto).
        #[arg(long)]
        no_save: bool,
    },

    /// Desofusca un archivo, texto o portapapeles utilizando una tabla de mapeos guardada.
    Deobfuscate {
        /// Archivo de entrada a desofuscar.
        #[arg(short, long)]
        input: Option<PathBuf>,

        /// Archivo de salida.
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Texto directo a desofuscar.
        #[arg(short, long)]
        text: Option<String>,

        /// Procesar el contenido actual del portapapeles (ejecución atómica).
        #[arg(short, long)]
        clipboard: bool,

        /// Archivo con los mapeos a aplicar (JSON o bóveda cifrada .lexi). Si se omite, usa el default.
        #[arg(short, long)]
        mappings: Option<PathBuf>,

        /// Contraseña para descifrar el archivo de mapeos.
        #[arg(short, long)]
        password: Option<String>,

        /// Solicitar la contraseña de descifrado interactivamente.
        #[arg(long)]
        ask_password: bool,

        /// Formato estructural (auto, json, xml, log, text).
        #[arg(short, long, value_enum, default_value_t = CliFormat::Auto)]
        format: CliFormat,
    },

    /// Escanea un archivo, texto o portapapeles y muestra los datos sensibles detectados sin modificarlos.
    Scan {
        /// Archivo a escanear.
        #[arg(short, long)]
        input: Option<PathBuf>,

        /// Texto a escanear.
        #[arg(short, long)]
        text: Option<String>,

        /// Escanear el contenido actual del portapapeles.
        #[arg(short, long)]
        clipboard: bool,

        /// Ruta opcional para guardar los mapeos detectados tras confirmar.
        #[arg(short, long)]
        save_mappings: Option<PathBuf>,

        /// Contraseña si el archivo de mapeos se guardará cifrado.
        #[arg(short, long)]
        password: Option<String>,

        /// Solicitar la contraseña de cifrado interactivamente.
        #[arg(long)]
        ask_password: bool,

        /// Guardar sin pedir confirmación interactiva.
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// Monitoriza el portapapeles en tiempo real (modo escucha nativo).
    Watch {
        /// Dirección de la transformación continua.
        #[arg(short, long, value_enum, default_value_t = CliDirection::Obfuscate)]
        direction: CliDirection,

        /// Archivo de mapeos a cargar y mantener sincronizado (JSON o cifrado).
        #[arg(short, long)]
        mappings: Option<PathBuf>,

        /// Contraseña si el archivo de mapeos está o se guardará cifrado.
        #[arg(short, long)]
        password: Option<String>,

        /// Solicitar la contraseña interactivamente.
        #[arg(long)]
        ask_password: bool,

        /// Formato estructural (auto, json, xml, log, text).
        #[arg(short, long, value_enum, default_value_t = CliFormat::Auto)]
        format: CliFormat,
    },

    /// Gestiona diccionarios de mapeos (soporta JSON y archivos cifrados .lexi).
    Dict {
        #[command(subcommand)]
        subcommand: DictCommands,
    },
}

/// Resuelve el contenido de entrada desde un archivo, texto o portapapeles.
/// Resuelve la ruta del archivo de bóveda, añadiendo el directorio por defecto si se provee solo el nombre, y extensión .lexi
fn resolve_vault_path(
    user_provided: Option<PathBuf>,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    match user_provided {
        None => Ok(lexishield::config::get_default_vault_path()?),
        Some(mut path) => {
            // Si es solo un nombre de archivo (sin ./ ni directorios)
            if path.parent().is_none_or(|p| p.as_os_str().is_empty()) {
                let config_dir = lexishield::config::get_user_config_dir()?;
                path = config_dir.join(path);
            }

            // Si no tiene extensión, añadir ".lexi" por defecto
            if path.extension().is_none() {
                path.set_extension("lexi");
            }

            Ok(path)
        }
    }
}

fn resolve_input_content(
    input: Option<&Path>,
    text: Option<&str>,
    clipboard: bool,
) -> Result<String, Box<dyn std::error::Error>> {
    if clipboard {
        return Ok(get_clipboard_text()?);
    }
    match (input, text) {
        (Some(p), _) => Ok(fs::read_to_string(p)?),
        (_, Some(t)) => Ok(t.to_string()),
        (None, None) => {
            log::error!(
                "Debe proporcionar un archivo con --input, texto con --text o activar --clipboard"
            );
            Err("No se especificó ninguna fuente de entrada válida".into())
        }
    }
}

/// Resuelve la contraseña ya sea desde CLI, de forma interactiva o detectando si el archivo está cifrado.
fn check_password_complexity(pwd: &str) {
    if pwd.len() < 8 {
        log::warn!("Complejidad de contraseña baja: muy corta (menos de 8 caracteres).");
    } else if pwd.chars().all(|c| c.is_ascii_lowercase()) {
        log::warn!("Complejidad de contraseña baja: solo contiene letras minúsculas.");
    } else if pwd.chars().all(|c| c.is_ascii_digit()) {
        log::warn!("Complejidad de contraseña baja: solo contiene números.");
    } else if pwd.chars().all(|c| c.is_alphabetic()) {
        log::warn!("Complejidad de contraseña baja: no contiene números ni símbolos.");
    }
}

fn prompt_new_password(prompt: &str) -> Result<String, Box<dyn std::error::Error>> {
    loop {
        let p1 = rpassword::prompt_password(prompt)?;
        let p2 = rpassword::prompt_password("Confirma la nueva contraseña: ")?;
        if p1 == p2 {
            check_password_complexity(&p1);
            return Ok(p1);
        } else {
            println!("Las contraseñas no coinciden. Inténtalo de nuevo.");
        }
    }
}

fn resolve_password(
    cli_pwd: Option<String>,
    ask_pwd: bool,
    path: Option<&Path>,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    if let Some(p) = cli_pwd {
        return Ok(Some(p));
    }

    let exists = path.is_some_and(|p| p.exists());
    let is_encrypted = if exists {
        let data = fs::read(path.unwrap()).unwrap_or_default();
        lexishield::is_encrypted_vault(&data)
    } else {
        false
    };

    if exists && is_encrypted {
        let p = rpassword::prompt_password(format!(
            "El archivo '{}' está cifrado. Introduce la contraseña actual: ",
            path.unwrap().display()
        ))?;
        return Ok(Some(p));
    }

    if ask_pwd {
        if exists && !is_encrypted {
            let p = prompt_new_password("Introduce una NUEVA contraseña para cifrar la bóveda: ")?;
            return Ok(Some(p));
        } else {
            let p =
                prompt_new_password("Introduce la contraseña para la NUEVA bóveda de mapeos: ")?;
            return Ok(Some(p));
        }
    }

    Ok(None)
}

struct ObfuscateOptions {
    input: Option<PathBuf>,
    output: Option<PathBuf>,
    text: Option<String>,
    clipboard: bool,
    format: FormatType,
    save_mappings: Option<PathBuf>,
    password: Option<String>,
    ask_password: bool,
    no_save: bool,
}

/// Manejador de la acción de ofuscación.
fn handle_obfuscate(
    config: LexiConfig,
    opts: ObfuscateOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let content =
        resolve_input_content(opts.input.as_deref(), opts.text.as_deref(), opts.clipboard)?;

    let is_default = opts.save_mappings.is_none();
    let map_path = if !opts.no_save {
        Some(resolve_vault_path(opts.save_mappings)?)
    } else {
        None
    };

    let mut engine = ObfuscatorEngine::new(config);

    let mut final_pwd = opts.password.clone();
    if let Some(ref path) = map_path
        && path.exists()
    {
        let pwd = resolve_password(opts.password.clone(), opts.ask_password, Some(path))?;
        final_pwd = pwd.clone();
        if let Ok(loaded) = lexishield::load_mappings_auto(path, pwd.as_deref()) {
            let _ = engine.manager.load_mappings(loaded);
        }
    }

    let newly_added = engine.scan_and_register_mappings(&content)?;
    log::info!(
        "Detectados {} NUEVOS elementos sensibles.",
        newly_added.len()
    );

    let (result, report) = engine.obfuscate_text(&content, opts.format)?;

    if let Some(map_path) = map_path {
        // Forzar pregunta de contraseña si es el archivo por defecto y no se pasó una ni se cargó antes
        let force_ask = is_default && final_pwd.is_none();

        let pwd = resolve_password(final_pwd, opts.ask_password || force_ask, None)?;
        let mappings = engine.manager.get_mappings();
        lexishield::save_mappings_auto(&map_path, &mappings, pwd.as_deref())?;
        if pwd.is_some() {
            log::info!("Mapeos guardados y CIFRADOS en: {}", map_path.display());
        } else {
            log::info!("Mapeos guardados en: {}", map_path.display());
        }
    }

    if opts.clipboard {
        set_clipboard_text(&result)?;
        log::info!("Resultado ofuscado copiado al portapapeles.");
    } else if let Some(out_path) = opts.output {
        fs::write(&out_path, &result)?;
        log::info!("Resultado guardado en: {}", out_path.display());
    } else {
        println!("{}", result);
    }

    log::info!(
        "Informe: Reemplazos: {} | Formato: {:?} | Tiempo: {:.2}ms",
        report.replacements_applied,
        report.format_detected,
        report.elapsed_ms
    );

    Ok(())
}

struct DeobfuscateOptions {
    input: Option<PathBuf>,
    output: Option<PathBuf>,
    text: Option<String>,
    clipboard: bool,
    mappings: Option<PathBuf>,
    password: Option<String>,
    ask_password: bool,
    format: FormatType,
}

/// Manejador de la acción de desofuscación.
fn handle_deobfuscate(
    config: LexiConfig,
    opts: DeobfuscateOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let content =
        resolve_input_content(opts.input.as_deref(), opts.text.as_deref(), opts.clipboard)?;

    let is_default = opts.mappings.is_none();
    let map_path = opts
        .mappings
        .or_else(|| lexishield::config::get_default_vault_path().ok())
        .ok_or("No se especificó archivo de mapeos y no se pudo usar el default")?;

    let force_ask = is_default && opts.password.is_none();
    let pwd = resolve_password(
        opts.password,
        opts.ask_password || force_ask,
        Some(&map_path),
    )?;
    let loaded_mappings = lexishield::load_mappings_auto(&map_path, pwd.as_deref())?;

    let mut engine = ObfuscatorEngine::new(config);
    engine.manager.load_mappings(loaded_mappings)?;

    let (result, report) = engine.deobfuscate_text(&content, opts.format)?;

    if opts.clipboard {
        set_clipboard_text(&result)?;
        log::info!("Resultado desofuscado copiado al portapapeles.");
    } else if let Some(out_path) = opts.output {
        fs::write(&out_path, &result)?;
        log::info!("Resultado restaurado en: {}", out_path.display());
    } else {
        println!("{}", result);
    }

    log::info!(
        "Informe: Restauraciones aplicadas: {} | Tiempo: {:.2}ms",
        report.replacements_applied,
        report.elapsed_ms
    );

    Ok(())
}

struct ScanOptions {
    input: Option<PathBuf>,
    text: Option<String>,
    clipboard: bool,
    save_mappings: Option<PathBuf>,
    password: Option<String>,
    ask_password: bool,
    yes: bool,
}

/// Manejador de la acción de escaneo.
fn collect_files(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if dir.is_dir() {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str())
                && name.starts_with('.')
            {
                continue; // Saltar archivos ocultos (.git, etc)
            }
            if path.is_dir() {
                files.extend(collect_files(&path)?);
            } else if path.is_file() {
                files.push(path);
            }
        }
    } else {
        files.push(dir.to_path_buf());
    }
    Ok(files)
}

fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

fn is_binary_or_compressed(path: &Path, file: &mut std::fs::File) -> bool {
    // 1. Detección por extensiones comunes binarias/comprimidas
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        let ext_lower = ext.to_lowercase();
        let binary_extensions = [
            // Comprimidos y empaquetados
            "zip", "gz", "tar", "tgz", "bz2", "tbz2", "xz", "txz", "7z", "rar", "zst", "iso", "cab",
            "deb", "rpm", "apk", "jar", "war", "ear",
            // Ejecutables y librerías binarias
            "exe", "dll", "so", "dylib", "bin", "o", "a", "obj", "pyc", "class", "wasm",
            // Multimedia
            "png", "jpg", "jpeg", "gif", "bmp", "ico", "webp", "tiff", "psd", "mp3", "mp4", "mkv",
            "avi", "mov", "wav", "flac", "ogg", "webm",
            // Documentos compilados y bases de datos
            "pdf", "docx", "xlsx", "pptx", "db", "sqlite", "sqlite3", "mdb",
            // Tipografías
            "woff", "woff2", "ttf", "eot", "otf", // Bóvedas cifradas de LexiShield
            "lexi",
        ];
        if binary_extensions.contains(&ext_lower.as_str()) {
            return true;
        }
    }

    // 2. Inspección rápida de cabecera (primeros 1024 bytes)
    use std::io::{Read, Seek, SeekFrom};
    let mut header = [0u8; 1024];
    if let Ok(n) = file.read(&mut header) {
        let _ = file.seek(SeekFrom::Start(0));
        if n > 0 {
            // Magic bytes de compresión
            if n >= 2 && header[0..2] == [0x1f, 0x8b] {
                return true;
            }
            if n >= 4 && header[0..4] == [0x50, 0x4b, 0x03, 0x04] {
                return true;
            }
            if n >= 6 && header[0..6] == [0x37, 0x7a, 0xbc, 0xaf, 0x27, 0x1c] {
                return true;
            }
            if n >= 3 && header[0..3] == [0x42, 0x5a, 0x68] {
                return true;
            }
            if n >= 6 && header[0..6] == [0xfd, 0x37, 0x7a, 0x58, 0x5a, 0x00] {
                return true;
            }
            if n >= 4 && header[0..4] == [0x28, 0xb5, 0x2f, 0xfd] {
                return true;
            }
            if n >= 4 && header[0..4] == [0x52, 0x61, 0x72, 0x21] {
                return true;
            }

            // Si contiene bytes nulos en la cabecera es binario
            if header[..n].contains(&0x00) {
                return true;
            }
        }
    }

    false
}

/// Escanea un archivo línea por línea con reporte de progreso en tiempo real (bytes leídos / tamaño total).
fn scan_file_with_progress(
    engine: &mut ObfuscatorEngine,
    path: &Path,
    prefix_tag: &str,
    display_name: &str,
) -> Result<Vec<Mapping>, Box<dyn std::error::Error>> {
    use std::fs::File;
    use std::io::{BufRead, BufReader, Write};

    let mut file = match File::open(path) {
        Ok(f) => f,
        Err(e) => {
            log::warn!("No se pudo abrir el archivo '{}': {}", path.display(), e);
            return Ok(Vec::new());
        }
    };

    let total_bytes = file.metadata().map(|m| m.len()).unwrap_or(0);

    if is_binary_or_compressed(path, &mut file) {
        println!(
            "\r  {}[ {} ] Omitido (formato binario o comprimido no admitido): {} \x1b[K",
            prefix_tag,
            format_size(total_bytes),
            display_name
        );
        return Ok(Vec::new());
    }

    let mut reader = BufReader::with_capacity(64 * 1024, file);
    let mut newly_added = Vec::new();
    let mut bytes_read = 0u64;
    let mut last_update = std::time::Instant::now();
    let mut byte_buf = Vec::new();

    print!(
        "\r  {}[ 0 B / {} ] (0.0%) Analizando: {} \x1b[K",
        prefix_tag,
        format_size(total_bytes),
        display_name
    );
    let _ = std::io::stdout().flush();

    loop {
        byte_buf.clear();
        let n = reader.read_until(b'\n', &mut byte_buf)?;
        if n == 0 {
            break;
        }
        bytes_read += n as u64;

        let line = String::from_utf8_lossy(&byte_buf);
        let mut file_new = engine.scan_and_register_mappings(&line)?;
        newly_added.append(&mut file_new);

        if last_update.elapsed().as_millis() >= 80 || bytes_read >= total_bytes {
            let pct = if total_bytes > 0 {
                ((bytes_read as f64 / total_bytes as f64) * 100.0).min(100.0)
            } else {
                100.0
            };
            print!(
                "\r  {}[ {} / {} ] ({:.1}%) Analizando: {} \x1b[K",
                prefix_tag,
                format_size(bytes_read),
                format_size(total_bytes),
                pct,
                display_name
            );
            let _ = std::io::stdout().flush();
            last_update = std::time::Instant::now();
        }
    }

    println!(
        "\r  {}[ {} / {} ] (100.0%) Analizado: {} \x1b[K",
        prefix_tag,
        format_size(total_bytes),
        format_size(total_bytes),
        display_name
    );

    Ok(newly_added)
}

/// Parsea una entrada de texto que contiene números o rangos (ej: "1, 3-5 8") a índices (0-based).
fn parse_omitted_indices(input: &str, total_count: usize) -> Option<Vec<usize>> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }

    let parts: Vec<&str> = trimmed
        .split(|c: char| c.is_whitespace() || c == ',')
        .filter(|s| !s.is_empty())
        .collect();

    if parts.is_empty() {
        return None;
    }

    let mut indices = Vec::new();
    let mut any_valid = false;

    for p in parts {
        if p.contains('-') {
            if let Some((start_str, end_str)) = p.split_once('-')
                && let (Ok(start), Ok(end)) = (start_str.parse::<usize>(), end_str.parse::<usize>())
            {
                if start > 0 && end >= start && end <= total_count {
                    for num in start..=end {
                        indices.push(num - 1);
                    }
                    any_valid = true;
                } else {
                    println!(
                        "⚠️  Aviso: el rango '{}-{}' está fuera de los límites (1-{}).",
                        start, end, total_count
                    );
                }
            }
        } else if let Ok(num) = p.parse::<usize>() {
            if num > 0 && num <= total_count {
                indices.push(num - 1);
                any_valid = true;
            } else {
                println!(
                    "⚠️  Aviso: el número '{}' está fuera de rango (1-{}).",
                    num, total_count
                );
            }
        }
    }

    if any_valid { Some(indices) } else { None }
}

/// Muestra y gestiona interactivamente los nuevos mapeos detectados en un archivo concreto.
fn prompt_file_mappings(
    file_display_name: &str,
    mappings: &mut [Mapping],
    engine: &mut ObfuscatorEngine,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::io::{self, Write};
    let count = mappings.len();
    if count == 0 {
        return Ok(());
    }

    println!(
        "\n📄 Nuevos elementos detectados en '{}' ({} nuevo{}):",
        file_display_name,
        count,
        if count == 1 { "" } else { "s" }
    );

    let mut omitted_indices: Vec<usize> = Vec::new();
    let chunk_size = 20;

    for (i, m) in mappings.iter().enumerate() {
        println!(
            "  {}. [{:?}] '{}' -> '{}'",
            i + 1,
            m.detector_type,
            m.original,
            m.pseudonym
        );

        if (i + 1) % chunk_size == 0 && i + 1 < count {
            print!(
                "--- Mostrados {} de {} [Enter para continuar, 'q' para saltar al final, o números/rangos a omitir (ej: 1, 3-5)]: ",
                i + 1,
                count
            );
            io::stdout().flush()?;
            let mut buf = String::new();
            io::stdin().read_line(&mut buf)?;
            let trimmed = buf.trim();
            if trimmed.eq_ignore_ascii_case("q") {
                break;
            }
            if let Some(mut newly_omitted) = parse_omitted_indices(trimmed, count) {
                let om_display: Vec<String> = newly_omitted
                    .iter()
                    .map(|idx| (idx + 1).to_string())
                    .collect();
                println!("  -> Marcados para OMITIR: {}", om_display.join(", "));
                omitted_indices.append(&mut newly_omitted);
            }
        }
    }

    omitted_indices.sort_unstable();
    omitted_indices.dedup();

    let omitted_msg = if !omitted_indices.is_empty() {
        format!(
            " ({} marcados previamente para omitir)",
            omitted_indices.len()
        )
    } else {
        String::new()
    };

    print!(
        "¿Aceptar estos mapeos para '{}'{}? [S/n, o números adicionales a omitir (ej: 1, 3-5)]: ",
        file_display_name, omitted_msg
    );
    io::stdout().flush()?;
    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let trimmed = input.trim();

    if trimmed.is_empty()
        || trimmed.eq_ignore_ascii_case("s")
        || trimmed.eq_ignore_ascii_case("si")
        || trimmed.eq_ignore_ascii_case("y")
        || trimmed.eq_ignore_ascii_case("yes")
    {
        // Aceptar con las omisiones seleccionadas
    } else if trimmed.eq_ignore_ascii_case("n") || trimmed.eq_ignore_ascii_case("no") {
        for idx in 0..count {
            omitted_indices.push(idx);
        }
    } else if let Some(mut newly_omitted) = parse_omitted_indices(trimmed, count) {
        let om_display: Vec<String> = newly_omitted
            .iter()
            .map(|idx| (idx + 1).to_string())
            .collect();
        println!("  -> Marcados para OMITIR: {}", om_display.join(", "));
        omitted_indices.append(&mut newly_omitted);
    }

    omitted_indices.sort_unstable();
    omitted_indices.dedup();

    for &idx in &omitted_indices {
        if let Some(m) = mappings.get_mut(idx) {
            m.pseudonym = "=".to_string();
            m.omitted = true;
            let _ = engine.manager.add_mapping(m.clone());
            log::info!("Mapeo '{}' omitido por elección del usuario.", m.original);
        }
    }

    println!();
    Ok(())
}

fn handle_scan(config: LexiConfig, opts: ScanOptions) -> Result<(), Box<dyn std::error::Error>> {
    let is_default = opts.save_mappings.is_none();
    let map_path = resolve_vault_path(opts.save_mappings)?;

    let mut engine = ObfuscatorEngine::new(config);

    let mut final_pwd = opts.password.clone();
    if map_path.exists() {
        let pwd = resolve_password(opts.password.clone(), opts.ask_password, Some(&map_path))?;
        final_pwd = pwd.clone();
        if let Ok(loaded) = lexishield::load_mappings_auto(&map_path, pwd.as_deref()) {
            let _ = engine.manager.load_mappings(loaded);
        }
    }

    let mut total_discovered = 0usize;

    if let Some(ref path) = opts.input {
        if path.is_dir() {
            let files = collect_files(path)?;
            let total = files.len();
            println!(
                "🔍 Escaneando directorio '{}' ({} archivos)...",
                path.display(),
                total
            );
            for (idx, f) in files.iter().enumerate() {
                let current = idx + 1;
                let rel_path = f.strip_prefix(path).unwrap_or(f);
                let prefix_tag = format!("[{}/{}] ", current, total);
                let mut file_new = scan_file_with_progress(
                    &mut engine,
                    f,
                    &prefix_tag,
                    &rel_path.display().to_string(),
                )?;

                if !file_new.is_empty() {
                    total_discovered += file_new.len();
                    if !opts.yes {
                        let disp = format!("[{}/{}] {}", current, total, rel_path.display());
                        prompt_file_mappings(&disp, &mut file_new, &mut engine)?;
                    }
                }
            }
        } else {
            println!("🔍 Escaneando archivo '{}'...", path.display());
            let mut file_new =
                scan_file_with_progress(&mut engine, path, "", &path.display().to_string())?;
            if !file_new.is_empty() {
                total_discovered += file_new.len();
                if !opts.yes {
                    prompt_file_mappings(&path.display().to_string(), &mut file_new, &mut engine)?;
                }
            }
        }
    } else {
        let content =
            resolve_input_content(opts.input.as_deref(), opts.text.as_deref(), opts.clipboard)?;
        println!(
            "🔍 Analizando texto en memoria ({})...",
            format_size(content.len() as u64)
        );
        let mut text_new = engine.scan_and_register_mappings(&content)?;
        if !text_new.is_empty() {
            total_discovered += text_new.len();
            if !opts.yes {
                prompt_file_mappings("Texto / Portapapeles", &mut text_new, &mut engine)?;
            }
        }
    }

    log::info!(
        "Escaneo completado: {} nuevos elementos sensibles identificados en total.",
        total_discovered
    );

    if total_discovered == 0 {
        println!("\n✨ Escaneo completado: no se encontraron nuevos elementos sensibles.");
        return Ok(());
    }

    let final_mappings = engine.manager.get_mappings();
    let omitted_count = final_mappings.iter().filter(|m| m.omitted).count();
    let active_count = final_mappings.len() - omitted_count;

    let force_ask = is_default && final_pwd.is_none();
    let pwd = resolve_password(final_pwd, opts.ask_password || force_ask, None)?;
    lexishield::save_mappings_auto(&map_path, &final_mappings, pwd.as_deref())?;
    println!(
        "\n💾 Se han guardado {} mapeos ({} activos, {} omitidos) en '{}'.",
        final_mappings.len(),
        active_count,
        omitted_count,
        map_path.display()
    );

    Ok(())
}

/// Manejador del comando Dict.
fn handle_dict(subcommand: DictCommands) -> Result<(), Box<dyn std::error::Error>> {
    match subcommand {
        DictCommands::List {
            mappings,
            password,
            ask_password,
        } => {
            let is_default = mappings.is_none();
            let map_path = resolve_vault_path(mappings)?;
            let force_ask = is_default && password.is_none();

            let pwd = resolve_password(password, ask_password || force_ask, Some(&map_path))?;
            let list = lexishield::load_mappings_auto(&map_path, pwd.as_deref())?;
            println!("Mapeos en {}:", map_path.display());
            for (i, m) in list.iter().enumerate() {
                println!(
                    "  {}. [{:?}] '{}' -> '{}'",
                    i + 1,
                    m.detector_type,
                    m.original,
                    m.pseudonym
                );
            }
        }
        DictCommands::Rekey {
            mappings,
            password,
            new_password,
        } => {
            let map_path = resolve_vault_path(mappings)?;

            if !map_path.exists() {
                return Err("El archivo de mapeos especificado no existe.".into());
            }

            // 1. Pedir la original 1 vez (o tomarla por parámetro)
            let pwd = resolve_password(password, true, Some(&map_path))?;

            // Cargar archivo para validar clave original
            let list = lexishield::load_mappings_auto(&map_path, pwd.as_deref())?;

            // 2. Pedir nueva contraseña 2 veces (o usar parámetro y validar)
            let new_pwd = if let Some(np) = new_password {
                check_password_complexity(&np);
                np
            } else {
                prompt_new_password("Introduce la NUEVA contraseña para el archivo de mapeos: ")?
            };

            // 3. Guardar con la nueva contraseña
            lexishield::save_mappings_auto(&map_path, &list, Some(&new_pwd))?;
            log::info!(
                "Contraseña cambiada correctamente en {}",
                map_path.display()
            );
        }
        DictCommands::Add {
            original,
            pseudonym,
            mappings,
            password,
            ask_password,
        } => {
            let is_default = mappings.is_none();
            let map_path = resolve_vault_path(mappings)?;
            let force_ask = is_default && password.is_none();

            let pwd = resolve_password(password, ask_password || force_ask, Some(&map_path))?;
            let mut list = if map_path.exists() {
                lexishield::load_mappings_auto(&map_path, pwd.as_deref()).unwrap_or_default()
            } else {
                Vec::new()
            };
            list.retain(|m| m.original != original);
            list.push(Mapping {
                original: original.clone(),
                pseudonym: pseudonym.clone(),
                detector_type: DetectorType::GenericToken,
                omitted: false,
            });
            lexishield::save_mappings_auto(&map_path, &list, pwd.as_deref())?;
            log::info!("Mapeo añadido correctamente a {}", map_path.display());
        }
        DictCommands::Remove {
            original,
            mappings,
            password,
            ask_password,
        } => {
            let is_default = mappings.is_none();
            let map_path = resolve_vault_path(mappings)?;
            let force_ask = is_default && password.is_none();

            let pwd = resolve_password(password, ask_password || force_ask, Some(&map_path))?;

            if !map_path.exists() {
                return Err("El archivo de mapeos no existe.".into());
            }

            let mut list = lexishield::load_mappings_auto(&map_path, pwd.as_deref())?;
            let initial_len = list.len();
            list.retain(|m| m.original != original);

            if list.len() < initial_len {
                lexishield::save_mappings_auto(&map_path, &list, pwd.as_deref())?;
                log::info!("Mapeo eliminado correctamente de {}", map_path.display());
            } else {
                log::warn!("No se encontró el mapeo '{}' en la bóveda.", original);
            }
        }
        DictCommands::Clear {
            mappings,
            password,
            ask_password,
        } => {
            let is_default = mappings.is_none();
            let map_path = resolve_vault_path(mappings)?;
            let force_ask = is_default && password.is_none();

            let pwd = resolve_password(password, ask_password || force_ask, Some(&map_path))?;
            lexishield::save_mappings_auto(&map_path, &[], pwd.as_deref())?;
            log::info!("Diccionario vaciado en {}", map_path.display());
        }
        DictCommands::Delete { mappings, yes } => {
            let map_path = resolve_vault_path(mappings)?;
            if !map_path.exists() {
                return Err("El archivo de mapeos especificado no existe.".into());
            }

            if !yes {
                use std::io::{self, Write};
                print!(
                    "⚠️  ¡ATENCIÓN! Vas a eliminar de forma IRREVERSIBLE la bóveda de mapeos '{}'.\nSi lo haces, NO podrás desofuscar los textos procesados con este archivo.\n¿Estás completamente seguro? [s/N]: ",
                    map_path.display()
                );
                io::stdout().flush()?;
                let mut input = String::new();
                io::stdin().read_line(&mut input)?;
                let trimmed = input.trim().to_lowercase();
                if trimmed != "s" && trimmed != "si" && trimmed != "y" && trimmed != "yes" {
                    log::info!("Operación cancelada.");
                    return Ok(());
                }
            }

            fs::remove_file(&map_path)?;
            log::info!("Bóveda de mapeos eliminada: {}", map_path.display());
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logger();
    let cli = Cli::parse();
    let config = load_config(None);

    match cli.command {
        Commands::Obfuscate {
            input,
            output,
            text,
            clipboard,
            format,
            save_mappings,
            password,
            ask_password,
            no_save,
        } => handle_obfuscate(
            config,
            ObfuscateOptions {
                input,
                output,
                text,
                clipboard,
                format: format.into(),
                save_mappings,
                password,
                ask_password,
                no_save,
            },
        ),

        Commands::Deobfuscate {
            input,
            output,
            text,
            clipboard,
            mappings,
            password,
            ask_password,
            format,
        } => handle_deobfuscate(
            config,
            DeobfuscateOptions {
                input,
                output,
                text,
                clipboard,
                mappings,
                password,
                ask_password,
                format: format.into(),
            },
        ),

        Commands::Scan {
            input,
            text,
            clipboard,
            save_mappings,
            password,
            ask_password,
            yes,
        } => handle_scan(
            config,
            ScanOptions {
                input,
                text,
                clipboard,
                save_mappings,
                password,
                ask_password,
                yes,
            },
        ),

        Commands::Watch {
            direction,
            mappings,
            password,
            ask_password,
            format,
        } => {
            let is_default = mappings.is_none();
            let map_path = mappings.or_else(|| lexishield::config::get_default_vault_path().ok());

            let force_ask = is_default && password.is_none() && map_path.is_some();
            let pwd = resolve_password(password, ask_password || force_ask, map_path.as_deref())?;
            watch_clipboard_loop(
                direction.into(),
                format.into(),
                config,
                map_path.as_deref(),
                pwd.as_deref(),
            )
        }

        Commands::Dict { subcommand } => handle_dict(subcommand),
    }
}
