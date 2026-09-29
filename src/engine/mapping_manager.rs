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
            log::debug!("Mapeo marcado como omitido ('='), descartado de sustitución: {}", mapping.original);
            return Ok(());
        }

        if mapping.original == mapping.pseudonym {
            return Ok(());
        }

        // Si ya existe el original con el mismo seudónimo, no es error
        if let Some(existing) = self.forward.get(&mapping.original)
            && existing.pseudonym == mapping.pseudonym
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

        self.reverse.insert(mapping.pseudonym.clone(), mapping.original.clone());
        self.forward.insert(mapping.original.clone(), mapping);
        Ok(())
    }

    /// Carga múltiples mapeos a la tabla.
    pub fn load_mappings(&mut self, mappings: Vec<Mapping>) -> Result<(), ObfuscationError> {
        for m in mappings {
            self.add_mapping(m)?;
        }
        Ok(())
    }

    /// Obtiene la lista de todos los mapeos activos válidos.
    pub fn get_mappings(&self) -> Vec<Mapping> {
        self.forward.values().cloned().collect()
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
            } else {
                map.insert(m.original.clone(), m.pseudonym.clone());
            }
        }
        map
    }

    pub fn count(&self) -> usize {
        self.forward.len()
    }

    pub fn clear(&mut self) {
        self.forward.clear();
        self.reverse.clear();
    }
}

