//! Var2Data: datos de largo variable, direccionados por el VarMeta asociado.
//!
//! Port de `Var2Data.java` (MPXJ). En cada offset que el VarMeta declara,
//! el bloque trae `size:i32` seguido de `size` bytes. MPXJ recorre los
//! offsets ordenados y tolera offsets fuera de rango o sizes corruptos
//! saltándolos; replicamos eso.

use std::collections::BTreeMap;

use super::var_meta::VarMeta;
use crate::util::{get_i32, get_unicode_string};

#[derive(Debug)]
pub struct Var2Data {
    map: BTreeMap<u32, Vec<u8>>,
}

impl Var2Data {
    pub fn parse(meta: &VarMeta, data: &[u8]) -> Self {
        let mut map = BTreeMap::new();
        for &offset in meta.sorted_offsets() {
            let o = offset as usize;
            if o >= data.len() {
                continue;
            }
            let Some(size) = get_i32(data, o) else {
                continue;
            };
            // MPXJ: "Try our best to handle corrupt files gracefully"
            if size < 0 || o + 4 + size as usize > data.len() {
                continue;
            }
            map.insert(offset, data[o + 4..o + 4 + size as usize].to_vec());
        }
        Var2Data { map }
    }

    pub fn byte_array(&self, meta: &VarMeta, uid: u32, typ: u16) -> Option<&[u8]> {
        self.map.get(&meta.offset(uid, typ)?).map(Vec::as_slice)
    }

    /// String UTF-16LE del campo `typ` del item `uid`.
    pub fn unicode_string(&self, meta: &VarMeta, uid: u32, typ: u16) -> Option<String> {
        get_unicode_string(self.byte_array(meta, uid, typ)?, 0)
    }

    pub fn item_count(&self) -> usize {
        self.map.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blocks::var_meta::{BLOCK_MAGIC, VarMeta};

    /// VarMeta sintético con entradas (uid, offset, tipo).
    fn meta(entries: &[(u32, u32, u16)]) -> VarMeta {
        let mut d = Vec::new();
        d.extend(BLOCK_MAGIC.to_le_bytes());
        d.extend(0u32.to_le_bytes());
        d.extend((entries.len() as u32).to_le_bytes());
        d.extend([0u8; 12]);
        for &(uid, off, typ) in entries {
            d.extend(uid.to_le_bytes());
            d.extend(off.to_le_bytes());
            d.extend(typ.to_le_bytes());
            d.extend(0u16.to_le_bytes());
        }
        VarMeta::parse(&d, "test").unwrap()
    }

    fn sized(content: &[u8]) -> Vec<u8> {
        let mut v = (content.len() as i32).to_le_bytes().to_vec();
        v.extend(content);
        v
    }

    #[test]
    fn reads_items_at_offsets() {
        let m = meta(&[(1, 0, 14), (2, 10, 14)]);
        let name: Vec<u8> = "Hola".encode_utf16().flat_map(u16::to_le_bytes).collect();
        let mut data = sized(&name); // item en offset 0 (4 + 8 = 12 bytes)...
        data.truncate(10); // fuerza padding hasta el offset 10 del segundo item
        data.resize(10, 0);
        data.extend(sized(&name));
        let v = Var2Data::parse(&m, &data);
        assert_eq!(v.unicode_string(&m, 2, 14).as_deref(), Some("Hola"));
    }

    #[test]
    fn skips_corrupt_sizes_and_out_of_range_offsets() {
        let m = meta(&[(1, 0, 14), (2, 500, 14)]);
        let data = (-5i32).to_le_bytes().to_vec(); // size negativo en offset 0
        let v = Var2Data::parse(&m, &data);
        assert_eq!(v.item_count(), 0);
        assert!(v.byte_array(&m, 1, 14).is_none());
        assert!(v.byte_array(&m, 2, 14).is_none());
    }
}
