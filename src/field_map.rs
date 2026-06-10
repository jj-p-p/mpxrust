//! L2 — FieldMap: dónde vive cada campo dentro de los bloques de un archivo.
//!
//! Port de `FieldMap.java` + `FieldMap14.java` (MPXJ). El mapa se serializa
//! DENTRO de cada `.mpp` (claves `*_FIELD_MAP` del Props del proyecto) como
//! entradas de 28 bytes; los offsets NO son constantes entre archivos (riesgo
//! R2 de docs/02). En MPP14 la var data key es el índice del campo
//! (`typeValue & 0xFFFF`); las únicas sustituciones de `VAR_DATA_MAP` afectan
//! a custom fields de asignaciones, fuera del alcance v1.

use std::collections::HashMap;

use crate::blocks::Props;
use crate::error::MppError;
use crate::util::{get_i32, get_u16};

/// Claves Props de los mapas serializados (PropsKey.java).
const TASK_KEYS: [i32; 2] = [131092, 50331668];
const RESOURCE_KEYS: [i32; 2] = [131093, 50331669];
const ASSIGNMENT_KEYS: [i32; 2] = [131095, 50331671];

/// Bases de fieldID por entidad (MPPTaskField/MPPResourceField/MPPAssignmentField).
pub const TASK_FIELD_BASE: i32 = 0x0B40_0000;
pub const RESOURCE_FIELD_BASE: i32 = 0x0C40_0000;
pub const ASSIGNMENT_FIELD_BASE: i32 = 0x0F40_0000;

/// Índices de campos de tarea (FIELD_ARRAY de MPPTaskField.java).
/// En MPP14, 29/35/36 son las variantes *scheduled* (mapMpp14).
pub mod task {
    pub const WORK: u16 = 0;
    pub const COST: u16 = 5;
    pub const NAME: u16 = 14;
    pub const NOTES: u16 = 15;
    pub const WBS: u16 = 16;
    pub const CONSTRAINT_TYPE: u16 = 17;
    pub const CONSTRAINT_DATE: u16 = 18;
    pub const ID: u16 = 23;
    pub const PRIORITY: u16 = 25;
    pub const SCHEDULED_DURATION: u16 = 29;
    pub const DURATION_UNITS: u16 = 30;
    pub const PERCENT_COMPLETE: u16 = 32;
    pub const SCHEDULED_START: u16 = 35;
    pub const SCHEDULED_FINISH: u16 = 36;
    pub const OUTLINE_LEVEL: u16 = 85;
    pub const UNIQUE_ID: u16 = 86;
    pub const PARENT_TASK_UNIQUE_ID: u16 = 160;
    pub const OUTLINE_LEVEL_ALT: u16 = 249;
    pub const DEADLINE: u16 = 437;
    /// "Task Start"/"Task Finish" en MPP14: fechas manuales.
    pub const START: u16 = 1283;
    pub const FINISH: u16 = 1284;
}

/// Índices de campos de recurso (MPPResourceField.java).
pub mod resource {
    pub const NAME: u16 = 1;
    #[allow(dead_code)] // lo usará el createResourceMap completo (post-v1)
    pub const UNIQUE_ID: u16 = 27;
    pub const EMAIL_ADDRESS: u16 = 35;
}

/// Índices de campos de asignación (MPPAssignmentField.java).
pub mod assignment {
    pub const UNIQUE_ID: u16 = 0;
    pub const TASK_UNIQUE_ID: u16 = 1;
    pub const RESOURCE_UNIQUE_ID: u16 = 2;
    pub const UNITS: u16 = 7;
    pub const WORK: u16 = 8;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldLocation {
    FixedData {
        block: usize,
        offset: usize,
    },
    VarData {
        key: u16,
    },
    /// Flags booleanos en FixedMeta — MPXJ los lee por posiciones fijas
    /// conocidas, no desde el field map (ver MPP14Reader bit flags).
    MetaData,
    Unknown,
}

#[derive(Debug)]
pub struct FieldMap {
    /// índice de campo (low word del typeValue) → ubicación.
    items: HashMap<u16, FieldLocation>,
    /// Cota inferior del tamaño de los bloques FixedData [bloque 0, bloque 1].
    /// MPXJ conoce el tamaño de TODOS los tipos de campo; nosotros solo el de
    /// los portados, así que esto subestima — se usa solo para la heurística
    /// del 75% en el task map, donde alcanza.
    max_fixed_size: [usize; 2],
}

impl FieldMap {
    pub fn for_tasks(props: &Props) -> Result<Self, MppError> {
        Self::create(props, &TASK_KEYS, TASK_FIELD_BASE, "TASK_FIELD_MAP")
    }

    pub fn for_resources(props: &Props) -> Result<Self, MppError> {
        Self::create(
            props,
            &RESOURCE_KEYS,
            RESOURCE_FIELD_BASE,
            "RESOURCE_FIELD_MAP",
        )
    }

    pub fn for_assignments(props: &Props) -> Result<Self, MppError> {
        Self::create(
            props,
            &ASSIGNMENT_KEYS,
            ASSIGNMENT_FIELD_BASE,
            "ASSIGNMENT_FIELD_MAP",
        )
    }

    fn create(props: &Props, keys: &[i32], base: i32, context: &str) -> Result<Self, MppError> {
        let data = keys
            .iter()
            .find_map(|&k| props.byte_array(k))
            .ok_or_else(|| {
                // MPXJ cae a tablas default; no las hemos necesitado en archivos
                // reales (Project siempre serializa el mapa) — pendiente H6.
                MppError::corrupt(
                    context,
                    "el archivo no trae field map serializado (defaults no implementados)",
                )
            })?;
        Ok(Self::parse(data, base))
    }

    /// Port de `FieldMap.createFieldMap`: entradas de 28 bytes.
    fn parse(data: &[u8], base: i32) -> Self {
        let mut items = HashMap::new();
        let mut max_fixed_size = [0usize; 2];
        let mut last_offset = 0i32;
        let mut block = 0usize;

        let mut index = 0;
        while index + 28 <= data.len() {
            let data_block_offset = get_u16(data, index + 4).unwrap() as i32;
            let type_value = get_i32(data, index + 12).unwrap();
            let category = get_u16(data, index + 20).unwrap();

            // solo campos de la entidad de este mapa
            if (type_value & 0x7FFF_0000) != (base & 0x7FFF_0000) {
                index += 28;
                continue;
            }
            let field_index = (type_value & 0xFFFF) as u16;

            let location = match category {
                0x0B | 0x64 => FieldLocation::MetaData,
                _ => {
                    if data_block_offset != 65535 {
                        if data_block_offset < last_offset {
                            block += 1; // MPXJ: offset retrocede => bloque siguiente (Fixed2Data)
                        }
                        last_offset = data_block_offset;
                        if block < 2 {
                            let end = data_block_offset as usize + fixed_field_size(field_index);
                            max_fixed_size[block] = max_fixed_size[block].max(end);
                        }
                        FieldLocation::FixedData {
                            block: block.min(1),
                            offset: data_block_offset as usize,
                        }
                    } else if field_index != 0 {
                        FieldLocation::VarData { key: field_index }
                    } else {
                        FieldLocation::Unknown
                    }
                }
            };

            items.insert(field_index, location);
            index += 28;
        }

        FieldMap {
            items,
            max_fixed_size,
        }
    }

    pub fn location(&self, field_index: u16) -> FieldLocation {
        self.items
            .get(&field_index)
            .copied()
            .unwrap_or(FieldLocation::Unknown)
    }

    /// Offset en FixedData si el campo vive ahí.
    pub fn fixed_offset(&self, field_index: u16) -> Option<(usize, usize)> {
        match self.location(field_index) {
            FieldLocation::FixedData { block, offset } => Some((block, offset)),
            _ => None,
        }
    }

    /// Var data key si el campo vive en Var2Data.
    pub fn var_key(&self, field_index: u16) -> Option<u16> {
        match self.location(field_index) {
            FieldLocation::VarData { key } => Some(key),
            _ => None,
        }
    }

    pub fn max_fixed_data_size(&self, block: usize) -> usize {
        self.max_fixed_size.get(block).copied().unwrap_or(0)
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

/// Tamaño en bytes de los campos fixed que conocemos (subset de
/// `FieldMap.getFixedDataFieldSize`); desconocidos = 2 (cota inferior).
fn fixed_field_size(field_index: u16) -> usize {
    use task::*;
    match field_index {
        SCHEDULED_START | SCHEDULED_FINISH | START | FINISH | CONSTRAINT_DATE | DEADLINE => 4,
        ID | UNIQUE_ID | PARENT_TASK_UNIQUE_ID | SCHEDULED_DURATION => 4,
        WORK | COST => 8,
        _ => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Entrada sintética de 28 bytes del field map.
    fn entry(fixed_offset: u16, type_value: i32, category: u16) -> Vec<u8> {
        let mut e = vec![0u8; 28];
        e[4..6].copy_from_slice(&fixed_offset.to_le_bytes());
        e[12..16].copy_from_slice(&type_value.to_le_bytes());
        e[20..22].copy_from_slice(&category.to_le_bytes());
        e
    }

    #[test]
    fn parses_fixed_var_and_meta_locations() {
        let mut d = Vec::new();
        d.extend(entry(0, TASK_FIELD_BASE | task::UNIQUE_ID as i32, 0x03));
        d.extend(entry(65535, TASK_FIELD_BASE | task::NAME as i32, 0x08));
        d.extend(entry(0, TASK_FIELD_BASE | 24, 0x0B)); // MILESTONE en metadata
        let fm = FieldMap::parse(&d, TASK_FIELD_BASE);
        assert_eq!(fm.fixed_offset(task::UNIQUE_ID), Some((0, 0)));
        assert_eq!(fm.var_key(task::NAME), Some(task::NAME));
        assert_eq!(fm.location(24), FieldLocation::MetaData);
        assert_eq!(fm.location(9999), FieldLocation::Unknown);
    }

    #[test]
    fn block_index_advances_when_offsets_rewind() {
        let mut d = Vec::new();
        d.extend(entry(0, TASK_FIELD_BASE | task::UNIQUE_ID as i32, 0x03));
        d.extend(entry(
            120,
            TASK_FIELD_BASE | task::SCHEDULED_START as i32,
            0x13,
        ));
        d.extend(entry(8, TASK_FIELD_BASE | task::ID as i32, 0x03)); // retrocede → bloque 1
        let fm = FieldMap::parse(&d, TASK_FIELD_BASE);
        assert_eq!(fm.fixed_offset(task::UNIQUE_ID), Some((0, 0)));
        assert_eq!(fm.fixed_offset(task::SCHEDULED_START), Some((0, 120)));
        assert_eq!(fm.fixed_offset(task::ID), Some((1, 8)));
        assert_eq!(fm.max_fixed_data_size(0), 124);
    }

    #[test]
    fn ignores_entries_from_other_entities() {
        let d = entry(0, RESOURCE_FIELD_BASE | 1, 0x08);
        let fm = FieldMap::parse(&d, TASK_FIELD_BASE);
        assert!(fm.is_empty());
    }
}
