//! FixedMeta: metadatos de los bloques de datos de tamaño fijo (FixedData).
//!
//! Port de `FixedMeta.java` (MPXJ). Header de 16 bytes (magic, ?, item_count, ?)
//! seguido de items de tamaño fijo conocido (47 bytes para tareas MPP14).
//! MPXJ ignora el item_count del header y lo recalcula del tamaño del stream
//! ("adjusted item count"); replicamos eso. El offset del bloque de datos
//! correspondiente va en los bytes [4..8] de cada item meta.

use crate::error::MppError;
use crate::util::get_u32;

use super::var_meta::BLOCK_MAGIC;

const HEADER_SIZE: usize = 16;

#[derive(Debug)]
pub struct FixedMeta {
    items: Vec<Vec<u8>>,
}

impl FixedMeta {
    /// Constructor con tamaño de item conocido (p. ej. 47 para TBkndTask MPP14).
    pub fn parse(data: &[u8], item_size: usize, context: &str) -> Result<Self, MppError> {
        let magic = get_u32(data, 0)
            .ok_or_else(|| MppError::corrupt(context, "FixedMeta más corto que el header"))?;
        if magic != BLOCK_MAGIC {
            return Err(MppError::corrupt(
                context,
                format!("magic FixedMeta inválido: {magic:#010x}"),
            ));
        }
        if item_size == 0 || data.len() < HEADER_SIZE {
            return Err(MppError::corrupt(context, "FixedMeta vacío o item_size 0"));
        }

        let adjusted = (data.len() - HEADER_SIZE) / item_size;
        let items = (0..adjusted)
            .map(|i| data[HEADER_SIZE + i * item_size..HEADER_SIZE + (i + 1) * item_size].to_vec())
            .collect();
        Ok(FixedMeta { items })
    }

    /// Constructor heurístico: elige el tamaño de item entre varios candidatos
    /// (MPXJ lo usa para Fixed2Meta de tareas: 92, 93, 94, 95 o 96 bytes según
    /// la versión de Project que escribió el archivo). `other_block_count` es
    /// la cantidad de items de un bloque hermano ya parseado, usada como
    /// desempate fuerte.
    pub fn parse_with_candidate_sizes(
        data: &[u8],
        other_block_count: usize,
        candidates: &[usize],
        context: &str,
    ) -> Result<Self, MppError> {
        let item_count = get_u32(data, 8).unwrap_or(0) as usize;
        let available = data.len().saturating_sub(HEADER_SIZE);

        let mut item_size = candidates[0];
        let mut distance = i64::MIN;
        for &test in candidates {
            if test == 0 || !available.is_multiple_of(test) {
                continue;
            }
            // encaja exacto Y coincide con el bloque hermano → es ese
            if available / test == other_block_count {
                item_size = test;
                break;
            }
            // si no, regla de pulgar de MPXJ: el más cercano por debajo
            let test_distance = (item_count * test) as i64 - available as i64;
            if test_distance <= 0 && test_distance > distance {
                item_size = test;
                distance = test_distance;
            }
        }

        Self::parse(data, item_size, context)
    }

    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    pub fn item(&self, index: usize) -> Option<&[u8]> {
        self.items.get(index).map(Vec::as_slice)
    }

    /// Offset (dentro del FixedData) del bloque de datos del item `index`.
    pub fn data_offset(&self, index: usize) -> Option<i32> {
        crate::util::get_i32(self.item(index)?, 4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(item_count: u32, items: &[&[u8]]) -> Vec<u8> {
        let mut d = Vec::new();
        d.extend(BLOCK_MAGIC.to_le_bytes());
        d.extend(0u32.to_le_bytes());
        d.extend(item_count.to_le_bytes());
        d.extend(0u32.to_le_bytes());
        for i in items {
            d.extend_from_slice(i);
        }
        d
    }

    #[test]
    fn fixed_item_size_uses_stream_length_not_header_count() {
        let item = [0u8; 8];
        // header dice 1 item, pero el stream trae 3 → manda el stream (MPXJ)
        let d = block(1, &[&item, &item, &item]);
        let fm = FixedMeta::parse(&d, 8, "test").unwrap();
        assert_eq!(fm.item_count(), 3);
    }

    #[test]
    fn data_offset_reads_bytes_4_to_8() {
        let mut item = [0u8; 8];
        item[4..8].copy_from_slice(&123i32.to_le_bytes());
        let d = block(1, &[&item]);
        let fm = FixedMeta::parse(&d, 8, "test").unwrap();
        assert_eq!(fm.data_offset(0), Some(123));
    }

    #[test]
    fn candidate_sizes_prefers_match_with_sibling_block() {
        // 24 bytes disponibles: candidatos 8 y 12 dividen exacto;
        // el bloque hermano tiene 2 items → gana 12
        let d = block(2, &[&[0u8; 24]]);
        let fm = FixedMeta::parse_with_candidate_sizes(&d, 2, &[8, 12], "test").unwrap();
        assert_eq!(fm.item_count(), 2);
    }

    #[test]
    fn rejects_bad_magic() {
        let mut d = block(0, &[]);
        d[0] = 0x00;
        assert!(FixedMeta::parse(&d, 8, "test").is_err());
    }
}
