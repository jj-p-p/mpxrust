//! Lectura de tareas desde `TBkndTask`.
//!
//! Port del camino de tareas de `MPP14Reader.java`: `createTaskMap` (con su
//! manejo de tareas borradas, nulas y duplicadas), lectura de campos vía
//! FieldMap, flags booleanos desde los bytes de FixedMeta (posiciones según
//! la versión de Project que escribió el archivo) y la regla
//! scheduled→actual para fechas/duración de tareas auto-planificadas.

use std::collections::BTreeMap;
use std::io::{Read, Seek};

use crate::blocks::{FixedData, FixedMeta, Var2Data, VarMeta};
use crate::container::MppContainer;
use crate::dec;
use crate::error::MppError;
use crate::field_map::{FieldLocation, FieldMap, task};
use crate::model::{ConstraintType, Task};
use crate::util::{get_i32, get_u16, get_unicode_string};

/// Tamaños de FixedMeta (MPP14Reader): tareas 47; Fixed2Meta varía 92–96.
const TASK_FIXED_META_ITEM_SIZE: usize = 47;
const TASK_FIXED2_META_ITEM_SIZES: &[usize] = &[92, 93, 94, 95, 96];
const NULL_TASK_BLOCK_SIZE: usize = 16;

pub struct TaskBlocks {
    pub var_meta: VarMeta,
    pub var_data: Var2Data,
    pub fixed_meta: FixedMeta,
    pub fixed_data: FixedData,
    pub fixed2_meta: FixedMeta,
    pub fixed2_data: FixedData,
}

pub fn read_blocks<F: Read + Seek>(c: &mut MppContainer<F>) -> Result<TaskBlocks, MppError> {
    let dir = "TBkndTask";
    let var_meta = VarMeta::parse(&c.stream(&format!("{dir}/VarMeta"))?, dir)?;
    let var_data = Var2Data::parse(&var_meta, &c.stream(&format!("{dir}/Var2Data"))?);
    let fixed_meta = FixedMeta::parse(
        &c.stream(&format!("{dir}/FixedMeta"))?,
        TASK_FIXED_META_ITEM_SIZE,
        dir,
    )?;
    // max_expected_size=0: MPXJ acota con maxFixedDataSize del FieldMap, que
    // nosotros subestimamos (solo conocemos el tamaño de los campos portados);
    // sin límite el corte por offsets es idéntico al de MPXJ en archivos sanos.
    let fixed_data = FixedData::parse(&fixed_meta, &c.stream(&format!("{dir}/FixedData"))?, 0);
    let fixed2_meta = FixedMeta::parse_with_candidate_sizes(
        &c.stream(&format!("{dir}/Fixed2Meta"))?,
        fixed_data.item_count(),
        TASK_FIXED2_META_ITEM_SIZES,
        dir,
    )?;
    let fixed2_data = FixedData::parse(&fixed2_meta, &c.stream(&format!("{dir}/Fixed2Data"))?, 0);
    Ok(TaskBlocks {
        var_meta,
        var_data,
        fixed_meta,
        fixed_data,
        fixed2_meta,
        fixed2_data,
    })
}

/// Port de `MPP14Reader.createTaskMap`: uid → índice de bloque (None =
/// tarea borrada, se registra para no resucitarla desde otro bloque).
fn create_task_map(fm: &FieldMap, b: &TaskBlocks) -> BTreeMap<u32, Option<usize>> {
    let mut map: BTreeMap<u32, Option<usize>> = BTreeMap::new();
    let uid_offset = fm
        .fixed_offset(task::UNIQUE_ID)
        .map(|(_, o)| o)
        .unwrap_or(0);
    let max_size = fm.max_fixed_data_size(0);
    let item_count = b.fixed_meta.item_count();

    // MPXJ: los primeros 3 items no son tareas; se recorre hacia atrás porque
    // ante duplicados la versión correcta es la última (mpxj#152).
    for index in (3..item_count).rev() {
        let (Some(data), Some(_data2)) = (b.fixed_data.item(index), b.fixed2_data.item(index))
        else {
            continue;
        };
        let flags = b
            .fixed_meta
            .item(index)
            .and_then(|m| get_i32(m, 0))
            .unwrap_or(0);

        if flags & 0x02 != 0 {
            // tarea borrada: Project guarda solo un short con el uid
            if let Some(uid) = get_u16(data, 0) {
                map.entry(uid as u32).or_insert(None);
            }
            continue;
        }

        if data.len() == NULL_TASK_BLOCK_SIZE {
            if let Some(uid) = get_i32(data, 0) {
                map.entry(uid as u32).or_insert(Some(index));
            }
            continue;
        }

        // heurística MPXJ: con >75% del tamaño esperado, el bloque es válido
        if max_size != 0 && (data.len() * 100) / max_size <= 75 {
            continue;
        }
        let Some(uid) = get_i32(data, uid_offset).map(|v| v as u32) else {
            continue;
        };

        let already = map.contains_key(&uid);
        let has_var_data = b.var_meta.types_for(uid).next().is_some();
        // MPXJ: un uid repetido solo se sobreescribe si tiene var data
        // (no es fantasma) y el flag 0x04 está apagado
        if !already || (has_var_data && flags & 0x04 == 0) {
            map.insert(uid, Some(index));
        }
    }

    map
}

/// Bits booleanos en los bytes de FixedMeta/Fixed2Meta, por versión de la app
/// (tablas `PROJECT20xx_TASK_META_DATA*_BIT_FLAGS` de MPP14Reader).
struct MetaBits {
    milestone: (usize, u8),
    /// En Fixed2Meta ("metaData2"): tarea con planificación manual.
    manual: (usize, u8),
}

fn meta_bits(application_version: u32) -> MetaBits {
    if application_version <= 14 {
        // Project 2010
        MetaBits {
            milestone: (8, 0x20),
            manual: (8, 0x08),
        }
    } else {
        // Project 2013 y 2016+ coinciden en estos dos flags
        MetaBits {
            milestone: (10, 0x02),
            manual: (8, 0x80),
        }
    }
}

fn bit(data: Option<&[u8]>, (byte, mask): (usize, u8)) -> bool {
    data.and_then(|d| d.get(byte))
        .map(|&b| b & mask != 0)
        .unwrap_or(false)
}

/// Lector de un campo concreto resolviendo su ubicación vía FieldMap.
struct FieldReader<'a> {
    fm: &'a FieldMap,
    uid: u32,
    data: &'a [u8],
    data2: Option<&'a [u8]>,
    var_meta: &'a VarMeta,
    var_data: &'a Var2Data,
}

impl FieldReader<'_> {
    /// Bytes del campo: slice del FixedData correspondiente o entrada var.
    fn bytes(&self, field: u16) -> Option<&[u8]> {
        match self.fm.location(field) {
            FieldLocation::FixedData { block, offset } => {
                let d = if block == 0 { self.data } else { self.data2? };
                (offset < d.len()).then(|| &d[offset..])
            }
            FieldLocation::VarData { key } => {
                self.var_data.byte_array(self.var_meta, self.uid, key)
            }
            _ => None,
        }
    }

    fn i32(&self, field: u16) -> Option<i32> {
        get_i32(self.bytes(field)?, 0)
    }

    fn u16(&self, field: u16) -> Option<u16> {
        get_u16(self.bytes(field)?, 0)
    }

    fn string(&self, field: u16) -> Option<String> {
        // trim: MS Project conserva espacios accidentales del usuario que
        // ningún consumidor quiere (summaries de Jira incluidos)
        get_unicode_string(self.bytes(field)?, 0)
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    }

    fn timestamp(&self, field: u16) -> Option<String> {
        dec::get_timestamp(self.bytes(field)?, 0)
    }

    fn work_hours(&self, field: u16) -> Option<f64> {
        dec::work_hours(self.bytes(field)?, 0)
    }

    fn currency(&self, field: u16) -> Option<f64> {
        dec::currency(self.bytes(field)?, 0)
    }
}

pub fn read_tasks(
    fm: &FieldMap,
    blocks: &TaskBlocks,
    application_version: u32,
    minutes_per_day: f64,
) -> Vec<Task> {
    let bits = meta_bits(application_version);
    let task_map = create_task_map(fm, blocks);
    let mut tasks = Vec::new();

    for (&uid, &slot) in &task_map {
        let Some(index) = slot else { continue }; // borrada
        let Some(data) = blocks.fixed_data.item(index) else {
            continue;
        };
        if data.len() == NULL_TASK_BLOCK_SIZE {
            continue; // tarea nula (sin nombre ni datos): irrelevante para el modelo v1
        }
        let meta = blocks.fixed_meta.item(index);
        let meta2 = blocks.fixed2_meta.item(index);

        let r = FieldReader {
            fm,
            uid,
            data,
            data2: blocks.fixed2_data.item(index),
            var_meta: &blocks.var_meta,
            var_data: &blocks.var_data,
        };

        let manual = bit(meta2, bits.manual);

        // MPP14Reader: para tareas auto-planificadas mandan las *scheduled*
        let mut start = r.timestamp(task::START);
        let mut finish = r.timestamp(task::FINISH);
        let scheduled_start = r.timestamp(task::SCHEDULED_START);
        let scheduled_finish = r.timestamp(task::SCHEDULED_FINISH);
        if start.is_none() || (!manual && scheduled_start.is_some()) {
            start = scheduled_start;
        }
        if finish.is_none() || (!manual && scheduled_finish.is_some()) {
            finish = scheduled_finish;
        }

        let duration_days = r.i32(task::SCHEDULED_DURATION).and_then(|tenths| {
            let units = dec::duration_units(r.u16(task::DURATION_UNITS).unwrap_or(7));
            dec::duration_to_days(tenths, units, minutes_per_day)
        });

        let parent_uid = r
            .i32(task::PARENT_TASK_UNIQUE_ID)
            .filter(|&p| p >= 0 && p as u32 != uid)
            .map(|p| p as u32);

        let notes = r.string(task::NOTES).filter(|s| !s.starts_with("{\\rtf"));

        tasks.push(Task {
            uid,
            id: r.i32(task::ID).filter(|&v| v >= 0).map(|v| v as u32),
            name: r.string(task::NAME),
            wbs: r.string(task::WBS),
            outline_level: r
                .u16(task::OUTLINE_LEVEL_ALT)
                .or_else(|| r.u16(task::OUTLINE_LEVEL))
                .map(u32::from),
            parent_uid,
            is_summary: false, // derivado al final (¿alguien me tiene de padre?)
            is_milestone: bit(meta, bits.milestone),
            start_date: start,
            finish_date: finish,
            duration_days,
            work_hours: r.work_hours(task::WORK),
            percent_complete: r
                .bytes(task::PERCENT_COMPLETE)
                .and_then(|b| dec::percentage(b, 0)),
            priority: r.u16(task::PRIORITY).map(u32::from),
            constraint_type: r.u16(task::CONSTRAINT_TYPE).and_then(constraint_type),
            constraint_date: r.timestamp(task::CONSTRAINT_DATE),
            deadline: r.timestamp(task::DEADLINE),
            cost: r.currency(task::COST),
            notes,
            predecessors: Vec::new(), // los completa el reader de relaciones
        });
    }

    // summary = alguna tarea lo tiene como padre (MPXJ: hasChildTasks)
    let parents: std::collections::BTreeSet<u32> =
        tasks.iter().filter_map(|t| t.parent_uid).collect();
    for t in &mut tasks {
        t.is_summary = parents.contains(&t.uid);
    }

    tasks
}

/// Los 8 tipos de constraint, en el orden del formato (ConstraintType.java).
fn constraint_type(raw: u16) -> Option<ConstraintType> {
    Some(match raw {
        0 => ConstraintType::AsSoonAsPossible,
        1 => ConstraintType::AsLateAsPossible,
        2 => ConstraintType::MustStartOn,
        3 => ConstraintType::MustFinishOn,
        4 => ConstraintType::StartNoEarlierThan,
        5 => ConstraintType::StartNoLaterThan,
        6 => ConstraintType::FinishNoEarlierThan,
        7 => ConstraintType::FinishNoLaterThan,
        _ => return None,
    })
}
