//! Props: diccionario clave→bytes con propiedades del proyecto y los mapas
//! de campos (FieldMap) serializados.
//!
//! Port de `Props.java` + `Props14.java` (MPXJ), la variante MPP14:
//! header de 16 bytes con la cantidad de entradas en el u16 del offset 12;
//! cada entrada es `size:i32, key:i32, ?:i32` + `size` bytes de datos,
//! alineada a 2 bytes.

use std::collections::BTreeMap;

use crate::error::MppError;
use crate::util::{get_f64, get_i32, get_u16, get_unicode_string};

const HEADER_SIZE: usize = 16;

#[derive(Debug)]
pub struct Props {
    map: BTreeMap<i32, Vec<u8>>,
}

impl Props {
    pub fn parse(data: &[u8], context: &str) -> Result<Self, MppError> {
        if data.len() < HEADER_SIZE {
            return Err(MppError::corrupt(context, "Props más corto que el header"));
        }
        let header_count = get_u16(data, 12).unwrap() as usize;

        let mut map = BTreeMap::new();
        let mut pos = HEADER_SIZE;
        let mut found = 0usize;
        while found < header_count {
            // MPXJ: "if we don't have at least 12 bytes left to read, then bail out"
            if data.len().saturating_sub(pos) < 12 {
                break;
            }
            let size = get_i32(data, pos).unwrap();
            let key = get_i32(data, pos + 4).unwrap();
            pos += 12; // size + key + atributo ignorado

            if size < 1 || data.len().saturating_sub(pos) < size as usize {
                break;
            }
            map.insert(key, data[pos..pos + size as usize].to_vec());
            pos += size as usize;
            found += 1;

            // alineación a 2 bytes
            if size % 2 != 0 {
                pos += 1;
            }
        }

        Ok(Props { map })
    }

    pub fn byte_array(&self, key: i32) -> Option<&[u8]> {
        self.map.get(&key).map(Vec::as_slice)
    }

    pub fn get_i32(&self, key: i32) -> Option<i32> {
        get_i32(self.byte_array(key)?, 0)
    }

    pub fn get_u16(&self, key: i32) -> Option<u16> {
        get_u16(self.byte_array(key)?, 0)
    }

    pub fn get_f64(&self, key: i32) -> Option<f64> {
        get_f64(self.byte_array(key)?, 0)
    }

    /// MPXJ getBoolean: short != 0.
    pub fn get_bool(&self, key: i32) -> Option<bool> {
        Some(get_u16(self.byte_array(key)?, 0)? != 0)
    }

    pub fn get_unicode_string(&self, key: i32) -> Option<String> {
        get_unicode_string(self.byte_array(key)?, 0)
    }

    pub fn keys(&self) -> impl Iterator<Item = i32> + '_ {
        self.map.keys().copied()
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn props(entries: &[(i32, &[u8])]) -> Vec<u8> {
        let mut d = vec![0u8; HEADER_SIZE];
        d[12..14].copy_from_slice(&(entries.len() as u16).to_le_bytes());
        for &(key, content) in entries {
            d.extend((content.len() as i32).to_le_bytes());
            d.extend(key.to_le_bytes());
            d.extend(0i32.to_le_bytes());
            d.extend(content);
            if content.len() % 2 != 0 {
                d.push(0); // alineación
            }
        }
        d
    }

    #[test]
    fn parses_typed_entries_with_odd_alignment() {
        let name: Vec<u8> = "Plan".encode_utf16().flat_map(u16::to_le_bytes).collect();
        let d = props(&[
            (10, &[0x07]), // entrada impar → fuerza alineación
            (20, &8.5f64.to_le_bytes()),
            (30, &name),
        ]);
        let p = Props::parse(&d, "test").unwrap();
        assert_eq!(p.len(), 3);
        assert_eq!(p.byte_array(10), Some(&[0x07][..]));
        assert_eq!(p.get_f64(20), Some(8.5));
        assert_eq!(p.get_unicode_string(30).as_deref(), Some("Plan"));
        assert_eq!(p.get_i32(99), None);
    }

    #[test]
    fn stops_at_corrupt_size_without_error() {
        let mut d = props(&[(10, &[0x01, 0x02])]);
        // segunda entrada declarada en el header pero con size absurdo
        d[12..14].copy_from_slice(&2u16.to_le_bytes());
        d.extend((9999i32).to_le_bytes());
        d.extend(20i32.to_le_bytes());
        d.extend(0i32.to_le_bytes());
        let p = Props::parse(&d, "test").unwrap();
        assert_eq!(p.len(), 1);
    }
}
