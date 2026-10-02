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
    /// Lista los mapeos contenidos en un archivo JSON.
    List {
        /// Ruta al archivo JSON de mapeos.
        #[arg(short, long)]
        mappings: PathBuf,
    },
    /// Añade un nuevo mapeo manual al archivo JSON.
    Add {
        /// Valor original sensible.
        #[arg(short, long)]
        original: String,

        /// Seudónimo ofuscado.
        #[arg(short, long)]
        pseudonym: String,

        /// Ruta al archivo JSON de mapeos.
        #[arg(short, long)]
        mappings: PathBuf,
    },
    /// Vacía o elimina todos los mapeos del archivo JSON.
    Clear {
        /// Ruta al archivo JSON de mapeos.
        #[arg(short, long)]
        mappings: PathBuf,
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

        /// Ruta opcional para guardar la tabla de mapeos generada en JSON.
        #[arg(short, long)]
        save_mappings: Option<PathBuf>,
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

        /// Archivo JSON con los mapeos a aplicar.
        #[arg(short, long)]
        mappings: PathBuf,

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
    },

    /// Monitoriza el portapapeles en tiempo real (modo escucha nativo).
    Watch {
        /// Dirección de la transformación continua.
        #[arg(short, long, value_enum, default_value_t = CliDirection::Obfuscate)]
        direction: CliDirection,

        /// Archivo de mapeos a cargar y mantener sincronizado.
        #[arg(short, long)]
        mappings: Option<PathBuf>,

        /// Formato estructural (auto, json, xml, log, text).
        #[arg(short, long, value_enum, default_value_t = CliFormat::Auto)]
        format: CliFormat,
    },

    /// Gestiona manualmente diccionarios de mapeos en formato JSON.
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

/// Manejador de la acción de ofuscación.
fn handle_obfuscate(
    config: LexiConfig,
    input: Option<PathBuf>,
    output: Option<PathBuf>,
    text: Option<String>,
    clipboard: bool,
    format: FormatType,
    save_mappings: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let content = resolve_input_content(input.as_deref(), text.as_deref(), clipboard)?;

    let mut engine = ObfuscatorEngine::new(config);
    let detected_count = engine.scan_and_register_mappings(&content)?;
    log::info!("Detectados {} elementos sensibles.", detected_count);

    let (result, report) = engine.obfuscate_text(&content, format)?;

    if let Some(map_path) = save_mappings {
        let mappings = engine.manager.get_mappings();
        let json_data = serde_json::to_string_pretty(&mappings)?;
        fs::write(&map_path, json_data)?;
        log::info!("Mapeos guardados en: {}", map_path.display());
    }

    if clipboard {
        set_clipboard_text(&result)?;
        log::info!("Resultado ofuscado copiado al portapapeles.");
    } else if let Some(out_path) = output {
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

/// Manejador de la acción de desofuscación.
fn handle_deobfuscate(
    config: LexiConfig,
    input: Option<PathBuf>,
    output: Option<PathBuf>,
    text: Option<String>,
    clipboard: bool,
    mappings_path: PathBuf,
    format: FormatType,
) -> Result<(), Box<dyn std::error::Error>> {
    let content = resolve_input_content(input.as_deref(), text.as_deref(), clipboard)?;

    let map_data = fs::read_to_string(&mappings_path)?;
    let loaded_mappings: Vec<Mapping> = serde_json::from_str(&map_data)?;

    let mut engine = ObfuscatorEngine::new(config);
    engine.manager.load_mappings(loaded_mappings)?;

    let (result, report) = engine.deobfuscate_text(&content, format)?;

    if clipboard {
        set_clipboard_text(&result)?;
        log::info!("Resultado desofuscado copiado al portapapeles.");
    } else if let Some(out_path) = output {
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

/// Manejador de la acción de escaneo.
fn handle_scan(
    config: LexiConfig,
    input: Option<PathBuf>,
    text: Option<String>,
    clipboard: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let content = resolve_input_content(input.as_deref(), text.as_deref(), clipboard)?;

    let mut engine = ObfuscatorEngine::new(config);
    let count = engine.scan_and_register_mappings(&content)?;
    let mappings = engine.manager.get_mappings();

    log::info!(
        "Escaneo completado: {} elementos sensibles identificados.",
        count
    );

    for (i, m) in mappings.iter().enumerate() {
        println!(
            "{}. [{:?}] '{}' -> '{}'",
            i + 1,
            m.detector_type,
            m.original,
            m.pseudonym
        );
    }

    Ok(())
}

/// Manejador del comando Dict.
fn handle_dict(subcommand: DictCommands) -> Result<(), Box<dyn std::error::Error>> {
    match subcommand {
        DictCommands::List { mappings } => {
            let data = fs::read_to_string(&mappings)?;
            let list: Vec<Mapping> = serde_json::from_str(&data)?;
            println!("Mapeos en {}:", mappings.display());
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
        } => {
            let mut list: Vec<Mapping> = if mappings.exists() {
                let data = fs::read_to_string(&mappings)?;
                serde_json::from_str(&data).unwrap_or_default()
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
            let json = serde_json::to_string_pretty(&list)?;
            fs::write(&mappings, json)?;
            log::info!("Mapeo añadido correctamente a {}", mappings.display());
        }
        DictCommands::Clear { mappings } => {
            fs::write(&mappings, "[]")?;
            log::info!("Diccionario vaciado en {}", mappings.display());
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
        } => handle_obfuscate(
            config,
            input,
            output,
            text,
            clipboard,
            format.into(),
            save_mappings,
        ),

        Commands::Deobfuscate {
            input,
            output,
            text,
            clipboard,
            mappings,
            format,
        } => handle_deobfuscate(
            config,
            input,
            output,
            text,
            clipboard,
            mappings,
            format.into(),
        ),

        Commands::Scan {
            input,
            text,
            clipboard,
        } => handle_scan(config, input, text, clipboard),

        Commands::Watch {
            direction,
            mappings,
            format,
        } => watch_clipboard_loop(direction.into(), format.into(), config, mappings.as_deref()),

        Commands::Dict { subcommand } => handle_dict(subcommand),
    }
}
