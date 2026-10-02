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
fn resolve_password(
    cli_pwd: Option<String>,
    ask_pwd: bool,
    path: Option<&Path>,
) -> Result<Option<String>, Box<dyn std::error::Error>> {
    if let Some(p) = cli_pwd {
        return Ok(Some(p));
    }
    if ask_pwd {
        let p = rpassword::prompt_password("Introduce la contraseña de la bóveda de mapeos: ")?;
        return Ok(Some(p));
    }
    if let Some(path) = path
        && path.exists()
        && let Ok(data) = fs::read(path)
        && lexishield::is_encrypted_vault(&data)
    {
        let p = rpassword::prompt_password(format!(
            "El archivo '{}' está cifrado con contraseña. Introduce la contraseña: ",
            path.display()
        ))?;
        return Ok(Some(p));
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

    let mut engine = ObfuscatorEngine::new(config);
    let detected_count = engine.scan_and_register_mappings(&content)?;
    log::info!("Detectados {} elementos sensibles.", detected_count);

    let (result, report) = engine.obfuscate_text(&content, opts.format)?;

    let is_default = opts.save_mappings.is_none();
    let map_path = if !opts.no_save {
        opts.save_mappings
            .or_else(|| lexishield::config::get_default_vault_path().ok())
    } else {
        None
    };

    if let Some(map_path) = map_path {
        // Forzar pregunta de contraseña si es el archivo por defecto y no se pasó una
        let force_ask = is_default && opts.password.is_none();
        
        let pwd = resolve_password(opts.password, opts.ask_password || force_ask, None)?;
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
    let pwd = resolve_password(opts.password, opts.ask_password || force_ask, Some(&map_path))?;
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
fn handle_scan(config: LexiConfig, opts: ScanOptions) -> Result<(), Box<dyn std::error::Error>> {
    let content =
        resolve_input_content(opts.input.as_deref(), opts.text.as_deref(), opts.clipboard)?;

    let mut engine = ObfuscatorEngine::new(config);
    let count = engine.scan_and_register_mappings(&content)?;
    let mappings = engine.manager.get_mappings();

    log::info!(
        "Escaneo completado: {} elementos sensibles identificados.",
        count
    );

    if count == 0 {
        return Ok(());
    }

    for (i, m) in mappings.iter().enumerate() {
        println!(
            "{}. [{:?}] '{}' -> '{}'",
            i + 1,
            m.detector_type,
            m.original,
            m.pseudonym
        );
    }

    let is_default = opts.save_mappings.is_none();
    let map_path = opts
        .save_mappings
        .or_else(|| lexishield::config::get_default_vault_path().ok())
        .ok_or("No se pudo resolver la ruta de mapeos por defecto")?;

    let mut save = opts.yes;
    let mut omitted_indices: Vec<usize> = Vec::new();
    
    if !save {
        use std::io::{self, Write};
        print!(
            "¿Desea guardar los {} mapeos en '{}'? [s/N, o números a omitir (ej: 1,3)]: ",
            count,
            map_path.display()
        );
        io::stdout().flush()?;
        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let trimmed = input.trim().to_lowercase();
        
        if trimmed == "s" || trimmed == "si" || trimmed == "y" || trimmed == "yes" {
            save = true;
        } else if trimmed == "n" || trimmed == "no" || trimmed.is_empty() {
            save = false;
        } else {
            let parts: Vec<&str> = trimmed
                .split(|c: char| !c.is_ascii_digit())
                .filter(|s| !s.is_empty())
                .collect();
                
            if !parts.is_empty() {
                let mut valid = true;
                for p in parts {
                    if let Ok(num) = p.parse::<usize>() {
                        if num > 0 && num <= count {
                            omitted_indices.push(num - 1);
                        } else {
                            println!("Aviso: el número {} está fuera de rango.", num);
                            valid = false;
                        }
                    } else {
                        valid = false;
                    }
                }
                if valid {
                    save = true;
                } else {
                    println!("Entrada inválida. Cancelando guardado.");
                    save = false;
                }
            } else {
                save = false;
            }
        }
    }

    if save {
        let mut final_mappings = mappings;
        for idx in omitted_indices {
            if let Some(m) = final_mappings.get_mut(idx) {
                m.pseudonym = "=".to_string();
                m.omitted = true;
                log::info!("Mapeo {} omitido por elección del usuario.", m.original);
            }
        }

        let force_ask = is_default && opts.password.is_none();
        let pwd = resolve_password(opts.password, opts.ask_password || force_ask, None)?;
        lexishield::save_mappings_auto(&map_path, &final_mappings, pwd.as_deref())?;
        log::info!("Mapeos guardados correctamente en: {}", map_path.display());
    } else {
        log::info!("Operación de guardado cancelada.");
    }

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
            let map_path = mappings.or_else(|| lexishield::config::get_default_vault_path().ok()).unwrap();
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
        DictCommands::Add {
            original,
            pseudonym,
            mappings,
            password,
            ask_password,
        } => {
            let is_default = mappings.is_none();
            let map_path = mappings.or_else(|| lexishield::config::get_default_vault_path().ok()).unwrap();
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
        DictCommands::Clear { mappings, password, ask_password } => {
            let is_default = mappings.is_none();
            let map_path = mappings.or_else(|| lexishield::config::get_default_vault_path().ok()).unwrap();
            let force_ask = is_default && password.is_none();

            let pwd = resolve_password(password, ask_password || force_ask, Some(&map_path))?;
            lexishield::save_mappings_auto(&map_path, &[], pwd.as_deref())?;
            log::info!("Diccionario vaciado en {}", map_path.display());
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
            let map_path = mappings
                .or_else(|| lexishield::config::get_default_vault_path().ok());
                
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
