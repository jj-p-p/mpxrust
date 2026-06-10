//! Lectura de tipos primitivos little-endian sobre slices.
//!
//! Port del subconjunto necesario de `MPPUtility.java` (MPXJ). Todas las
//! funciones devuelven `None` cuando el slice no alcanza, en lugar de hacer
//! panic: los `.mpp` reales traen bloques truncados y MPXJ los tolera.

pub fn get_u16(data: &[u8], offset: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        data.get(offset..offset + 2)?.try_into().ok()?,
    ))
}

pub fn get_i32(data: &[u8], offset: usize) -> Option<i32> {
    Some(i32::from_le_bytes(
        data.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

pub fn get_u32(data: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        data.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

pub fn get_f64(data: &[u8], offset: usize) -> Option<f64> {
    Some(f64::from_le_bytes(
        data.get(offset..offset + 8)?.try_into().ok()?,
    ))
}

/// String UTF-16LE terminada en `0x0000` (o en el fin del slice).
/// Equivale a `MPPUtility.getUnicodeString(data, offset)`.
pub fn get_unicode_string(data: &[u8], offset: usize) -> Option<String> {
    let bytes = data.get(offset..)?;
    let units: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .take_while(|&u| u != 0)
        .collect();
    String::from_utf16(&units).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primitives() {
        let d = [0x01, 0x00, 0xFF, 0xFF, 0xFF, 0xFF];
        assert_eq!(get_u16(&d, 0), Some(1));
        assert_eq!(get_i32(&d, 2), Some(-1));
        assert_eq!(get_i32(&d, 4), None); // fuera de rango: None, no panic
    }

    #[test]
    fn unicode_string_with_terminator_and_accents() {
        // "Año" + terminador + basura
        let mut d: Vec<u8> = "Año".encode_utf16().flat_map(u16::to_le_bytes).collect();
        d.extend([0x00, 0x00, 0xAB, 0xCD]);
        assert_eq!(get_unicode_string(&d, 0).as_deref(), Some("Año"));
    }

    #[test]
    fn unicode_string_without_terminator() {
        let d: Vec<u8> = "abc".encode_utf16().flat_map(u16::to_le_bytes).collect();
        assert_eq!(get_unicode_string(&d, 0).as_deref(), Some("abc"));
    }
}
