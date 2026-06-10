//! Propiedades del proyecto desde `   114/Props`.
//!
//! Claves de `PropsKey.java`; lecturas según `ProjectPropertiesReader`/
//! `MPP14Reader` de MPXJ. `minutes_per_day` es crítico: convierte las
//! duraciones (almacenadas en décimas de minuto) a días.

use crate::blocks::Props;
use crate::dec;
use crate::model::ProjectProperties;

const PROJECT_START_DATE: i32 = 37748738;
const PROJECT_FINISH_DATE: i32 = 37748739;
const TITLE: i32 = 37748744;
const MINUTES_PER_DAY: i32 = 37748765;
const MINUTES_PER_WEEK: i32 = 37748766;
const DAYS_PER_MONTH: i32 = 37753743;

pub fn read(props: &Props) -> ProjectProperties {
    ProjectProperties {
        title: props.get_unicode_string(TITLE).filter(|s| !s.is_empty()),
        start_date: props
            .byte_array(PROJECT_START_DATE)
            .and_then(|b| dec::get_timestamp(b, 0)),
        finish_date: props
            .byte_array(PROJECT_FINISH_DATE)
            .and_then(|b| dec::get_timestamp(b, 0)),
        minutes_per_day: props
            .get_i32(MINUTES_PER_DAY)
            .filter(|&v| v > 0)
            .map(|v| v as u32),
        minutes_per_week: props
            .get_i32(MINUTES_PER_WEEK)
            .filter(|&v| v > 0)
            .map(|v| v as u32),
        days_per_month: props
            .get_u16(DAYS_PER_MONTH)
            .filter(|&v| v > 0)
            .map(u32::from),
    }
}

/// Minutos por día efectivos (default de MS Project: 480 = 8 horas).
pub fn minutes_per_day(p: &ProjectProperties) -> f64 {
    p.minutes_per_day.unwrap_or(480) as f64
}
