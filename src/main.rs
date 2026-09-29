//! Interfaz de Línea de Comandos (CLI) de LexiShield en Rust.

use clap::{Parser, Subcommand, ValueEnum};
use lexishield::config::load_config;
use lexishield::engine::ObfuscatorEngine;
use lexishield::logger::init_logger;
use lexishield::models::{FormatType, Mapping};
use std::fs;
use std::path::PathBuf;

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
        } => {
            let content = match (input.as_ref(), text.as_ref()) {
                (Some(p), _) => fs::read_to_string(p)?,
                (_, Some(t)) => t.clone(),
                (None, None) => {
                    eprintln!("Error: Debe proporcionar un archivo con --input o texto con --text");
                    std::process::exit(1);
                }
            };

            let mut engine = ObfuscatorEngine::new(config);
            let detected_count = engine.scan_and_register_mappings(&content)?;
            println!("Detectados {} elementos sensibles.", detected_count);

            let (result, report) = engine.obfuscate_text(&content, format.into())?;

            if let Some(map_path) = save_mappings {
                let mappings = engine.manager.get_mappings();
                let json_data = serde_json::to_string_pretty(&mappings)?;
                fs::write(&map_path, json_data)?;
                println!("Mapeos guardados en: {}", map_path.display());
            }

            match output {
                Some(out_path) => {
                    fs::write(&out_path, &result)?;
                    println!("Resultado guardado en: {}", out_path.display());
                }
                None => {
                    println!("\n--- RESULTADO OFUSCADO ---\n{}", result);
                }
            }

            println!(
                "\nInforme: Reemplazos aplicados: {} | Formato: {:?} | Esquema intacto: {} | Sintaxis válida: {} | Tiempo: {:.2}ms",
                report.replacements_applied,
                report.format_detected,
                report.schema_intact,
                report.syntax_valid,
                report.elapsed_ms
            );
        }

        Commands::Deobfuscate {
            input,
            output,
            text,
            mappings,
            format,
        } => {
            let content = match (input.as_ref(), text.as_ref()) {
                (Some(p), _) => fs::read_to_string(p)?,
                (_, Some(t)) => t.clone(),
                (None, None) => {
                    eprintln!("Error: Debe proporcionar un archivo con --input o texto con --text");
                    std::process::exit(1);
                }
            };

            let map_data = fs::read_to_string(&mappings)?;
            let loaded_mappings: Vec<Mapping> = serde_json::from_str(&map_data)?;

            let mut engine = ObfuscatorEngine::new(config);
            engine.manager.load_mappings(loaded_mappings)?;

            let (result, report) = engine.deobfuscate_text(&content, format.into())?;

            match output {
                Some(out_path) => {
                    fs::write(&out_path, &result)?;
                    println!("Resultado restaurado en: {}", out_path.display());
                }
                None => {
                    println!("\n--- RESULTADO DESOFUSCADO ---\n{}", result);
                }
            }

            println!(
                "\nInforme: Restauraciones aplicadas: {} | Tiempo: {:.2}ms",
                report.replacements_applied, report.elapsed_ms
            );
        }

        Commands::Scan { input, text } => {
            let content = match (input.as_ref(), text.as_ref()) {
                (Some(p), _) => fs::read_to_string(p)?,
                (_, Some(t)) => t.clone(),
                (None, None) => {
                    eprintln!("Error: Debe proporcionar un archivo con --input o texto con --text");
                    std::process::exit(1);
                }
            };

            let mut engine = ObfuscatorEngine::new(config);
            let count = engine.scan_and_register_mappings(&content)?;
            let mappings = engine.manager.get_mappings();

            println!(
                "Escaneo completado: {} elementos sensibles identificados.\n",
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
        }
    }

    Ok(())
}
