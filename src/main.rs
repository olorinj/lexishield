//! Interfaz de Línea de Comandos (CLI) de LexiShield en Rust.

use clap::{Parser, Subcommand, ValueEnum};
use lexishield::config::{LexiConfig, load_config};
use lexishield::engine::ObfuscatorEngine;
use lexishield::logger::init_logger;
use lexishield::models::{FormatType, Mapping};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Parser)]
#[command(name = "lexishield")]
#[command(about = "Motor de anonimización y ofuscación de datos y logs de alto rendimiento", long_about = None)]
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

#[derive(Subcommand)]
enum Commands {
    /// Ofusca un archivo o texto de entrada.
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

        /// Formato estructural (auto, json, xml, log, text).
        #[arg(short, long, value_enum, default_value_t = CliFormat::Auto)]
        format: CliFormat,

        /// Ruta opcional para guardar la tabla de mapeos generada en JSON.
        #[arg(short, long)]
        save_mappings: Option<PathBuf>,
    },

    /// Desofusca un archivo o texto utilizando una tabla de mapeos guardada.
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

        /// Archivo JSON con los mapeos a aplicar.
        #[arg(short, long)]
        mappings: PathBuf,

        /// Formato estructural (auto, json, xml, log, text).
        #[arg(short, long, value_enum, default_value_t = CliFormat::Auto)]
        format: CliFormat,
    },

    /// Escanea un archivo o texto y muestra los datos sensibles detectados sin modificarlos.
    Scan {
        /// Archivo a escanear.
        #[arg(short, long)]
        input: Option<PathBuf>,

        /// Texto a escanear.
        #[arg(short, long)]
        text: Option<String>,
    },
}

/// Resuelve el contenido de entrada desde un archivo o un argumento de texto directo.
fn resolve_input_content(
    input: Option<&Path>,
    text: Option<&str>,
) -> Result<String, Box<dyn std::error::Error>> {
    match (input, text) {
        (Some(p), _) => Ok(fs::read_to_string(p)?),
        (_, Some(t)) => Ok(t.to_string()),
        (None, None) => {
            log::error!("Debe proporcionar un archivo con --input o texto directo con --text");
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
    format: FormatType,
    save_mappings: Option<PathBuf>,
) -> Result<(), Box<dyn std::error::Error>> {
    let content = resolve_input_content(input.as_deref(), text.as_deref())?;

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

    match output {
        Some(out_path) => {
            fs::write(&out_path, &result)?;
            log::info!("Resultado guardado en: {}", out_path.display());
        }
        None => {
            println!("{}", result);
        }
    }

    log::info!(
        "Informe: Reemplazos aplicados: {} | Formato: {:?} | Esquema intacto: {} | Sintaxis válida: {} | Tiempo: {:.2}ms",
        report.replacements_applied,
        report.format_detected,
        report.schema_intact,
        report.syntax_valid,
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
    mappings_path: PathBuf,
    format: FormatType,
) -> Result<(), Box<dyn std::error::Error>> {
    let content = resolve_input_content(input.as_deref(), text.as_deref())?;

    let map_data = fs::read_to_string(&mappings_path)?;
    let loaded_mappings: Vec<Mapping> = serde_json::from_str(&map_data)?;

    let mut engine = ObfuscatorEngine::new(config);
    engine.manager.load_mappings(loaded_mappings)?;

    let (result, report) = engine.deobfuscate_text(&content, format)?;

    match output {
        Some(out_path) => {
            fs::write(&out_path, &result)?;
            log::info!("Resultado restaurado en: {}", out_path.display());
        }
        None => {
            println!("{}", result);
        }
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
) -> Result<(), Box<dyn std::error::Error>> {
    let content = resolve_input_content(input.as_deref(), text.as_deref())?;

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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logger();
    let cli = Cli::parse();
    let config = load_config(None);

    match cli.command {
        Commands::Obfuscate {
            input,
            output,
            text,
            format,
            save_mappings,
        } => handle_obfuscate(config, input, output, text, format.into(), save_mappings),

        Commands::Deobfuscate {
            input,
            output,
            text,
            mappings,
            format,
        } => handle_deobfuscate(config, input, output, text, mappings, format.into()),

        Commands::Scan { input, text } => handle_scan(config, input, text),
    }
}
