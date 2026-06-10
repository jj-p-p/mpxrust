//! Decodificación de los tipos de dato propios de MS Project.
//!
//! Port del subconjunto necesario de `MPPUtility.java` (MPXJ):
//! - fechas: días desde el epoch 1983-12-31
//! - horas: décimas de minuto desde medianoche
//! - duraciones: décimas de minuto, con unidad aparte
//! - trabajo: milésimas de minuto (double / 60000 = horas)
//! - moneda y porcentaje

use crate::util::{get_f64, get_u16};

/// Días entre 0000-03-01 y el epoch de MS Project (1983-12-31), según el
/// algoritmo civil de Howard Hinnant que usamos abajo.
const MS_EPOCH_DAYS: i64 = days_from_civil(1983, 12, 31);

/// <https://howardhinnant.github.io/date_algorithms.html#days_from_civil>
const fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// <https://howardhinnant.github.io/date_algorithms.html#civil_from_days>
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn iso_date(days_since_ms_epoch: i64) -> String {
    let (y, m, d) = civil_from_days(MS_EPOCH_DAYS + days_since_ms_epoch);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Timestamp MPP: `time:u16` (unidades de 6 segundos) + `date:u16` (días desde
/// 1983-12-31). Port de `MPPUtility.getTimestamp`, incluidas las heurísticas
/// de NA. Devuelve ISO-8601 `YYYY-MM-DDTHH:MM:SS`.
pub fn get_timestamp(data: &[u8], offset: usize) -> Option<String> {
    let days = get_u16(data, offset + 2)? as i64;
    if days <= 1 || days == 65535 {
        return None;
    }
    let mut time = get_u16(data, offset)? as i64;
    if time == 65535 {
        time = 0;
    }
    let seconds = time * 6;
    // MPXJ: días muy chicos con segundos sueltos == NA en MS Project
    if days < 100 && seconds % 60 != 0 {
        return None;
    }
    let (h, min, s) = (seconds / 3600, (seconds % 3600) / 60, seconds % 60);
    Some(format!("{}T{h:02}:{min:02}:{s:02}", iso_date(days)))
}

/// Fecha MPP de 2 bytes: días desde 1983-12-31 (`MPPUtility.getDate`).
#[allow(dead_code)]
pub fn get_date(data: &[u8], offset: usize) -> Option<String> {
    let days = get_u16(data, offset)? as i64;
    if days == 65535 {
        return None;
    }
    Some(iso_date(days))
}

/// Unidades de duración MPP (`MPPUtility.getDurationTimeUnits`, máscara 0x1F).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeUnit {
    Minutes,
    Hours,
    Days,
    ElapsedDays,
    Weeks,
    Months,
}

pub fn duration_units(raw: u16) -> TimeUnit {
    match raw & 0x1F {
        3 | 4 => TimeUnit::Minutes,
        5 | 6 => TimeUnit::Hours,
        8 => TimeUnit::ElapsedDays,
        9 | 10 => TimeUnit::Weeks,
        11 | 12 => TimeUnit::Months,
        _ => TimeUnit::Days, // 7 y desconocidas: MPXJ defaultea a días
    }
}

/// Duración almacenada en décimas de minuto → días laborales del proyecto.
/// Port de `MPPUtility.getAdjustedDuration` normalizando SIEMPRE a días
/// (el modelo v1 expone `duration_days`). Todas las unidades laborales se
/// guardan en décimas de minuto, así que solo distinguimos laboral (días de
/// `minutes_per_day`) de transcurrido (días de 24h).
pub fn duration_to_days(
    tenths_of_minute: i32,
    units: TimeUnit,
    minutes_per_day: f64,
) -> Option<f64> {
    if tenths_of_minute == -1 || minutes_per_day == 0.0 {
        return None;
    }
    let minutes = tenths_of_minute as f64 / 10.0;
    Some(match units {
        TimeUnit::ElapsedDays => minutes / (24.0 * 60.0),
        _ => minutes / minutes_per_day,
    })
}

/// Trabajo: double en milésimas de minuto → horas. MPXJ ignora < 1 minuto.
pub fn work_hours(data: &[u8], offset: usize) -> Option<f64> {
    let raw = get_f64(data, offset)?;
    Some(if raw.abs() < 1000.0 {
        0.0
    } else {
        raw / 60000.0
    })
}

/// Moneda: double en centésimas. MPXJ ignora < 0.1 centavo.
pub fn currency(data: &[u8], offset: usize) -> Option<f64> {
    let raw = get_f64(data, offset)?;
    Some(if raw.abs() < 0.1 { 0.0 } else { raw / 100.0 })
}

/// Porcentaje: short 0..=100; fuera de rango = None (`MPPUtility.getPercentage`).
pub fn percentage(data: &[u8], offset: usize) -> Option<u32> {
    let v = get_u16(data, offset)?;
    (v <= 100).then_some(v as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts(time: u16, days: u16) -> Vec<u8> {
        let mut v = time.to_le_bytes().to_vec();
        v.extend(days.to_le_bytes());
        v
    }

    #[test]
    fn epoch_math_matches_known_dates() {
        // 1983-12-31 + 1 día = 1984-01-01
        assert_eq!(iso_date(1), "1984-01-01");
        // verificado contra MPXJ: el JSON del proyecto ANCO usa 2026
        assert_eq!(iso_date(15486), "2026-05-25");
    }

    #[test]
    fn timestamp_decodes_time_component() {
        // 8:00 AM = 480 min = 4800 unidades de 6s... no: 8h*3600s/6s = 4800
        let d = ts(4800, 15486);
        assert_eq!(get_timestamp(&d, 0).as_deref(), Some("2026-05-25T08:00:00"));
    }

    #[test]
    fn timestamp_na_values() {
        assert_eq!(get_timestamp(&ts(0, 0), 0), None);
        assert_eq!(get_timestamp(&ts(0, 65535), 0), None);
        // time 65535 => 00:00
        assert_eq!(
            get_timestamp(&ts(65535, 200), 0).as_deref(),
            Some("1984-07-18T00:00:00")
        );
    }

    #[test]
    fn duration_units_decode() {
        assert_eq!(duration_units(7), TimeUnit::Days);
        assert_eq!(duration_units(5), TimeUnit::Hours);
        assert_eq!(duration_units(0x27), TimeUnit::Days); // estimado: bit alto + 7
    }

    #[test]
    fn duration_conversion_to_days() {
        // 5 días * 480 min/día * 10 décimas = 24000
        assert_eq!(duration_to_days(24000, TimeUnit::Days, 480.0), Some(5.0));
        // 16 horas = 9600 décimas; a días de 8h = 2
        assert_eq!(duration_to_days(9600, TimeUnit::Hours, 480.0), Some(2.0));
        assert_eq!(duration_to_days(-1, TimeUnit::Days, 480.0), None);
    }

    #[test]
    fn work_and_currency_thresholds() {
        let w = 64.0 * 60000.0_f64; // 64 horas
        assert_eq!(work_hours(&w.to_le_bytes(), 0), Some(64.0));
        assert_eq!(work_hours(&500.0_f64.to_le_bytes(), 0), Some(0.0));
        assert_eq!(currency(&12345.0_f64.to_le_bytes(), 0), Some(123.45));
    }
}
