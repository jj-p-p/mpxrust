//! VarMeta: metadatos de los datos de largo variable de un Var2Data.
//!
//! Port de `VarMeta12.java` (MPXJ) — la variante que usan MPP12 y MPP14.
//! Layout: header de 24 bytes (magic, ?, item_count, ?, ?, data_size) seguido
//! de entradas de 12 bytes: `uid:i32, offset:i32, tipo:u16, ?:u16`.

use std::collections::BTreeMap;

use crate::error::MppError;
use crate::util::{get_u16, get_u32};

pub(crate) const BLOCK_MAGIC: u32 = 0xFADF_ADBA;

const HEADER_SIZE: usize = 24;
const ENTRY_SIZE: usize = 12;

/// Mapa `uid → { tipo de campo → offset en Var2Data }` más la lista de
/// offsets ordenada que Var2Data necesita para recorrer su stream.
#[derive(Debug)]
pub struct VarMeta {
    table: BTreeMap<u32, BTreeMap<u16, u32>>,
    offsets: Vec<u32>,
}

impl VarMeta {
    pub fn parse(data: &[u8], context: &str) -> Result<Self, MppError> {
        let magic = get_u32(data, 0)
            .ok_or_else(|| MppError::corrupt(context, "VarMeta más corto que el header"))?;
        // MPXJ: "I have one example where an otherwise valid VarMeta block has
        // zero for a magic number. MS Project reads the file OK, so we'll
        // treat zero as a valid value."
        if magic != BLOCK_MAGIC && magic != 0 {
            return Err(MppError::corrupt(
                context,
                format!("magic VarMeta inválido: {magic:#010x}"),
            ));
        }

        let item_count = get_u32(data, 8).unwrap_or(0) as usize;
        let mut table: BTreeMap<u32, BTreeMap<u16, u32>> = BTreeMap::new();
        let mut offsets = Vec::with_capacity(item_count);

        let mut pos = HEADER_SIZE;
        for _ in 0..item_count {
            // MPXJ corta sin error si el stream se queda corto (archivos reales truncos)
            if pos + ENTRY_SIZE > data.len() {
                break;
            }
            let uid = get_u32(data, pos).unwrap();
            let offset = get_u32(data, pos + 4).unwrap();
            let typ = get_u16(data, pos + 8).unwrap();
            table.entry(uid).or_default().insert(typ, offset);
            offsets.push(offset);
            pos += ENTRY_SIZE;
        }

        offsets.sort_unstable();
        Ok(VarMeta { table, offsets })
    }

    /// UIDs presentes, en orden ascendente.
    pub fn unique_ids(&self) -> impl Iterator<Item = u32> + '_ {
        self.table.keys().copied()
    }

    pub fn uid_count(&self) -> usize {
        self.table.len()
    }

    pub fn entry_count(&self) -> usize {
        self.table.values().map(BTreeMap::len).sum()
    }

    /// Offset en Var2Data del campo `typ` del item `uid`.
    pub fn offset(&self, uid: u32, typ: u16) -> Option<u32> {
        self.table.get(&uid)?.get(&typ).copied()
    }

    /// Tipos de campo presentes para un uid.
    pub fn types_for(&self, uid: u32) -> impl Iterator<Item = u16> + '_ {
        self.table
            .get(&uid)
            .into_iter()
            .flat_map(|m| m.keys().copied())
    }

    /// Offsets ordenados ascendente (los usa Var2Data para leer su stream).
    pub(crate) fn sorted_offsets(&self) -> &[u32] {
        &self.offsets
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn header(item_count: u32, magic: u32) -> Vec<u8> {
        let mut h = Vec::new();
        h.extend(magic.to_le_bytes());
        h.extend(0u32.to_le_bytes());
        h.extend(item_count.to_le_bytes());
        h.extend([0u8; 12]); // unknown2, unknown3, data_size (irrelevante para el parse)
        h
    }

    fn entry(uid: u32, offset: u32, typ: u16) -> Vec<u8> {
        let mut e = Vec::new();
        e.extend(uid.to_le_bytes());
        e.extend(offset.to_le_bytes());
        e.extend(typ.to_le_bytes());
        e.extend(0u16.to_le_bytes());
        e
    }

    #[test]
    fn parses_entries_grouped_by_uid() {
        let mut d = header(3, BLOCK_MAGIC);
        d.extend(entry(7, 100, 14));
        d.extend(entry(7, 40, 6));
        d.extend(entry(9, 0, 14));
        let vm = VarMeta::parse(&d, "test").unwrap();
        assert_eq!(vm.uid_count(), 2);
        assert_eq!(vm.entry_count(), 3);
        assert_eq!(vm.offset(7, 14), Some(100));
        assert_eq!(vm.offset(7, 6), Some(40));
        assert_eq!(vm.offset(9, 14), Some(0));
        assert_eq!(vm.sorted_offsets(), &[0, 40, 100]);
    }

    #[test]
    fn accepts_zero_magic_like_mpxj() {
        let d = header(0, 0);
        assert!(VarMeta::parse(&d, "test").is_ok());
    }

    #[test]
    fn rejects_bad_magic() {
        let d = header(0, 0xDEAD_BEEF);
        assert!(matches!(
            VarMeta::parse(&d, "test"),
            Err(MppError::Corrupt { .. })
        ));
    }

    #[test]
    fn truncated_entries_stop_silently() {
        let mut d = header(5, BLOCK_MAGIC);
        d.extend(entry(1, 0, 14));
        // dice 5 items pero solo hay 1 completo + bytes sueltos
        d.extend([0xAA, 0xBB]);
        let vm = VarMeta::parse(&d, "test").unwrap();
        assert_eq!(vm.entry_count(), 1);
    }
}
