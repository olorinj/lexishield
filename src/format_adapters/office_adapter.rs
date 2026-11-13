//! Adaptador de formato para documentos Microsoft Office OpenXML (.docx, .xlsx, .pptx).
//!
//! Desempaqueta contenedores ZIP, parsea los flujos XML internos (<w:t>, sharedStrings, <a:t>)
//! mediante streaming (quick-xml) y sustituye las entidades confidenciales preservando al 100%
//! los estilos, tablas, fuentes, márgenes e imágenes sin corromper el formato.

use crate::format_adapters::xml_adapter::XmlAdapter;
use crate::models::ObfuscationError;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::io::{Cursor, Read, Write};
use std::sync::LazyLock;
use zip::write::SimpleFileOptions;
use zip::{ZipArchive, ZipWriter};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfficeType {
    Word,
    Excel,
    PowerPoint,
    Unknown,
}

#[derive(Default)]
pub struct OfficeAdapter {
    xml_adapter: XmlAdapter,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ZipResourceMetrics {
    pub compressed_bytes: u64,
    pub uncompressed_bytes: u64,
    pub compression_ratio: f64,
    pub available_memory_bytes: Option<u64>,
    pub requires_warning: bool,
}

impl OfficeAdapter {
    pub fn new() -> Self {
        Self {
            xml_adapter: XmlAdapter::new(),
        }
    }

    /// Calcula el tamaño descomprimido proyectado de un contenedor ZIP y evalúa si puede afectar a la memoria disponible del sistema.
    #[allow(clippy::cast_precision_loss)]
    pub fn inspect_zip_resources(bytes: &[u8]) -> Result<ZipResourceMetrics, ObfuscationError> {
        let compressed_bytes = bytes.len() as u64;
        let cursor = Cursor::new(bytes);
        let mut archive = ZipArchive::new(cursor).map_err(|e| {
            ObfuscationError::XmlError(format!("Error al inspeccionar contenedor ZIP: {e}"))
        })?;

        let mut uncompressed_bytes: u64 = 0;
        for i in 0..archive.len() {
            if let Ok(file) = archive.by_index_raw(i) {
                uncompressed_bytes = uncompressed_bytes.saturating_add(file.size());
            }
        }

        let compression_ratio = if compressed_bytes > 0 {
            uncompressed_bytes as f64 / compressed_bytes as f64
        } else {
            1.0
        };

        let available_memory_bytes = Self::get_system_available_memory_bytes();

        // Se requiere advertencia al usuario si:
        // 1. El tamaño descomprimido supera el 50% de la RAM disponible (si se conoce), o
        // 2. El tamaño descomprimido supera 500 MB, o
        // 3. El tamaño descomprimido supera 50 MB con un ratio de compresión extremo (> 100:1)
        let exceeds_ram_share =
            available_memory_bytes.is_some_and(|avail| uncompressed_bytes > avail / 2);
        let requires_warning = exceeds_ram_share
            || uncompressed_bytes > 500 * 1024 * 1024
            || (uncompressed_bytes > 50 * 1024 * 1024 && compression_ratio > 100.0);

        Ok(ZipResourceMetrics {
            compressed_bytes,
            uncompressed_bytes,
            compression_ratio,
            available_memory_bytes,
            requires_warning,
        })
    }

    /// Obtiene una estimación de la memoria RAM disponible en el sistema sin dependencias pesadas.
    pub fn get_system_available_memory_bytes() -> Option<u64> {
        #[cfg(target_os = "linux")]
        {
            if let Ok(meminfo) = std::fs::read_to_string("/proc/meminfo") {
                for line in meminfo.lines() {
                    if line.starts_with("MemAvailable:") {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 2
                            && let Ok(kb) = parts[1].parse::<u64>()
                        {
                            return Some(kb.saturating_mul(1024));
                        }
                    }
                }
            }
            None
        }
        #[cfg(target_os = "windows")]
        {
            #[repr(C)]
            struct MemoryStatusEx {
                dw_length: u32,
                dw_memory_load: u32,
                ull_total_phys: u64,
                ull_avail_phys: u64,
                ull_total_page_file: u64,
                ull_avail_page_file: u64,
                ull_total_virtual: u64,
                ull_avail_virtual: u64,
                ull_avail_extended_virtual: u64,
            }

            #[link(name = "kernel32")]
            unsafe extern "system" {
                fn GlobalMemoryStatusEx(lp_buffer: *mut MemoryStatusEx) -> i32;
            }

            let mut status = MemoryStatusEx {
                dw_length: std::mem::size_of::<MemoryStatusEx>() as u32,
                dw_memory_load: 0,
                ull_total_phys: 0,
                ull_avail_phys: 0,
                ull_total_page_file: 0,
                ull_avail_page_file: 0,
                ull_total_virtual: 0,
                ull_avail_virtual: 0,
                ull_avail_extended_virtual: 0,
            };

            // SAFETY: `status` is a properly initialized `#[repr(C)]` struct on the stack
            // with `dw_length` set to its exact byte size as required by the Win32 API.
            let ok = unsafe { GlobalMemoryStatusEx(&raw mut status) };
            if ok != 0 {
                return Some(status.ull_avail_phys);
            }
            None
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        {
            None
        }
    }

    /// Detecta el tipo de documento Office analizando la estructura interna del contenedor ZIP.
    pub fn detect_office_type(bytes: &[u8]) -> OfficeType {
        let cursor = Cursor::new(bytes);
        let Ok(archive) = ZipArchive::new(cursor) else {
            return OfficeType::Unknown;
        };

        for name in archive.file_names() {
            if name.starts_with("word/") || name == "word/document.xml" {
                return OfficeType::Word;
            }
            if name.starts_with("xl/") || name == "xl/workbook.xml" {
                return OfficeType::Excel;
            }
            if name.starts_with("ppt/") || name == "ppt/presentation.xml" {
                return OfficeType::PowerPoint;
            }
        }

        OfficeType::Unknown
    }

    /// Comprueba si el archivo XML interno contiene texto legible por el usuario que deba ser sanitizado.
    fn is_translatable_xml(name: &str, office_type: OfficeType) -> bool {
        let is_xml = std::path::Path::new(name)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("xml"));
        if !is_xml {
            return false;
        }

        let name_lower = name.to_ascii_lowercase();
        match office_type {
            OfficeType::Word => {
                name_lower.starts_with("word/")
                    && (name_lower.contains("document")
                        || name_lower.contains("header")
                        || name_lower.contains("footer")
                        || name_lower.contains("footnote")
                        || name_lower.contains("endnote")
                        || name_lower.contains("comment"))
            }
            OfficeType::Excel => {
                name_lower.starts_with("xl/")
                    && (name_lower.contains("sharedstrings")
                        || name_lower.contains("sheet")
                        || name_lower.contains("comment")
                        || name_lower.contains("drawing"))
            }
            OfficeType::PowerPoint => {
                name_lower.starts_with("ppt/")
                    && (name_lower.contains("slide")
                        || name_lower.contains("notesslide")
                        || name_lower.contains("comment"))
            }
            OfficeType::Unknown => true,
        }
    }

    fn strip_rpr_metadata(rpr: &str) -> String {
        static METADATA_RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"<w:lang\b[^>]*/>|<w:noProof\b[^>]*/>|<w:rsidRPr\b[^>]*/>")
                .expect("Regex metadata inválida")
        });
        static EMPTY_RPR_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"<w:rPr>\s*</w:rPr>").expect("Regex empty rpr inválida"));
        static WS_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"\s+").expect("Regex whitespace inválida"));
        let without_metadata = METADATA_RE.replace_all(rpr, "");
        let without_empty_rpr = EMPTY_RPR_RE.replace_all(&without_metadata, "");
        WS_RE
            .replace_all(without_empty_rpr.trim(), " ")
            .into_owned()
    }

    fn are_rpr_compatible(rpr1: &str, rpr2: &str, t2: &str) -> bool {
        let s1 = Self::strip_rpr_metadata(rpr1);
        let s2 = Self::strip_rpr_metadata(rpr2);
        if s1 == s2 {
            return true;
        }
        // Si el segundo run es puntuación pura o espacios y no define estilos visuales propios,
        // se fusiona con el run anterior para preservar la continuidad semántica del token.
        if s2.is_empty()
            && t2
                .chars()
                .all(|c| c.is_ascii_punctuation() || c.is_whitespace())
        {
            return true;
        }
        false
    }

    fn consolidate_paragraph_runs(paragraph: &str) -> String {
        static CLEANUP_RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"<w:proofErr\b[^>]*/>|<w:lastRenderedPageBreak\b[^>]*/>")
                .expect("Regex cleanup inválida")
        });
        static ADJACENT_TEXT_NODES_RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"(?s)<w:t(?:\b[^>]*)>([^<]*)</w:t>\s*<w:t(?:\b[^>]*)>([^<]*)</w:t>")
                .expect("Regex adjacent text nodes inválida")
        });
        static ADJACENT_RUNS_RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
                r"(?s)<w:r(\b[^>]*)>\s*(<w:rPr>.*?</w:rPr>)?\s*(<w:br\b[^>]*/>)?\s*<w:t(?:\b[^>]*)>([^<]*)</w:t>\s*</w:r>(\s*(?:<(?:w:bookmark(?:Start|End)|w:commentRange(?:Start|End))\b[^>]*/>\s*)*)<w:r(?:\b[^>]*)>\s*(<w:rPr>.*?</w:rPr>)?\s*<w:t(?:\b[^>]*)>([^<]*)</w:t>\s*(<w:br\b[^>]*/>)?\s*</w:r>",
            )
            .expect("Regex adjacent runs inválida")
        });

        let cleaned = CLEANUP_RE.replace_all(paragraph, "");

        let mut current = cleaned.into_owned();
        loop {
            let replaced =
                ADJACENT_TEXT_NODES_RE.replace_all(&current, |caps: &regex::Captures| {
                    format!(
                        r#"<w:t xml:space="preserve">{}{}</w:t>"#,
                        &caps[1], &caps[2]
                    )
                });
            if replaced == current {
                break;
            }
            current = replaced.into_owned();
        }

        loop {
            let replaced = ADJACENT_RUNS_RE.replace_all(&current, |caps: &regex::Captures| {
                let attrs = caps.get(1).map_or("", |m| m.as_str());
                let rpr1 = caps.get(2).map_or("", |m| m.as_str());
                let leading_br = caps.get(3).map_or("", |m| m.as_str());
                let t1 = &caps[4];
                let intermediate_tags = caps.get(5).map_or("", |m| m.as_str());
                let rpr2 = caps.get(6).map_or("", |m| m.as_str());
                let t2 = &caps[7];
                let trailing_br = caps.get(8).map_or("", |m| m.as_str());

                if !Self::are_rpr_compatible(rpr1, rpr2, t2) {
                    return caps[0].to_string();
                }

                format!(
                    r#"<w:r{attrs}>{rpr1}{leading_br}<w:t xml:space="preserve">{t1}{t2}</w:t>{trailing_br}</w:r>{intermediate_tags}"#
                )
            });
            if replaced == current {
                break;
            }
            current = replaced.into_owned();
        }

        current
    }

    /// Desfragmenta y normaliza los elementos de texto en XML de Word (.docx).
    /// Elimina marcas de corrección ortográfica (<w:proofErr>) y saltos de página cacheados que fracturan palabras,
    /// y une etiquetas adyacentes <w:r><w:t> dentro del mismo párrafo (incluso con atributos w:rsidR, propiedades <w:rPr>
    /// o saltos de línea <w:br/>) para evitar falsos negativos en fronteras de palabras y signos de puntuación terminales.
    pub fn consolidate_word_xml_runs(xml: &str) -> String {
        static PARAGRAPH_RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"(?s)<w:p\b[^>]*>.*?</w:p>").expect("Regex paragraph inválida")
        });

        PARAGRAPH_RE
            .replace_all(xml, |caps: &regex::Captures| {
                Self::consolidate_paragraph_runs(&caps[0])
            })
            .into_owned()
    }

    /// Extrae todo el texto plano de un documento Office para permitir su escaneo e identificación de entidades.
    pub fn extract_text(&self, bytes: &[u8]) -> Result<String, ObfuscationError> {
        let cursor = Cursor::new(bytes);
        let mut archive = ZipArchive::new(cursor).map_err(|e| {
            ObfuscationError::XmlError(format!("Error al abrir archivo Office (ZIP): {e}"))
        })?;

        let office_type = Self::detect_office_type(bytes);
        let mut extracted_chunks = Vec::new();

        for i in 0..archive.len() {
            let mut file = archive.by_index(i).map_err(|e| {
                ObfuscationError::XmlError(format!("Error leyendo entrada ZIP: {e}"))
            })?;
            let file_name = file.name().to_string();

            if Self::is_translatable_xml(&file_name, office_type) {
                let mut xml_content = String::new();
                if file.read_to_string(&mut xml_content).is_ok() {
                    let normalized = if office_type == OfficeType::Word {
                        Self::consolidate_word_xml_runs(&xml_content)
                    } else {
                        xml_content
                    };

                    let mut reader = Reader::from_str(&normalized);
                    reader.config_mut().trim_text(false);
                    let mut buf = Vec::new();
                    let mut current_block = String::new();

                    while let Ok(event) = reader.read_event_into(&mut buf) {
                        match event {
                            Event::Eof => break,
                            Event::Text(t) => {
                                if let Ok(decoded) = std::str::from_utf8(&t)
                                    && let Ok(raw) = quick_xml::escape::unescape(decoded)
                                {
                                    current_block.push_str(&raw);
                                }
                            }
                            Event::End(ref e) => {
                                let local = e.local_name();
                                let tag = local.as_ref();
                                if tag == b"p" || tag == b"tc" || tag == b"si" || tag == b"row" {
                                    let trimmed = current_block.trim();
                                    if !trimmed.is_empty() {
                                        extracted_chunks.push(trimmed.to_string());
                                    }
                                    current_block.clear();
                                }
                            }
                            _ => {}
                        }
                        buf.clear();
                    }
                    let trimmed = current_block.trim();
                    if !trimmed.is_empty() {
                        extracted_chunks.push(trimmed.to_string());
                    }
                }
            }
        }

        Ok(extracted_chunks.join("\n"))
    }

    /// Procesa y ofusca/desofusca un documento Office completo en memoria, preservando 100% de la estructura.
    pub fn process_office(
        &self,
        input_bytes: &[u8],
        replacement_map: &HashMap<String, String>,
        strict_word_boundaries: bool,
    ) -> Result<(Vec<u8>, usize, OfficeType), ObfuscationError> {
        self.process_office_with_substring_keys(
            input_bytes,
            replacement_map,
            strict_word_boundaries,
            &HashSet::new(),
        )
    }

    /// Variante de `process_office` que permite omitir límites de palabra para claves específicas.
    pub fn process_office_with_substring_keys(
        &self,
        input_bytes: &[u8],
        replacement_map: &HashMap<String, String>,
        strict_word_boundaries: bool,
        substring_keys: &HashSet<String>,
    ) -> Result<(Vec<u8>, usize, OfficeType), ObfuscationError> {
        let office_type = Self::detect_office_type(input_bytes);
        if office_type == OfficeType::Unknown {
            return Err(ObfuscationError::XmlError(
                "El archivo proporcionado no es un documento Office OpenXML válido (.docx, .xlsx, .pptx)".into(),
            ));
        }

        let input_cursor = Cursor::new(input_bytes);
        let mut archive = ZipArchive::new(input_cursor)
            .map_err(|e| ObfuscationError::XmlError(format!("Error al descomprimir ZIP: {e}")))?;

        let mut output_bytes = Vec::new();
        let mut total_replacements = 0;

        {
            let output_cursor = Cursor::new(&mut output_bytes);
            let mut writer = ZipWriter::new(output_cursor);

            for i in 0..archive.len() {
                let mut file = archive.by_index(i).map_err(|e| {
                    ObfuscationError::XmlError(format!("Error leyendo entrada ZIP: {e}"))
                })?;
                let file_name = file.name().to_string();
                let options = SimpleFileOptions::default()
                    .compression_method(file.compression())
                    .unix_permissions(file.unix_mode().unwrap_or(0o644));

                writer.start_file(&file_name, options).map_err(|e| {
                    ObfuscationError::XmlError(format!("Error escribiendo entrada ZIP: {e}"))
                })?;

                if Self::is_translatable_xml(&file_name, office_type) {
                    let mut xml_content = String::new();
                    file.read_to_string(&mut xml_content).map_err(|e| {
                        ObfuscationError::XmlError(format!(
                            "Error leyendo XML interno {file_name}: {e}"
                        ))
                    })?;

                    let normalized_xml = if office_type == OfficeType::Word {
                        Self::consolidate_word_xml_runs(&xml_content)
                    } else {
                        xml_content
                    };

                    let (replaced_xml, count) = self.xml_adapter.process_xml_with_substring_keys(
                        &normalized_xml,
                        replacement_map,
                        strict_word_boundaries,
                        substring_keys,
                    )?;
                    total_replacements += count;

                    writer.write_all(replaced_xml.as_bytes()).map_err(|e| {
                        ObfuscationError::XmlError(format!(
                            "Error empaquetando XML {file_name}: {e}"
                        ))
                    })?;
                } else {
                    let mut buffer = Vec::new();
                    file.read_to_end(&mut buffer).map_err(|e| {
                        ObfuscationError::XmlError(format!(
                            "Error copiando archivo {file_name}: {e}"
                        ))
                    })?;

                    writer.write_all(&buffer).map_err(|e| {
                        ObfuscationError::XmlError(format!(
                            "Error empaquetando archivo {file_name}: {e}"
                        ))
                    })?;
                }
            }

            writer.finish().map_err(|e| {
                ObfuscationError::XmlError(format!("Error finalizando contenedor ZIP: {e}"))
            })?;
        }

        Ok((output_bytes, total_replacements, office_type))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_mock_docx(content_xml: &str) -> Vec<u8> {
        let mut buffer = Vec::new();
        {
            let cursor = Cursor::new(&mut buffer);
            let mut writer = ZipWriter::new(cursor);
            let options = SimpleFileOptions::default();

            writer.start_file("[Content_Types].xml", options).unwrap();
            writer
                .write_all(b"<?xml version=\"1.0\"?><Types></Types>")
                .unwrap();

            writer.start_file("word/document.xml", options).unwrap();
            writer.write_all(content_xml.as_bytes()).unwrap();

            writer.finish().unwrap();
        }
        buffer
    }

    fn create_mock_xlsx(shared_strings_xml: &str) -> Vec<u8> {
        let mut buffer = Vec::new();
        {
            let cursor = Cursor::new(&mut buffer);
            let mut writer = ZipWriter::new(cursor);
            let options = SimpleFileOptions::default();

            writer.start_file("xl/workbook.xml", options).unwrap();
            writer
                .write_all(b"<?xml version=\"1.0\"?><workbook></workbook>")
                .unwrap();

            writer.start_file("xl/sharedStrings.xml", options).unwrap();
            writer.write_all(shared_strings_xml.as_bytes()).unwrap();

            writer.finish().unwrap();
        }
        buffer
    }

    #[test]
    fn test_docx_detection_and_extraction() {
        let docx_xml = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:p>
                    <w:r>
                        <w:t>Servidor confidencial: 192.168.1.100 y correo admin@empresa.com</w:t>
                    </w:r>
                </w:p>
            </w:body>
        </w:document>"#;

        let bytes = create_mock_docx(docx_xml);
        assert_eq!(OfficeAdapter::detect_office_type(&bytes), OfficeType::Word);

        let adapter = OfficeAdapter::new();
        let text = adapter.extract_text(&bytes).unwrap();
        assert!(text.contains("192.168.1.100"));
        assert!(text.contains("admin@empresa.com"));
    }

    #[test]
    fn test_docx_obfuscation_and_roundtrip() {
        let docx_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:p>
                    <w:r><w:t>IP sensible: 10.0.0.5</w:t></w:r>
                </w:p>
            </w:body>
        </w:document>"#;

        let original_bytes = create_mock_docx(docx_xml);
        let adapter = OfficeAdapter::new();

        let mut map = HashMap::new();
        map.insert("10.0.0.5".to_string(), "192.168.99.1".to_string());

        // 1. Ofuscar
        let (obfuscated_bytes, count, office_type) = adapter
            .process_office(&original_bytes, &map, false)
            .unwrap();

        assert_eq!(office_type, OfficeType::Word);
        assert_eq!(count, 1);

        let extracted_obf = adapter.extract_text(&obfuscated_bytes).unwrap();
        assert!(extracted_obf.contains("192.168.99.1"));
        assert!(!extracted_obf.contains("10.0.0.5"));

        // 2. Desofuscar (Roundtrip biyectivo)
        let mut reverse_map = HashMap::new();
        reverse_map.insert("192.168.99.1".to_string(), "10.0.0.5".to_string());

        let (restored_bytes, count_rev, _) = adapter
            .process_office(&obfuscated_bytes, &reverse_map, false)
            .unwrap();

        assert_eq!(count_rev, 1);
        let extracted_restored = adapter.extract_text(&restored_bytes).unwrap();
        assert!(extracted_restored.contains("10.0.0.5"));
        assert!(!extracted_restored.contains("192.168.99.1"));
    }

    #[test]
    fn test_xlsx_obfuscation() {
        let shared_strings = r#"<?xml version="1.0" encoding="UTF-8"?>
        <sst xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main">
            <si><t>admin@empresa.com</t></si>
            <si><t>Servidor_Principal</t></si>
        </sst>"#;

        let bytes = create_mock_xlsx(shared_strings);
        assert_eq!(OfficeAdapter::detect_office_type(&bytes), OfficeType::Excel);

        let adapter = OfficeAdapter::new();
        let mut map = HashMap::new();
        map.insert(
            "admin@empresa.com".to_string(),
            "user01@anon.com".to_string(),
        );

        let (obfuscated_bytes, count, office_type) =
            adapter.process_office(&bytes, &map, false).unwrap();

        assert_eq!(office_type, OfficeType::Excel);
        assert_eq!(count, 1);

        let extracted = adapter.extract_text(&obfuscated_bytes).unwrap();
        assert!(extracted.contains("user01@anon.com"));
        assert!(!extracted.contains("admin@empresa.com"));
    }

    #[test]
    fn test_docx_fragmented_runs_and_punctuation() {
        let adapter = OfficeAdapter::new();
        // Simular documento con palabra 'Trampa' cortada por marcas de Word y punto final
        let docx_xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:p>
                    <w:r><w:t>.... </w:t></w:r>
                    <w:proofErr w:type="spellStart"/>
                    <w:r><w:t>Tram</w:t></w:r>
                    <w:proofErr w:type="spellEnd"/>
                    <w:r><w:t>pa.</w:t></w:r>
                </w:p>
            </w:body>
        </w:document>"#;

        let original_bytes = create_mock_docx(docx_xml);
        let extracted = adapter.extract_text(&original_bytes).unwrap();
        assert!(extracted.contains(".... Trampa."));

        let mut map = HashMap::new();
        map.insert("Trampa".to_string(), "kjdhdakjdka".to_string());

        let (obfuscated_bytes, count, _) =
            adapter.process_office(&original_bytes, &map, true).unwrap();

        assert_eq!(count, 1);
        let extracted_obf = adapter.extract_text(&obfuscated_bytes).unwrap();
        assert!(extracted_obf.contains(".... kjdhdakjdka."));
    }

    #[test]
    fn test_docx_end_of_line_period_variations() {
        let adapter = OfficeAdapter::new();
        let mut map = HashMap::new();
        map.insert("Madrid".to_string(), "CiudadAnonima".to_string());

        // Variación 1: En una frase normal
        let xml1 = r#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body><w:p><w:r><w:t>En Madrid hace sol.</w:t></w:r></w:p></w:body>
        </w:document>"#;
        let bytes1 = create_mock_docx(xml1);
        let (res1, c1, _) = adapter.process_office(&bytes1, &map, true).unwrap();
        assert_eq!(c1, 1, "Fallo en frase normal");
        assert!(
            adapter
                .extract_text(&res1)
                .unwrap()
                .contains("En CiudadAnonima hace sol.")
        );

        // Variación 2: Al final de frase con punto en el mismo run
        let xml2 = r#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body><w:p><w:r><w:t>La sede está en Madrid.</w:t></w:r></w:p></w:body>
        </w:document>"#;
        let bytes2 = create_mock_docx(xml2);
        let (res2, c2, _) = adapter.process_office(&bytes2, &map, true).unwrap();
        assert_eq!(c2, 1, "Fallo al final de frase con punto en mismo run");
        assert!(
            adapter
                .extract_text(&res2)
                .unwrap()
                .contains("La sede está en CiudadAnonima.")
        );

        // Variación 3: Al final de frase donde el punto está en un run separado por formato o idioma
        let xml3 = r#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:p>
                    <w:r><w:t>La sede está en </w:t></w:r>
                    <w:r><w:rPr><w:lang w:val="es-ES"/></w:rPr><w:t>Madrid</w:t></w:r>
                    <w:r><w:t>.</w:t></w:r>
                </w:p>
            </w:body>
        </w:document>"#;
        let bytes3 = create_mock_docx(xml3);
        let (res3, c3, _) = adapter.process_office(&bytes3, &map, true).unwrap();
        assert_eq!(c3, 1, "Fallo en punto separado en otro run");
        assert!(
            adapter
                .extract_text(&res3)
                .unwrap()
                .contains("La sede está en CiudadAnonima.")
        );

        // Variación 4: Con salto de línea de Word <w:br/> justo tras el punto
        let xml4 = r#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:p>
                    <w:r><w:t>La sede está en Madrid.</w:t><w:br/></w:r>
                </w:p>
            </w:body>
        </w:document>"#;
        let bytes4 = create_mock_docx(xml4);
        let (res4, c4, _) = adapter.process_office(&bytes4, &map, true).unwrap();
        assert_eq!(c4, 1, "Fallo con salto de línea <w:br/>");
        assert!(
            adapter
                .extract_text(&res4)
                .unwrap()
                .contains("La sede está en CiudadAnonima.")
        );

        // Variación 5: Palabra fragmentada por Word al final de frase (Madri + d.) con rsidR, rPr y bookmarks
        let xml5 = r#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:p>
                    <w:r w:rsidR="00A12345">
                        <w:rPr><w:lang w:val="es-ES"/></w:rPr>
                        <w:t>El viaje termina en Madri</w:t>
                    </w:r>
                    <w:bookmarkStart w:id="0" w:name="_GoBack"/>
                    <w:bookmarkEnd w:id="0"/>
                    <w:r w:rsidR="00B23456">
                        <w:rPr><w:lang w:val="es-ES"/></w:rPr>
                        <w:t>d.</w:t>
                    </w:r>
                </w:p>
            </w:body>
        </w:document>"#;
        let bytes5 = create_mock_docx(xml5);
        let (res5, c5, _) = adapter.process_office(&bytes5, &map, true).unwrap();
        assert_eq!(c5, 1, "Fallo en palabra fragmentada con rsidR y rPr");
        assert!(
            adapter
                .extract_text(&res5)
                .unwrap()
                .contains("El viaje termina en CiudadAnonima.")
        );
    }

    #[test]
    fn test_docx_table_preservation() {
        let adapter = OfficeAdapter::new();
        let mut map = HashMap::new();
        map.insert("servidor".to_string(), "host_anonimo".to_string());
        map.insert("Directorios".to_string(), "Carpetas".to_string());

        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:p>
                    <w:r><w:t>por el servidor.</w:t></w:r>
                </w:p>
                <w:tbl>
                    <w:tblPr><w:tblW w:w="0" w:type="auto"/></w:tblPr>
                    <w:tblGrid><w:gridCol w:w="9000"/></w:tblGrid>
                    <w:tr>
                        <w:tc>
                            <w:tcPr><w:tcW w:w="9000" w:type="dxa"/></w:tcPr>
                            <w:p>
                                <w:r><w:t>Directorios</w:t></w:r>
                            </w:p>
                        </w:tc>
                    </w:tr>
                </w:tbl>
                <w:p><w:r><w:t>Párrafo posterior.</w:t></w:r></w:p>
            </w:body>
        </w:document>"#;
        let bytes = create_mock_docx(xml);
        let (res, c, _) = adapter.process_office(&bytes, &map, false).unwrap();
        assert_eq!(c, 2);

        // Inspeccionar XML resultante
        let cursor = Cursor::new(&res);
        let mut archive = ZipArchive::new(cursor).unwrap();
        let mut file = archive.by_name("word/document.xml").unwrap();
        let mut doc_xml = String::new();
        file.read_to_string(&mut doc_xml).unwrap();
        println!("DOC_XML:\n{doc_xml}");

        assert!(doc_xml.contains("<w:tbl>"));
        assert!(doc_xml.contains("<w:tc>"));
        assert!(doc_xml.contains("host_anonimo"));
        assert!(doc_xml.contains("Carpetas"));
    }

    #[test]
    fn test_docx_toc_bookmark_preservation() {
        let adapter = OfficeAdapter::new();
        let mut map = HashMap::new();
        map.insert("IVH-001".to_string(), "VULN-999".to_string());

        // Simular encabezado con bookmark de tabla de contenidos (_Toc) entre runs
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:p>
                    <w:bookmarkStart w:id="37" w:name="_Toc244678021"/>
                    <w:r><w:t>IVH-0</w:t></w:r>
                    <w:r><w:t>01</w:t></w:r>
                    <w:r><w:t>. Creación no autorizada de usuarios</w:t></w:r>
                    <w:bookmarkEnd w:id="37"/>
                </w:p>
            </w:body>
        </w:document>"#;

        let bytes = create_mock_docx(xml);
        let (res, c, _) = adapter.process_office(&bytes, &map, false).unwrap();
        assert_eq!(c, 1);

        let cursor = Cursor::new(&res);
        let mut archive = ZipArchive::new(cursor).unwrap();
        let mut file = archive.by_name("word/document.xml").unwrap();
        let mut doc_xml = String::new();
        file.read_to_string(&mut doc_xml).unwrap();

        // Los marcadores TOC deben permanecer 100% intactos
        assert!(doc_xml.contains(r#"<w:bookmarkStart w:id="37" w:name="_Toc244678021"/>"#));
        assert!(doc_xml.contains(r#"<w:bookmarkEnd w:id="37"/>"#));
        assert!(doc_xml.contains("VULN-999. Creación no autorizada de usuarios"));
    }

    #[test]
    fn test_docx_style_preservation_different_rpr() {
        let adapter = OfficeAdapter::new();
        let mut map = HashMap::new();
        map.insert("TextoNormal".to_string(), "DatoAnonimo".to_string());

        // Run 1 en negrita y Run 2 en texto normal
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <w:document xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">
            <w:body>
                <w:p>
                    <w:r><w:rPr><w:b/></w:rPr><w:t>Negrita</w:t></w:r>
                    <w:r><w:t>TextoNormal</w:t></w:r>
                </w:p>
            </w:body>
        </w:document>"#;

        let bytes = create_mock_docx(xml);
        let (res, c, _) = adapter.process_office(&bytes, &map, false).unwrap();
        assert_eq!(c, 1);

        let cursor = Cursor::new(&res);
        let mut archive = ZipArchive::new(cursor).unwrap();
        let mut file = archive.by_name("word/document.xml").unwrap();
        let mut doc_xml = String::new();
        file.read_to_string(&mut doc_xml).unwrap();

        // Los runs con diferente rPr no deben fusionarse
        assert!(doc_xml.contains("<w:b/>"));
        assert!(doc_xml.contains("<w:t>Negrita</w:t>"));
        assert!(doc_xml.contains("DatoAnonimo"));
    }
}
