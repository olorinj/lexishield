//! Adaptador de formato para documentos Microsoft Office OpenXML (.docx, .xlsx, .pptx).
//!
//! Desempaqueta contenedores ZIP, parsea los flujos XML internos (<w:t>, sharedStrings, <a:t>)
//! mediante streaming (quick-xml) y sustituye las entidades confidenciales preservando al 100%
//! los estilos, tablas, fuentes, márgenes e imágenes sin corromper el formato.

use crate::format_adapters::xml_adapter::XmlAdapter;
use crate::models::ObfuscationError;
use quick_xml::events::Event;
use quick_xml::reader::Reader;
use std::collections::HashMap;
use std::io::{Cursor, Read, Write};
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

impl OfficeAdapter {
    pub fn new() -> Self {
        Self {
            xml_adapter: XmlAdapter::new(),
        }
    }

    /// Detecta el tipo de documento Office analizando la estructura interna del contenedor ZIP.
    pub fn detect_office_type(bytes: &[u8]) -> OfficeType {
        let cursor = Cursor::new(bytes);
        let Ok(mut archive) = ZipArchive::new(cursor) else {
            return OfficeType::Unknown;
        };

        let mut has_word = false;
        let mut has_excel = false;
        let mut has_ppt = false;

        for i in 0..archive.len() {
            if let Ok(file) = archive.by_index(i) {
                let name = file.name();
                if name.starts_with("word/") || name == "word/document.xml" {
                    has_word = true;
                } else if name.starts_with("xl/") || name == "xl/workbook.xml" {
                    has_excel = true;
                } else if name.starts_with("ppt/") || name == "ppt/presentation.xml" {
                    has_ppt = true;
                }
            }
        }

        if has_word {
            OfficeType::Word
        } else if has_excel {
            OfficeType::Excel
        } else if has_ppt {
            OfficeType::PowerPoint
        } else {
            OfficeType::Unknown
        }
    }

    /// Comprueba si el archivo XML interno contiene texto legible por el usuario que deba ser sanitizado.
    fn is_translatable_xml(name: &str, office_type: OfficeType) -> bool {
        let is_xml = std::path::Path::new(name)
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("xml"));
        if !is_xml {
            return false;
        }

        match office_type {
            OfficeType::Word => {
                name == "word/document.xml"
                    || name.starts_with("word/header")
                    || name.starts_with("word/footer")
                    || name == "word/footnotes.xml"
                    || name == "word/endnotes.xml"
                    || name == "word/comments.xml"
            }
            OfficeType::Excel => {
                name == "xl/sharedStrings.xml"
                    || name.starts_with("xl/worksheets/sheet")
                    || name.starts_with("xl/comments")
                    || name.starts_with("xl/drawings/drawing")
            }
            OfficeType::PowerPoint => {
                name.starts_with("ppt/slides/slide")
                    || name.starts_with("ppt/notesSlides/notesSlide")
                    || name.starts_with("ppt/comments/comment")
            }
            OfficeType::Unknown => true,
        }
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
                    let mut reader = Reader::from_str(&xml_content);
                    reader.config_mut().trim_text(false);
                    let mut buf = Vec::new();

                    while let Ok(event) = reader.read_event_into(&mut buf) {
                        match event {
                            Event::Eof => break,
                            Event::Text(t) => {
                                if let Ok(decoded) = std::str::from_utf8(&t)
                                    && let Ok(raw) = quick_xml::escape::unescape(decoded)
                                {
                                    let trimmed = raw.trim();
                                    if !trimmed.is_empty() {
                                        extracted_chunks.push(trimmed.to_string());
                                    }
                                }
                            }
                            _ => {}
                        }
                        buf.clear();
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

                    let (replaced_xml, count) = self.xml_adapter.process_xml(
                        &xml_content,
                        replacement_map,
                        strict_word_boundaries,
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
}
