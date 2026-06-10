//! Recursos (`TBkndRsc`) y asignaciones (`TBkndAssn`).
//!
//! Port simplificado de `MPP14Reader.processResourceData` y
//! `ResourceAssignmentFactory` (MPXJ), limitado al alcance v1: identidad del
//! recurso (uid, nombre, email) y asignaciones tarea↔recurso con unidades y
//! trabajo. Los FixedData de ambos directorios pasan por la ofuscación XOR.

use std::collections::BTreeSet;
use std::io::{Read, Seek};

use crate::blocks::{FixedData, FixedMeta, Var2Data, VarMeta};
use crate::container::MppContainer;
use crate::error::MppError;
use crate::field_map::{FieldMap, assignment, resource};
use crate::model::{Assignment, Resource};
use crate::util::{get_f64, get_i32, get_unicode_string};

/// MPXJ `MicrosoftProjectConstants.ASSIGNMENT_NULL_RESOURCE_ID`.
const NULL_RESOURCE_ID: i32 = -65535;

pub fn read_resources<F: Read + Seek>(
    c: &mut MppContainer<F>,
    fm: &FieldMap,
) -> Result<Vec<Resource>, MppError> {
    let dir = "TBkndRsc";
    let var_meta = VarMeta::parse(&c.stream(&format!("{dir}/VarMeta"))?, dir)?;
    let var_data = Var2Data::parse(&var_meta, &c.stream(&format!("{dir}/Var2Data"))?);

    let name_key = fm.var_key(resource::NAME).unwrap_or(resource::NAME);
    let email_key = fm
        .var_key(resource::EMAIL_ADDRESS)
        .unwrap_or(resource::EMAIL_ADDRESS);

    let mut resources = Vec::new();
    for uid in var_meta.unique_ids() {
        let name = var_data
            .byte_array(&var_meta, uid, name_key)
            .and_then(|b| get_unicode_string(b, 0))
            .filter(|s| !s.is_empty());
        let email = var_data
            .byte_array(&var_meta, uid, email_key)
            .and_then(|b| get_unicode_string(b, 0))
            .filter(|s| !s.is_empty());
        if name.is_some() || email.is_some() {
            resources.push(Resource { uid, name, email });
        }
    }
    Ok(resources)
}

pub fn read_assignments<F: Read + Seek>(
    c: &mut MppContainer<F>,
    fm: &FieldMap,
    valid_task_uids: &BTreeSet<u32>,
) -> Result<Vec<Assignment>, MppError> {
    let dir = "TBkndAssn";
    let var_meta = VarMeta::parse(&c.stream(&format!("{dir}/VarMeta"))?, dir)?;
    let var_data = Var2Data::parse(&var_meta, &c.stream(&format!("{dir}/Var2Data"))?);
    let _meta = FixedMeta::parse(&c.stream(&format!("{dir}/FixedMeta"))?, 34, dir)?;
    // items secuenciales de 110 bytes (MPP14Reader: `new FixedData(110, ...)`)
    let data = FixedData::parse_sequential(&c.stream_decrypted(&format!("{dir}/FixedData"))?, 110);

    let off = |field: u16| fm.fixed_offset(field).map(|(_, o)| o);
    let uid_off = off(assignment::UNIQUE_ID).unwrap_or(0);
    let task_off = off(assignment::TASK_UNIQUE_ID).unwrap_or(4);
    let rsc_off = off(assignment::RESOURCE_UNIQUE_ID).unwrap_or(8);

    let mut assignments = Vec::new();
    for index in 0..data.item_count() {
        let Some(item) = data.item(index) else {
            continue;
        };
        let Some(uid) = get_i32(item, uid_off).map(|v| v as u32) else {
            continue;
        };
        // ResourceAssignmentFactory: solo asignaciones presentes en el VarMeta
        if var_meta.types_for(uid).next().is_none() {
            continue;
        }
        let Some(task_uid) = get_i32(item, task_off).filter(|&t| t > 0) else {
            continue;
        };
        let Some(rsc_uid) = get_i32(item, rsc_off).filter(|&r| r != NULL_RESOURCE_ID && r >= 0)
        else {
            continue;
        };
        if !valid_task_uids.contains(&(task_uid as u32)) {
            continue;
        }

        // WORK y UNITS pueden venir fixed o var según el field map
        let read_f64 = |field: u16| -> Option<f64> {
            if let Some((0, o)) = fm.fixed_offset(field) {
                return get_f64(item, o);
            }
            let key = fm.var_key(field)?;
            get_f64(var_data.byte_array(&var_meta, uid, key)?, 0)
        };

        assignments.push(Assignment {
            task_uid: task_uid as u32,
            resource_uid: rsc_uid as u32,
            units: read_f64(assignment::UNITS).map(|u| u / 100.0),
            work_hours: read_f64(assignment::WORK)
                .map(|w| if w.abs() < 1000.0 { 0.0 } else { w / 60000.0 }),
        });
    }
    Ok(assignments)
}
