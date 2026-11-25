//! Gestor de tabla de mapeos bidireccional con inyectividad estricta y soporte para omisión.

use crate::models::{Mapping, ObfuscationError};
use std::collections::HashMap;

/// Gestor de mapeos bidireccionales de LexiShield.
///
/// Garantiza la propiedad matemática de biyección (inyectividad y sobreyectividad local):
/// Ningún par de valores originales distintos puede compartir el mismo seudónimo,
/// permitiendo una desofuscación 100% determinista y reversible.
#[derive(Debug, Clone, Default)]
pub struct MappingManager {
    forward: HashMap<String, Mapping>,
    reverse: HashMap<String, String>,
}

impl MappingManager {
    pub fn new() -> Self {
        Self {
            forward: HashMap::new(),
            reverse: HashMap::new(),
        }
    }

    /// Añade un mapeo garantizando inyectividad (Mejora 4) y manejando el comodín de omisión ("=").
    pub fn add_mapping(&mut self, mapping: Mapping) -> Result<(), ObfuscationError> {
        // Regla 6: Manejo de comodín de omisión ("=")
        if mapping.original == "=" || mapping.pseudonym == "=" || mapping.omitted {
            log::debug!(
                "Mapeo marcado como omitido ('='), se mantendrá original: {}",
                mapping.original
            );
            let mut m = mapping.clone();
            m.omitted = true;
            m.pseudonym = "=".to_string();
            if let Some(old) = self.forward.get(&m.original)
                && !old.omitted
                && old.pseudonym != "="
            {
                self.reverse.remove(&old.pseudonym);
            }
            self.forward.insert(m.original.clone(), m);
            // No lo añadimos al reverse map para evitar colisiones con múltiples "="
            return Ok(());
        }

        if mapping.original == mapping.pseudonym {
            return Ok(());
        }

        // Si ya existe el original, comprobamos si está omitido o si coincide el seudónimo
        if let Some(existing) = self.forward.get(&mapping.original)
            && (existing.omitted || existing.pseudonym == mapping.pseudonym)
        {
            return Ok(());
        }

        // Verificar inyectividad en el sentido inverso: el seudónimo no debe estar ocupado por otro original
        if let Some(existing_original) = self.reverse.get(&mapping.pseudonym)
            && existing_original != &mapping.original
        {
            return Err(ObfuscationError::CollisionError(format!(
                "Colisión de inyectividad: el seudónimo '{}' ya está asignado a '{}' (se intentó asociar a '{}')",
                mapping.pseudonym, existing_original, mapping.original
            )));
        }

        self.reverse
            .insert(mapping.pseudonym.clone(), mapping.original.clone());
        self.forward.insert(mapping.original.clone(), mapping);
        Ok(())
    }

    /// Carga múltiples mapeos a la tabla.
    pub fn load_mappings(
        &mut self,
        mappings: Vec<Mapping>,
    ) -> Result<Vec<Mapping>, ObfuscationError> {
        let mut newly_added = Vec::new();
        for m in mappings {
            if !self.forward.contains_key(&m.original) {
                self.add_mapping(m.clone())?;
                newly_added.push(m);
            }
        }
        Ok(newly_added)
    }

    /// Obtiene la lista de todos los mapeos activos válidos.
    pub fn get_mappings(&self) -> Vec<Mapping> {
        self.forward.values().cloned().collect()
    }

    /// Obtiene el mapeo completo para un valor original si existe.
    #[must_use]
    pub fn get_mapping(&self, original: &str) -> Option<&Mapping> {
        self.forward.get(original)
    }

    /// Obtiene el seudónimo asignado a un valor original si existe.
    #[must_use]
    pub fn get_pseudonym(&self, original: &str) -> Option<&str> {
        self.forward.get(original).map(|m| m.pseudonym.as_str())
    }

    /// Comprueba si ya existe un valor original registrado en el gestor.
    pub fn contains_original(&self, original: &str) -> bool {
        self.forward.contains_key(original)
    }

    /// Comprueba si ya existe un seudónimo ocupado en el gestor.
    pub fn has_pseudonym(&self, pseudonym: &str) -> bool {
        self.reverse.contains_key(pseudonym)
    }

    /// Obtiene el mapa directo de sustitución (Original -> Seudónimo o Seudónimo -> Original si reverse es True).
    pub fn get_replacement_map(&self, reverse: bool) -> HashMap<String, String> {
        let mut map = HashMap::with_capacity(self.forward.len());
        for m in self.forward.values() {
            if m.omitted || m.original == "=" || m.pseudonym == "=" {
                continue;
            }
            if reverse {
                map.insert(m.pseudonym.clone(), m.original.clone());
                if m.pseudonym.parse::<std::net::Ipv4Addr>().is_ok() {
                    map.entry(m.pseudonym.replace('.', r"\."))
                        .or_insert_with(|| m.original.clone());
                    map.entry(m.pseudonym.replace('.', "[.]"))
                        .or_insert_with(|| m.original.clone());
                }
            } else {
                map.insert(m.original.clone(), m.pseudonym.clone());
            }
        }
        map
    }

    /// Devuelve las claves de mapeos manuales/Custom (con longitud >= 3 caracteres)
    /// que deben sustituirse incluso dentro de otras cadenas sin exigir límites de palabra.
    pub fn get_substring_keys(&self, reverse: bool) -> std::collections::HashSet<String> {
        let mut keys = std::collections::HashSet::new();
        for m in self.forward.values() {
            if m.omitted || m.original == "=" || m.pseudonym == "=" {
                continue;
            }
            if matches!(m.detector_type, crate::models::DetectorType::Custom(_)) {
                let key = if reverse { &m.pseudonym } else { &m.original };
                if key.chars().count() >= 3 {
                    keys.insert(key.clone());
                }
            }
        }
        keys
    }

    pub fn count(&self) -> usize {
        self.forward.len()
    }

    /// Elimina un mapeo por su valor original.
    pub fn remove_mapping(&mut self, original: &str) -> bool {
        if let Some(m) = self.forward.remove(original) {
            if !m.omitted && m.pseudonym != "=" {
                self.reverse.remove(&m.pseudonym);
            }
            true
        } else {
            false
        }
    }

    /// Elimina múltiples mapeos en lote. Devuelve el número de eliminados.
    pub fn batch_remove(&mut self, originals: &[String]) -> usize {
        let mut removed = 0;
        for orig in originals {
            if self.remove_mapping(orig) {
                removed += 1;
            }
        }
        removed
    }

    /// Actualiza o reemplaza un mapeo existente.
    pub fn update_mapping(&mut self, mapping: Mapping) -> Result<(), ObfuscationError> {
        self.remove_mapping(&mapping.original);
        self.add_mapping(mapping)
    }

    pub fn clear(&mut self) {
        self.forward.clear();
        self.reverse.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::replacer::CompiledReplacer;
    use crate::models::{DetectorType, Mapping};

    #[test]
    fn test_reverse_map_includes_regex_ip_variants() {
        let mut manager = MappingManager::new();
        manager
            .add_mapping(Mapping::new(
                "10.0.0.1".to_string(),
                "1.1.1.1".to_string(),
                DetectorType::IPv4,
            ))
            .unwrap();
        manager
            .add_mapping(Mapping::new(
                "10.0.0.2".to_string(),
                "8.8.8.8".to_string(),
                DetectorType::IPv4,
            ))
            .unwrap();
        manager
            .add_mapping(Mapping::new(
                "srv-prod.local".to_string(),
                "srv-01.internal".to_string(),
                DetectorType::Hostname,
            ))
            .unwrap();

        let reverse_map = manager.get_replacement_map(true);
        assert_eq!(
            reverse_map.get("1.1.1.1").map(String::as_str),
            Some("10.0.0.1")
        );
        assert_eq!(
            reverse_map.get(r"1\.1\.1\.1").map(String::as_str),
            Some("10.0.0.1")
        );
        assert_eq!(
            reverse_map.get("1[.]1[.]1[.]1").map(String::as_str),
            Some("10.0.0.1")
        );
        assert!(!reverse_map.contains_key(r"srv-01\.internal"));

        let replacer = CompiledReplacer::new(&reverse_map, true);
        let (restored, count) = replacer.replace(r"^(1\.1\.1\.1|8[.]8[.]8[.]8)$");
        assert_eq!(restored, "^(10.0.0.1|10.0.0.2)$");
        assert_eq!(count, 2);
    }
}
