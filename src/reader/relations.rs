//! Dependencias entre tareas desde `TBkndCons`.
//!
//! Port de `ConstraintFactory.java` (MPXJ): FixedMeta de items de 10 bytes,
//! FixedData con items forzados a 20 bytes en los offsets del meta. El lag
//! cambia de lugar según la versión que escribió el archivo (>2010: lag en
//! el offset 14 y unidades en 18; 2010: lag en 16, unidades en 14).

use std::collections::BTreeMap;
use std::io::{Read, Seek};

use crate::blocks::{FixedData, FixedMeta};
use crate::container::MppContainer;
use crate::dec;
use crate::error::MppError;
use crate::model::{Relation, RelationType, Task};
use crate::util::{get_i32, get_u16};

pub fn attach_relations<F: Read + Seek>(
    c: &mut MppContainer<F>,
    tasks: &mut [Task],
    minutes_per_day: f64,
) -> Result<usize, MppError> {
    let dir = "TBkndCons";
    if !c.has_stream(&format!("{dir}/FixedMeta")) {
        return Ok(0); // proyectos sin dependencias pueden no traer el storage
    }
    let meta = FixedMeta::parse(&c.stream(&format!("{dir}/FixedMeta"))?, 10, dir)?;
    let data = FixedData::parse_with_item_size(
        &meta,
        &c.stream_decrypted(&format!("{dir}/FixedData"))?,
        20,
    );

    let project15 = c.application_version > 14;
    let (duration_offset, units_offset) = if project15 { (14, 18) } else { (16, 14) };

    let mut by_successor: BTreeMap<u32, Vec<Relation>> = BTreeMap::new();
    let mut count = 0usize;

    for index in 0..meta.item_count() {
        // flag de borrado: solo el short inicial (MPXJ, bug SourceForge 2209477)
        if meta.item(index).and_then(|m| get_u16(m, 0)).unwrap_or(1) != 0 {
            continue;
        }
        let Some(item) = data.item(index) else {
            continue;
        };
        if item.len() < 14 {
            continue;
        }

        let pred = get_i32(item, 4).unwrap_or(0);
        let succ = get_i32(item, 8).unwrap_or(0);
        // relaciones con la tarea resumen del proyecto o circulares: inválidas
        if pred <= 0 || succ <= 0 || pred == succ {
            continue;
        }

        let kind = match get_u16(item, 12).unwrap_or(1) {
            0 => RelationType::FinishFinish,
            2 => RelationType::StartFinish,
            3 => RelationType::StartStart,
            _ => RelationType::FinishStart, // 1 y desconocidos (default MPXJ)
        };

        let lag_days = get_i32(item, duration_offset)
            .and_then(|tenths| {
                let units = dec::duration_units(get_u16(item, units_offset).unwrap_or(7));
                dec::duration_to_days(tenths, units, minutes_per_day)
            })
            .unwrap_or(0.0);

        by_successor.entry(succ as u32).or_default().push(Relation {
            predecessor_uid: pred as u32,
            kind,
            lag_days,
        });
        count += 1;
    }

    for task in tasks {
        if let Some(rels) = by_successor.remove(&task.uid) {
            task.predecessors = rels;
        }
    }

    Ok(count)
}
