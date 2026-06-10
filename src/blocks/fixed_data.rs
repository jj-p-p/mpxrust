//! FixedData: bloques de datos de tamaño acotado, direccionados por FixedMeta.
//!
//! Port de `FixedData.java` (MPXJ). El tamaño de cada item se calcula como la
//! distancia al offset del item siguiente (el último llega hasta el final del
//! stream), con los mismos clamps que MPXJ aplica para tolerar offsets fuera
//! de secuencia, items solapados y tamaños absurdos.

use super::fixed_meta::FixedMeta;

#[derive(Debug)]
pub struct FixedData {
    items: Vec<Option<Vec<u8>>>,
}

impl FixedData {
    /// `max_expected_size = 0` significa "sin límite" (igual que MPXJ).
    pub fn parse(meta: &FixedMeta, data: &[u8], max_expected_size: usize) -> Self {
        let item_count = meta.item_count();
        let mut items: Vec<Option<Vec<u8>>> = vec![None; item_count];

        for (index, slot) in items.iter_mut().enumerate() {
            let Some(item_offset) = meta.data_offset(index) else {
                continue;
            };
            if item_offset < 0 || item_offset as usize > data.len() {
                continue;
            }
            let item_offset = item_offset as usize;

            let mut item_size: i64 = if index + 1 == item_count {
                (data.len() - item_offset) as i64
            } else {
                match meta.data_offset(index + 1) {
                    Some(next) => next as i64 - item_offset as i64,
                    None => continue,
                }
            };

            let available = data.len() - item_offset;
            if item_size < 0 || item_size as usize > available {
                item_size = if max_expected_size == 0 {
                    available as i64
                } else {
                    max_expected_size.min(available) as i64
                };
            }
            if max_expected_size != 0 && item_size as usize > max_expected_size {
                item_size = max_expected_size as i64;
            }

            if item_size > 0 {
                *slot = Some(data[item_offset..item_offset + item_size as usize].to_vec());
            }
        }

        FixedData { items }
    }

    /// Variante con tamaño de item forzado, direccionado por los offsets del
    /// FixedMeta (MPXJ `FixedData(FixedMeta, int itemSize, InputStream)`).
    /// La usa ConstraintFactory (TBkndCons, items de 20 bytes).
    pub fn parse_with_item_size(meta: &FixedMeta, data: &[u8], item_size: usize) -> Self {
        let mut items: Vec<Option<Vec<u8>>> = vec![None; meta.item_count()];
        for (index, slot) in items.iter_mut().enumerate() {
            let Some(offset) = meta.data_offset(index) else {
                continue;
            };
            if offset < 0 || offset as usize > data.len() {
                continue;
            }
            let offset = offset as usize;
            let size = item_size.min(data.len() - offset);
            if size > 0 {
                *slot = Some(data[offset..offset + size].to_vec());
            }
        }
        FixedData { items }
    }

    /// Variante secuencial sin metadatos: items consecutivos de tamaño fijo
    /// (MPXJ `FixedData(int itemSize, InputStream)`). La usan las
    /// asignaciones (TBkndAssn: 110 y 48 bytes).
    pub fn parse_sequential(data: &[u8], item_size: usize) -> Self {
        let items = data
            .chunks_exact(item_size)
            .map(|c| Some(c.to_vec()))
            .collect();
        FixedData { items }
    }

    pub fn item_count(&self) -> usize {
        self.items.len()
    }

    pub fn item(&self, index: usize) -> Option<&[u8]> {
        self.items.get(index)?.as_deref()
    }

    /// Items no nulos, con su índice.
    pub fn iter(&self) -> impl Iterator<Item = (usize, &[u8])> {
        self.items
            .iter()
            .enumerate()
            .filter_map(|(i, d)| Some((i, d.as_deref()?)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blocks::fixed_meta::FixedMeta;
    use crate::blocks::var_meta::BLOCK_MAGIC;

    /// FixedMeta sintético de items de 8 bytes cuyos data_offsets son `offsets`.
    fn meta(offsets: &[i32]) -> FixedMeta {
        let mut d = Vec::new();
        d.extend(BLOCK_MAGIC.to_le_bytes());
        d.extend(0u32.to_le_bytes());
        d.extend((offsets.len() as u32).to_le_bytes());
        d.extend(0u32.to_le_bytes());
        for &o in offsets {
            d.extend(0u32.to_le_bytes());
            d.extend(o.to_le_bytes());
        }
        FixedMeta::parse(&d, 8, "test").unwrap()
    }

    #[test]
    fn item_size_is_distance_to_next_offset() {
        let m = meta(&[0, 4, 10]);
        let data: Vec<u8> = (0..14).collect();
        let fd = FixedData::parse(&m, &data, 0);
        assert_eq!(fd.item(0), Some(&data[0..4]));
        assert_eq!(fd.item(1), Some(&data[4..10]));
        assert_eq!(fd.item(2), Some(&data[10..14])); // último: hasta el final
    }

    #[test]
    fn out_of_range_offset_leaves_none() {
        let m = meta(&[0, 100]);
        let data = [0u8; 10];
        let fd = FixedData::parse(&m, &data, 0);
        assert!(fd.item(0).is_some());
        assert!(fd.item(1).is_none());
        assert_eq!(fd.iter().count(), 1);
    }

    #[test]
    fn max_expected_size_clamps_items() {
        let m = meta(&[0]);
        let data = [7u8; 50];
        let fd = FixedData::parse(&m, &data, 16);
        assert_eq!(fd.item(0).unwrap().len(), 16);
    }

    #[test]
    fn negative_size_from_unsorted_offsets_falls_back() {
        // offsets fuera de secuencia: 10 luego 0 → size negativo → clamp
        let m = meta(&[10, 0]);
        let data: Vec<u8> = (0..20).collect();
        let fd = FixedData::parse(&m, &data, 4);
        assert_eq!(fd.item(0).unwrap(), &data[10..14]); // size negativo → clamp a max_expected_size
        assert_eq!(fd.item(1).unwrap(), &data[0..4]); // último item, también clampeado
    }
}
