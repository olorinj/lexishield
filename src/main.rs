//! Interfaz de Línea de Comandos (CLI) de LexiShield en Rust.

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use lexishield::clipboard::{
    WatchDirection, get_clipboard_text, set_clipboard_text, watch_clipboard_loop,
};
use lexishield::config::{LexiConfig, load_config_with_rules};
use lexishield::engine::ObfuscatorEngine;
use lexishield::logger::init_logger;
use lexishield::models::{DetectorType, FormatType, Mapping};
use mimalloc::MiMalloc;
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};

#[global_allocator]
static GLOBAL: MiMalloc = MiMalloc;

fn parse_path(s: &str) -> Result<PathBuf, String> {
    shellexpand::full(s)
        .map(|e| PathBuf::from(e.into_owned()))
        .map_err(|e| e.to_string())
}

#[derive(Parser)]
#[command(name = "lexishield")]
#[command(about = "Motor de anonimización y ofuscación semántica de alto rendimiento", long_about = None)]
#[command(version = "2.0.0")]
struct Cli {
    /// Activa el nivel de log Debug para más detalles (Telemetría en stderr).
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Activa el modo de depuración detallada (DEBUG) con telemetría completa en stderr.
    #[arg(long, global = true)]
    debug: bool,

    /// Ruta a un archivo de configuración TOML personalizado.
    #[arg(value_parser = parse_path, long, global = true)]
    config: Option<PathBuf>,

    /// Ruta a un archivo de reglas TOML personalizado (~/.lexishield/rules.toml).
    #[arg(value_parser = parse_path, long, global = true)]
    rules: Option<PathBuf>,

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
        #[arg(value_parser = parse_path, short, long)]
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
        #[arg(value_parser = parse_path, short, long)]
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
        #[arg(short = 's', long)]
        pseudonym: String,

        /// Ruta al archivo de mapeos. Si se omite, usa el default.
        #[arg(value_parser = parse_path, short, long)]
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
        #[arg(value_parser = parse_path, short, long)]
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
        #[arg(value_parser = parse_path, short, long)]
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
        #[arg(value_parser = parse_path, short, long)]
        mappings: Option<PathBuf>,

        /// Confirma la eliminación sin preguntar.
        #[arg(short = 'y', long = "yes")]
        yes: bool,
    },
    /// Actualiza un mapeo existente en el archivo de mapeos.
    Update {
        /// Valor original sensible existente.
        #[arg(short, long)]
        original: String,

        /// Nuevo seudónimo ofuscado.
        #[arg(short = 's', long)]
        pseudonym: String,

        /// Ruta al archivo de mapeos. Si se omite, usa el default.
        #[arg(value_parser = parse_path, short, long)]
        mappings: Option<PathBuf>,

        /// Contraseña si el archivo de mapeos está cifrado.
        #[arg(short, long)]
        password: Option<String>,

        /// Solicitar la contraseña de forma interactiva y oculta.
        #[arg(long)]
        ask_password: bool,
    },
    /// Elimina múltiples mapeos en lote por sus valores originales.
    BatchRemove {
        /// Valores originales a eliminar separados por coma o repetidos.
        #[arg(short, long, value_delimiter = ',')]
        originals: Vec<String>,

        /// Ruta al archivo de mapeos. Si se omite, usa el default.
        #[arg(value_parser = parse_path, short, long)]
        mappings: Option<PathBuf>,

        /// Contraseña si el archivo de mapeos está cifrado.
        #[arg(short, long)]
        password: Option<String>,

        /// Solicitar la contraseña de forma interactiva y oculta.
        #[arg(long)]
        ask_password: bool,
    },
}

#[derive(Subcommand)]
enum VaultCommands {
    /// Lista todas las bóvedas disponibles en ~/.lexishield con su tamaño y estado.
    List,
    /// Crea una nueva bóveda vacía (opcionalmente cifrada).
    Create {
        /// Nombre de la nueva bóveda (ej: clientes.lexi).
        name: String,
        /// Contraseña si se desea cifrar la bóveda.
        #[arg(short, long)]
        password: Option<String>,
        /// Solicitar contraseña interactivamente.
        #[arg(long)]
        ask_password: bool,
    },
    /// Renombra una bóveda existente.
    Rename {
        /// Nombre actual de la bóveda.
        old_name: String,
        /// Nuevo nombre de la bóveda.
        new_name: String,
    },
    /// Elimina una bóveda existente de ~/.lexishield.
    #[command(alias = "rm")]
    Delete {
        /// Nombre de la bóveda a eliminar.
        name: String,
        /// Confirma la eliminación sin preguntar.
        #[arg(short = 'y', long = "yes")]
        yes: bool,
    },
    /// Exporta una bóveda copiando el archivo a una ruta externa.
    Export {
        /// Nombre de la bóveda a exportar.
        name: String,
        /// Ruta de destino del archivo.
        destination: PathBuf,
    },
    /// Importa un archivo de bóveda externo al almacén ~/.lexishield.
    Import {
        /// Ruta al archivo externo a importar.
        source: PathBuf,
        /// Nombre opcional de destino.
        #[arg(short, long)]
        name: Option<String>,
    },
    /// Restaura una bóveda desde su copia de seguridad automática (.bak).
    RestoreBackup {
        /// Nombre de la bóveda a recuperar desde .bak.
        name: String,
    },
}

#[derive(Subcommand)]
enum BackupCommands {
    /// Crea una copia de seguridad completa en tar.bz2 de alta compresión (con opción de cifrado AES).
    Create {
        /// Archivo de salida de la copia de seguridad (.tar.bz2 o .lexibak).
        #[arg(value_parser = parse_path, short, long)]
        output: Option<PathBuf>,
        /// Contraseña para cifrar la copia de seguridad.
        #[arg(short, long)]
        password: Option<String>,
        /// Solicitar la contraseña interactivamente.
        #[arg(long)]
        ask_password: bool,
    },
    /// Restaura una copia de seguridad comprimida (tar.bz2) sobre ~/.lexishield.
    Restore {
        /// Archivo de copia de seguridad a restaurar.
        #[arg(value_parser = parse_path, short, long)]
        input: PathBuf,
        /// Contraseña si la copia de seguridad está cifrada.
        #[arg(short, long)]
        password: Option<String>,
        /// Solicitar la contraseña interactivamente.
        #[arg(long)]
        ask_password: bool,
        /// Confirmar sobreescritura sin preguntar.
        #[arg(short = 'y', long = "yes")]
        yes: bool,
    },
}

#[derive(Subcommand)]
enum ClipboardCommands {
    /// Vacía de forma explícita el contenido del portapapeles del sistema.
    Clear,
    /// Muestra el contenido actual del portapapeles.
    Get,
}

#[derive(Subcommand)]
enum Commands {
    /// Ofusca un archivo, texto directo, entrada estándar (stdin) o portapapeles.
    Obfuscate {
        /// Archivo de entrada a procesar (use '-' para leer desde stdin).
        #[arg(value_parser = parse_path, short, long)]
        input: Option<PathBuf>,

        /// Leer datos directamente desde la entrada estándar (stdin).
        #[arg(long)]
        stdin: bool,

        /// Archivo de salida donde guardar el resultado.
        #[arg(value_parser = parse_path, short, long)]
        output: Option<PathBuf>,

        /// Guardar resultado como una copia (sufijada) en lugar de sobrescribir el original.
        #[arg(long)]
        copy: bool,

        /// Sufijo personalizado para modo copia (por defecto: _obfuscated).
        #[arg(long)]
        suffix: Option<String>,

        /// Limpiar cachés y tablas de sesión antes de procesar.
        #[arg(long)]
        clean_cache: bool,

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
        #[arg(value_parser = parse_path, short, long, alias = "mappings")]
        save_mappings: Option<PathBuf>,

        /// Exporta el documento Office sanitizado a Markdown (.md) extrayendo imágenes mediante Pandoc para revisión con IA.
        #[arg(long, num_args = 0..=1, default_missing_value = "auto")]
        export_md: Option<String>,

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
        /// Archivo de entrada a desofuscar (use '-' para leer desde stdin).
        #[arg(value_parser = parse_path, short, long)]
        input: Option<PathBuf>,

        /// Leer datos directamente desde la entrada estándar (stdin).
        #[arg(long)]
        stdin: bool,

        /// Archivo de salida.
        #[arg(value_parser = parse_path, short, long)]
        output: Option<PathBuf>,

        /// Texto directo a desofuscar.
        #[arg(short, long)]
        text: Option<String>,

        /// Procesar el contenido actual del portapapeles (ejecución atómica).
        #[arg(short, long)]
        clipboard: bool,

        /// Archivo con los mapeos a aplicar (JSON o bóveda cifrada .lexi). Si se omite, usa el default.
        #[arg(value_parser = parse_path, short, long)]
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

        /// Exporta el documento Office desofuscado a Markdown (.md) extrayendo imágenes mediante Pandoc para revisión con IA.
        #[arg(long, num_args = 0..=1, default_missing_value = "auto")]
        export_md: Option<String>,
    },

    /// Escanea un archivo, texto, stdin o portapapeles y muestra los datos sensibles detectados sin modificarlos.
    Scan {
        /// Archivo o directorio a escanear (use '-' para leer desde stdin).
        #[arg(value_parser = parse_path, short, long)]
        input: Option<PathBuf>,

        /// Explorar subdirectorios de forma recursiva (por defecto: true).
        #[arg(short = 'r', long, default_value_t = true, action = clap::ArgAction::Set)]
        recursive: bool,

        /// Desactivar el filtrado de .gitignore durante el escaneo de directorios.
        #[arg(long)]
        no_git_ignore: bool,

        /// Leer datos directamente desde la entrada estándar (stdin).
        #[arg(long)]
        stdin: bool,

        /// Texto a escanear.
        #[arg(short, long)]
        text: Option<String>,

        /// Escanear el contenido actual del portapapeles.
        #[arg(short, long)]
        clipboard: bool,

        /// Ruta opcional para guardar los mapeos detectados tras confirmar.
        #[arg(value_parser = parse_path, short, long)]
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
        #[arg(value_parser = parse_path, short, long)]
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

    /// Gestiona las bóvedas de mapeos en el perfil del usuario (~/.lexishield).
    Vault {
        #[command(subcommand)]
        subcommand: VaultCommands,
    },

    /// Gestiona copias de seguridad de alta compresión (tar.bz2) y restauración cifrada.
    Backup {
        #[command(subcommand)]
        subcommand: BackupCommands,
    },

    /// Herramientas de consulta y vaciado seguro del portapapeles del sistema.
    Clipboard {
        #[command(subcommand)]
        subcommand: ClipboardCommands,
    },

    /// Gestiona la configuración del sistema (~/.lexishield/config.toml).
    Config {
        #[command(subcommand)]
        subcommand: ConfigCommands,
    },

    /// Genera scripts de autocompletado para terminales (bash, zsh, fish, powershell, elvish).
    Completions {
        /// Shell objetivo para generar el autocompletado.
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Muestra el contenido del archivo de configuración activo.
    Show,
    /// Muestra la ruta absoluta del archivo de configuración.
    Path,
    /// Restablece la configuración a los valores por defecto (~/.lexishield/config.toml).
    Reset {
        /// Confirma el restablecimiento sin preguntar.
        #[arg(short = 'y', long = "yes")]
        yes: bool,
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
    stdin: bool,
) -> Result<String, Box<dyn std::error::Error>> {
    use std::io::{IsTerminal, Read};
    if clipboard {
        return Ok(get_clipboard_text()?);
    }
    if let Some(t) = text {
        return Ok(t.to_string());
    }
    if let Some(p) = input {
        if p.as_os_str() == "-" {
            let mut buffer = String::new();
            std::io::stdin().read_to_string(&mut buffer)?;
            return Ok(buffer);
        }
        return Ok(fs::read_to_string(p)?);
    }
    if stdin || !std::io::stdin().is_terminal() {
        let mut buffer = String::new();
        std::io::stdin().read_to_string(&mut buffer)?;
        return Ok(buffer);
    }
    log::error!(
        "Debe proporcionar un archivo con --input, pasar datos por tubería (stdin), texto con --text o activar --clipboard"
    );
    Err("No se especificó ninguna fuente de entrada válida".into())
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
    stdin: bool,
    output: Option<PathBuf>,
    copy: bool,
    suffix: Option<String>,
    clean_cache: bool,
    text: Option<String>,
    clipboard: bool,
    format: FormatType,
    save_mappings: Option<PathBuf>,
    password: Option<String>,
    ask_password: bool,
    no_save: bool,
    export_md: Option<String>,
}

fn export_office_to_md_with_pandoc(
    docx_path: &Path,
    export_md_arg: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let (md_path, media_dir) = if export_md_arg == "auto" || export_md_arg.is_empty() {
        let parent = docx_path.parent().unwrap_or(Path::new("."));
        let stem = docx_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("documento");
        let md = parent.join(format!("{stem}.md"));
        let media = parent.join(stem);
        (md, media)
    } else {
        let md = PathBuf::from(export_md_arg);
        let stem = md.file_stem().and_then(|s| s.to_str()).unwrap_or("media");
        let parent = md.parent().unwrap_or(Path::new("."));
        let media = parent.join(stem);
        (md, media)
    };

    println!(
        "🔄 Exportando a Markdown mediante Pandoc: {}...",
        docx_path.display()
    );

    let status = std::process::Command::new("pandoc")
        .arg(docx_path)
        .arg("-o")
        .arg(&md_path)
        .arg("--wrap=none")
        .arg("-s")
        .arg(format!("--extract-media={}", media_dir.display()))
        .status();

    match status {
        Ok(exit_status) => {
            if exit_status.success() {
                println!(
                    "✅ Exportación a Markdown completada con éxito: {}\n   Imágenes y medios extraídos en: {}",
                    md_path.display(),
                    media_dir.display()
                );
            } else {
                eprintln!(
                    "⚠️ Pandoc terminó con código de error: {:?}. Verifique que el documento sea compatible.",
                    exit_status.code()
                );
            }
        }
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                eprintln!(
                    "⚠️ Pandoc no se encuentra instalado o no está en el PATH del sistema.\n   Para exportar a Markdown instale Pandoc (ej: 'sudo apt install pandoc' o 'winget install JohnMacFarlane.Pandoc')."
                );
            } else {
                eprintln!("⚠️ Error al ejecutar Pandoc: {}", e);
            }
        }
    }

    Ok(())
}

struct ScanOptions {
    input: Option<PathBuf>,
    recursive: bool,
    no_git_ignore: bool,
    stdin: bool,
    text: Option<String>,
    clipboard: bool,
    save_mappings: Option<PathBuf>,
    password: Option<String>,
    ask_password: bool,
    yes: bool,
}

/// Manejador de la acción de ofuscación.
fn handle_obfuscate(
    config: LexiConfig,
    opts: ObfuscateOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let is_default = opts.save_mappings.is_none();
    let map_path = if !opts.no_save {
        Some(resolve_vault_path(opts.save_mappings)?)
    } else {
        None
    };

    let is_office = if let Some(ref in_path) = opts.input {
        if let Ok(bytes) = fs::read(in_path) {
            lexishield::format_adapters::office_adapter::OfficeAdapter::detect_office_type(&bytes)
                != lexishield::format_adapters::office_adapter::OfficeType::Unknown
        } else {
            false
        }
    } else {
        false
    };

    let target_out_path = if let Some(ref out) = opts.output {
        Some(out.clone())
    } else if opts.copy
        && let Some(ref in_path) = opts.input
    {
        let stem = in_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("archivo");
        let ext = in_path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("txt");
        let suffix = opts.suffix.as_deref().unwrap_or("_obfuscated");
        Some(in_path.with_file_name(format!("{stem}{suffix}.{ext}")))
    } else {
        None
    };

    if is_office {
        let in_path = opts.input.as_ref().unwrap();
        let out_path = target_out_path.unwrap_or_else(|| {
            let stem = in_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("documento");
            let ext = in_path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("docx");
            in_path.with_file_name(format!("{stem}_sanitized.{ext}"))
        });

        let mut engine = ObfuscatorEngine::new(config);
        if opts.clean_cache {
            engine.clean_cache();
        }
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

        let newly_added = engine.scan_file(in_path)?;
        log::info!(
            "Detectados {} NUEVOS elementos sensibles en documento Office.",
            newly_added.len()
        );

        let report = engine.process_file(in_path, &out_path, opts.format, false)?;

        if let Some(map_path) = map_path {
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

        println!(
            "✅ Documento Office sanitizado con éxito en: {}",
            out_path.display()
        );
        log::info!(
            "Informe: Reemplazos: {} | Formato: Office OpenXML | Tiempo: {:.2}ms",
            report.replacements_applied,
            report.elapsed_ms
        );

        if let Some(ref md_arg) = opts.export_md {
            export_office_to_md_with_pandoc(&out_path, md_arg)?;
        }

        return Ok(());
    }

    let content = resolve_input_content(
        opts.input.as_deref(),
        opts.text.as_deref(),
        opts.clipboard,
        opts.stdin,
    )?;

    let mut engine = ObfuscatorEngine::new(config);
    if opts.clean_cache {
        engine.clean_cache();
    }

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
    } else if let Some(out_path) = target_out_path {
        fs::write(&out_path, &result)?;
        log::info!("Resultado guardado en: {}", out_path.display());
    } else {
        println!("{}", result);
    }

    if opts.export_md.is_some() {
        println!(
            "ℹ️ El archivo procesado es texto plano o Markdown; no se requiere conversión con Pandoc."
        );
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
    stdin: bool,
    output: Option<PathBuf>,
    text: Option<String>,
    clipboard: bool,
    mappings: Option<PathBuf>,
    password: Option<String>,
    ask_password: bool,
    format: FormatType,
    export_md: Option<String>,
}

/// Manejador de la acción de desofuscación.
fn handle_deobfuscate(
    config: LexiConfig,
    opts: DeobfuscateOptions,
) -> Result<(), Box<dyn std::error::Error>> {
    let is_office = if let Some(ref in_path) = opts.input {
        if let Ok(bytes) = fs::read(in_path) {
            lexishield::format_adapters::office_adapter::OfficeAdapter::detect_office_type(&bytes)
                != lexishield::format_adapters::office_adapter::OfficeType::Unknown
        } else {
            false
        }
    } else {
        false
    };

    if is_office {
        let in_path = opts.input.as_ref().unwrap();
        let out_path = opts.output.clone().unwrap_or_else(|| {
            let stem = in_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("documento");
            let ext = in_path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("docx");
            in_path.with_file_name(format!("{stem}_restored.{ext}"))
        });

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

        let report = engine.process_file(in_path, &out_path, opts.format, true)?;

        println!(
            "✅ Documento Office desofuscado con éxito en: {}",
            out_path.display()
        );
        log::info!(
            "Informe: Reemplazos aplicados: {} | Formato: Office OpenXML | Tiempo: {:.2}ms",
            report.replacements_applied,
            report.elapsed_ms
        );

        if let Some(ref md_arg) = opts.export_md {
            export_office_to_md_with_pandoc(&out_path, md_arg)?;
        }

        return Ok(());
    }

    let content = resolve_input_content(
        opts.input.as_deref(),
        opts.text.as_deref(),
        opts.clipboard,
        opts.stdin,
    )?;

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
        log::info!("Resultado guardado en: {}", out_path.display());
    } else {
        println!("{}", result);
    }

    log::info!(
        "Informe: Reemplazos aplicados: {} | Formato: {:?} | Tiempo: {:.2}ms",
        report.replacements_applied,
        report.format_detected,
        report.elapsed_ms
    );

    Ok(())
}

/// Recopila archivos a procesar usando el motor de ignorados estilo ripgrep (.gitignore, .lexiignore y extensiones configuradas).
fn collect_files_with_ignore(
    dir: &Path,
    ignore_cfg: &lexishield::config::IgnoreConfig,
    recursive: bool,
    git_ignore_enabled: bool,
) -> std::io::Result<Vec<PathBuf>> {
    if !dir.is_dir() {
        return Ok(vec![dir.to_path_buf()]);
    }

    use ignore::WalkBuilder;
    let mut builder = WalkBuilder::new(dir);
    builder.add_custom_ignore_filename(".lexiignore");
    builder.hidden(true);
    builder.git_ignore(git_ignore_enabled);
    builder.git_global(git_ignore_enabled);
    builder.git_exclude(git_ignore_enabled);
    if !recursive {
        builder.max_depth(Some(1));
    }

    let mut files = Vec::new();
    for result in builder.build() {
        match result {
            Ok(entry) => {
                let path = entry.path();
                if path.is_file() {
                    let in_ignored_dir = path.components().any(|c| {
                        if let std::path::Component::Normal(os_name) = c
                            && let Some(s) = os_name.to_str()
                        {
                            return ignore_cfg
                                .ignored_directories
                                .iter()
                                .any(|d| d.eq_ignore_ascii_case(s));
                        }
                        false
                    });
                    if in_ignored_dir {
                        continue;
                    }

                    if let Some(ext) = path.extension().and_then(|e| e.to_str())
                        && ignore_cfg
                            .ignored_extensions
                            .iter()
                            .any(|e| e.eq_ignore_ascii_case(ext))
                    {
                        continue;
                    }

                    files.push(path.to_path_buf());
                }
            }
            Err(err) => {
                log::warn!("Error al explorar ruta: {err}");
            }
        }
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
            "pdf", "db", "sqlite", "sqlite3", "mdb", // Tipografías
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
                // Comprobar si es un contenedor Office OpenXML (.docx, .xlsx, .pptx)
                if let Ok(bytes) = std::fs::read(path)
                    && lexishield::format_adapters::office_adapter::OfficeAdapter::detect_office_type(&bytes)
                        != lexishield::format_adapters::office_adapter::OfficeType::Unknown
                {
                    return false;
                }
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
    prefix_tag: Option<&str>,
    display_name: Option<&str>,
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
        if let (Some(tag), Some(name)) = (prefix_tag, display_name) {
            println!(
                "\r  {}[ {} ] Omitido (formato binario o comprimido no admitido): {} \x1b[K",
                tag,
                format_size(total_bytes),
                name
            );
        }
        return Ok(Vec::new());
    }

    // Comprobación híbrida en caché de 2 fases (Fast-path metadatos + Hash parcial)
    let mut scan_cache = lexishield::cache::ScanCache::load();
    let current_rules_hash = lexishield::cache::ScanCache::compute_rules_hash(&engine.config);
    let cache_check = scan_cache.check_file(path, current_rules_hash);
    if let lexishield::cache::CacheCheckResult::Skip { entities_found } = cache_check {
        if let (Some(tag), Some(name)) = (prefix_tag, display_name) {
            println!(
                "\r  {}[ {} ] En caché (sin cambios): {} ({} entidades) \x1b[K",
                tag,
                format_size(total_bytes),
                name,
                entities_found
            );
        }
        return Ok(Vec::new());
    }

    // Si es un documento Office OpenXML, usar el extractor especializado
    if let Ok(bytes) = std::fs::read(path)
        && lexishield::format_adapters::office_adapter::OfficeAdapter::detect_office_type(&bytes)
            != lexishield::format_adapters::office_adapter::OfficeType::Unknown
    {
        if let (Some(tag), Some(name)) = (prefix_tag, display_name) {
            print!(
                "\r  {}[ {} ] Analizando documento Office: {} \x1b[K",
                tag,
                format_size(total_bytes),
                name
            );
            let _ = std::io::stdout().flush();
        }
        let mappings = engine.scan_file(path)?;
        scan_cache.record_file(path, current_rules_hash, mappings.len());
        let _ = scan_cache.save();
        if let (Some(tag), Some(name)) = (prefix_tag, display_name) {
            println!(
                "\r  {}[ {} ] Analizado (Office): {} ({} entidades) \x1b[K",
                tag,
                format_size(total_bytes),
                name,
                mappings.len()
            );
        }
        return Ok(mappings);
    }

    let mut reader = BufReader::with_capacity(64 * 1024, file);
    let mut newly_added = Vec::new();
    let mut bytes_read = 0u64;
    let mut last_update = std::time::Instant::now();
    let mut byte_buf = Vec::new();

    if let (Some(tag), Some(name)) = (prefix_tag, display_name) {
        print!(
            "\r  {}[ 0 B / {} ] (0.0%) Analizando: {} \x1b[K",
            tag,
            format_size(total_bytes),
            name
        );
        let _ = std::io::stdout().flush();
    }

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
            if let (Some(tag), Some(name)) = (prefix_tag, display_name) {
                print!(
                    "\r  {}[ {} / {} ] ({:.1}%) Analizando: {} \x1b[K",
                    tag,
                    format_size(bytes_read),
                    format_size(total_bytes),
                    pct,
                    name
                );
                let _ = std::io::stdout().flush();
            }
            last_update = std::time::Instant::now();
        }
    }

    scan_cache.record_file(path, current_rules_hash, newly_added.len());
    let _ = scan_cache.save();

    if let (Some(tag), Some(name)) = (prefix_tag, display_name) {
        println!(
            "\r  {}[ {} / {} ] (100.0%) Analizado: {} \x1b[K",
            tag,
            format_size(total_bytes),
            format_size(total_bytes),
            name
        );
    }

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

    if opts.stdin || opts.input.as_ref().is_some_and(|p| p.as_os_str() == "-") {
        let content = resolve_input_content(
            opts.input.as_deref(),
            opts.text.as_deref(),
            opts.clipboard,
            opts.stdin,
        )?;
        println!(
            "🔍 Analizando flujo de entrada estándar stdin ({})...",
            format_size(content.len() as u64)
        );
        let mut text_new = engine.scan_and_register_mappings(&content)?;
        if !text_new.is_empty() {
            total_discovered += text_new.len();
            if !opts.yes {
                prompt_file_mappings("stdin", &mut text_new, &mut engine)?;
            }
        }
    } else if let Some(ref path) = opts.input {
        if path.is_dir() {
            let files = collect_files_with_ignore(
                path,
                &engine.config.ignore,
                opts.recursive,
                !opts.no_git_ignore,
            )?;
            let total = files.len();
            println!(
                "🔍 Escaneando directorio '{}' ({} archivos)...",
                path.display(),
                total
            );
            if opts.yes {
                use indicatif::{ProgressBar, ProgressStyle};
                let pb = ProgressBar::new(total as u64);
                pb.set_style(ProgressStyle::default_bar()
                    .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta})").unwrap()
                    .progress_chars("#>-"));

                let cfg_arc = engine.config.clone();
                let all_results: Vec<Vec<Mapping>> = files
                    .par_iter()
                    .map(|f| {
                        let mut local_engine = ObfuscatorEngine::new(cfg_arc.clone());
                        let res = scan_file_with_progress(&mut local_engine, f, None, None)
                            .unwrap_or_default();
                        pb.inc(1);
                        res
                    })
                    .collect();
                pb.finish_with_message("Escaneo multihilo completado.");

                let mut all_mappings = Vec::new();
                for mut batch in all_results {
                    all_mappings.append(&mut batch);
                }
                if !all_mappings.is_empty() {
                    let added = engine.manager.load_mappings(all_mappings)?;
                    total_discovered += added.len();
                }
            } else {
                for (idx, f) in files.iter().enumerate() {
                    let current = idx + 1;
                    let rel_path = f.strip_prefix(path).unwrap_or(f);
                    let prefix_tag = format!("[{}/{}] ", current, total);
                    let mut file_new = scan_file_with_progress(
                        &mut engine,
                        f,
                        Some(&prefix_tag),
                        Some(&rel_path.display().to_string()),
                    )?;

                    if !file_new.is_empty() {
                        total_discovered += file_new.len();
                        if !opts.yes {
                            let disp = format!("[{}/{}] {}", current, total, rel_path.display());
                            prompt_file_mappings(&disp, &mut file_new, &mut engine)?;
                        }
                    }
                }
            }
        } else {
            println!("🔍 Escaneando archivo '{}'...", path.display());
            let mut file_new = scan_file_with_progress(
                &mut engine,
                path,
                Some(""),
                Some(&path.display().to_string()),
            )?;
            if !file_new.is_empty() {
                total_discovered += file_new.len();
                if !opts.yes {
                    prompt_file_mappings(&path.display().to_string(), &mut file_new, &mut engine)?;
                }
            }
        }
    } else {
        let content = resolve_input_content(
            opts.input.as_deref(),
            opts.text.as_deref(),
            opts.clipboard,
            opts.stdin,
        )?;
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
        DictCommands::Update {
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

            let mut found = false;
            for m in &mut list {
                if m.original == original {
                    m.pseudonym = pseudonym.clone();
                    m.omitted = pseudonym == "=";
                    found = true;
                    break;
                }
            }
            if !found {
                list.push(Mapping {
                    original: original.clone(),
                    pseudonym: pseudonym.clone(),
                    detector_type: DetectorType::GenericToken,
                    omitted: pseudonym == "=",
                });
            }

            lexishield::save_mappings_auto(&map_path, &list, pwd.as_deref())?;
            log::info!(
                "Mapeo '{}' actualizado con éxito en {}",
                original,
                map_path.display()
            );
        }
        DictCommands::BatchRemove {
            originals,
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
            list.retain(|m| !originals.contains(&m.original));
            let removed_count = initial_len - list.len();

            lexishield::save_mappings_auto(&map_path, &list, pwd.as_deref())?;
            log::info!(
                "Se eliminaron {} mapeos de la bóveda '{}'.",
                removed_count,
                map_path.display()
            );
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

/// Manejador del comando Vault.
fn handle_vault(subcommand: VaultCommands) -> Result<(), Box<dyn std::error::Error>> {
    match subcommand {
        VaultCommands::List => {
            let vaults = lexishield::vault::list_vaults()?;
            println!("🗄️  Bóvedas disponibles en ~/.lexishield:");
            println!(
                "{:<28} {:<12} {:<14} {:<20}",
                "Nombre", "Tamaño", "Estado", "Última Modificación"
            );
            println!("{}", "-".repeat(78));
            if vaults.is_empty() {
                println!("  (No hay bóvedas creadas todavía)");
            } else {
                for v in vaults {
                    let status = if v.is_encrypted {
                        "🔒 Cifrada"
                    } else {
                        "🔓 Plana"
                    };
                    let mod_time = v.last_modified.unwrap_or_else(|| "N/A".into());
                    println!(
                        "{:<28} {:<12} {:<14} {:<20}",
                        v.name, v.size_formatted, status, mod_time
                    );
                }
            }
        }
        VaultCommands::Create {
            name,
            password,
            ask_password: _,
        } => {
            let pwd = if let Some(p) = password {
                if p.trim().is_empty() {
                    prompt_new_password(&format!(
                        "Introduce la contraseña obligatoria para la nueva bóveda '{name}': "
                    ))?
                } else {
                    p
                }
            } else {
                prompt_new_password(&format!(
                    "Introduce la contraseña obligatoria para la nueva bóveda '{name}': "
                ))?
            };
            let path = lexishield::vault::create_vault(&name, Some(&pwd))?;
            println!("✅ Bóveda cifrada creada con éxito en: {}", path.display());
        }
        VaultCommands::Rename { old_name, new_name } => {
            let path = lexishield::vault::rename_vault(&old_name, &new_name)?;
            println!("✅ Bóveda renombrada a: {}", path.display());
        }
        VaultCommands::Delete { name, yes } => {
            if !yes {
                use std::io::{self, Write};
                print!(
                    "⚠️  ¿Seguro que deseas eliminar la bóveda '{}'? [s/N]: ",
                    name
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
            lexishield::vault::delete_vault(&name)?;
            println!("✅ Bóveda '{}' eliminada con éxito.", name);
        }
        VaultCommands::Export { name, destination } => {
            lexishield::vault::export_vault(&name, &destination)?;
            println!(
                "✅ Bóveda '{}' exportada a: {}",
                name,
                destination.display()
            );
        }
        VaultCommands::Import { source, name } => {
            let path = lexishield::vault::import_vault(&source, name.as_deref())?;
            println!("✅ Bóveda importada con éxito a: {}", path.display());
        }
        VaultCommands::RestoreBackup { name } => {
            let path = lexishield::vault::restore_vault_backup(&name)?;
            println!(
                "✅ Bóveda '{}' recuperada exitosamente desde su copia (.bak) en: {}",
                name,
                path.display()
            );
        }
    }
    Ok(())
}

/// Manejador del comando Backup (tar.bz2 + cifrado AES).
fn handle_backup(subcommand: BackupCommands) -> Result<(), Box<dyn std::error::Error>> {
    match subcommand {
        BackupCommands::Create {
            output,
            password,
            ask_password,
        } => {
            let out_path = if let Some(p) = output {
                p
            } else {
                let config_dir = lexishield::config::get_user_config_dir()?;
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                config_dir.join(format!("backup_lexishield_{ts}.tar.bz2"))
            };

            let pwd = resolve_password(password, ask_password, None)?;
            let res = lexishield::backup::create_backup(&out_path, pwd.as_deref())?;
            println!(
                "📦 Copia de seguridad Bzip2 creada con éxito en: {}",
                res.display()
            );
        }
        BackupCommands::Restore {
            input,
            password,
            ask_password,
            yes,
        } => {
            if !yes {
                use std::io::{self, Write};
                print!(
                    "⚠️  ¿Restaurar copia de seguridad desde '{}'? Se sobrescribirán configuraciones y bóvedas con el mismo nombre. [s/N]: ",
                    input.display()
                );
                io::stdout().flush()?;
                let mut input_str = String::new();
                io::stdin().read_line(&mut input_str)?;
                let trimmed = input_str.trim().to_lowercase();
                if trimmed != "s" && trimmed != "si" && trimmed != "y" && trimmed != "yes" {
                    log::info!("Operación cancelada.");
                    return Ok(());
                }
            }

            let pwd = resolve_password(password, ask_password, Some(&input))?;
            let count = lexishield::backup::restore_backup(&input, pwd.as_deref())?;
            println!("✅ Copia de seguridad restaurada con éxito: {count} archivos recuperados.");
        }
    }
    Ok(())
}

/// Manejador del comando Clipboard.
fn handle_clipboard(subcommand: ClipboardCommands) -> Result<(), Box<dyn std::error::Error>> {
    match subcommand {
        ClipboardCommands::Clear => {
            lexishield::clipboard::clear_clipboard()?;
            println!("🧹 Portapapeles vaciado con éxito.");
        }
        ClipboardCommands::Get => {
            let text = lexishield::clipboard::get_clipboard_text()?;
            println!("{text}");
        }
    }
    Ok(())
}

fn handle_config(
    config_path_override: Option<&Path>,
    subcommand: ConfigCommands,
) -> Result<(), Box<dyn std::error::Error>> {
    let cfg_path = match config_path_override {
        Some(p) => p.to_path_buf(),
        None => lexishield::config::ensure_user_config_initialized()?,
    };

    match subcommand {
        ConfigCommands::Path => {
            println!("{}", cfg_path.display());
        }
        ConfigCommands::Show => {
            println!("⚙️  Archivo de configuración: {}", cfg_path.display());
            println!("--------------------------------------------------");
            let content = fs::read_to_string(&cfg_path)?;
            println!("{}", content);
        }
        ConfigCommands::Reset { yes } => {
            if !yes {
                use std::io::{self, Write};
                print!(
                    "⚠️  ¿Restablecer la configuración en '{}' a los valores por defecto? [s/N]: ",
                    cfg_path.display()
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
            let restored = lexishield::config::reset_config()?;
            println!(
                "✅ Configuración restablecida con éxito en: {}",
                restored.display()
            );
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    init_logger();
    if cli.verbose || cli.debug {
        lexishield::logger::set_debug_mode(true);
    }
    let config = load_config_with_rules(cli.config.as_deref(), cli.rules.as_deref());

    match cli.command {
        Commands::Config { subcommand } => handle_config(cli.config.as_deref(), subcommand),
        Commands::Vault { subcommand } => handle_vault(subcommand),
        Commands::Backup { subcommand } => handle_backup(subcommand),
        Commands::Clipboard { subcommand } => handle_clipboard(subcommand),
        Commands::Obfuscate {
            input,
            stdin,
            output,
            copy,
            suffix,
            clean_cache,
            text,
            clipboard,
            format,
            save_mappings,
            password,
            ask_password,
            no_save,
            export_md,
        } => handle_obfuscate(
            config,
            ObfuscateOptions {
                input,
                stdin,
                output,
                copy,
                suffix,
                clean_cache,
                text,
                clipboard,
                format: format.into(),
                save_mappings,
                password,
                ask_password,
                no_save,
                export_md,
            },
        ),

        Commands::Deobfuscate {
            input,
            stdin,
            output,
            text,
            clipboard,
            mappings,
            password,
            ask_password,
            format,
            export_md,
        } => handle_deobfuscate(
            config,
            DeobfuscateOptions {
                input,
                stdin,
                output,
                text,
                clipboard,
                mappings,
                password,
                ask_password,
                format: format.into(),
                export_md,
            },
        ),

        Commands::Scan {
            input,
            recursive,
            no_git_ignore,
            stdin,
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
                recursive,
                no_git_ignore,
                stdin,
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
        Commands::Completions { shell } => {
            let mut cmd = Cli::command();
            clap_complete::generate(shell, &mut cmd, "lexishield", &mut std::io::stdout());
            Ok(())
        }
    }
}
