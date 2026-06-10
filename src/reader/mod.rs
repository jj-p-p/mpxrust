//! L3 — Readers de dominio: convierten bloques en el modelo público.
//!
//! Port del subconjunto de `MPP14Reader.java` que cubre el alcance v1
//! (docs/03): tareas, relaciones, recursos y asignaciones.

pub mod project_props;
pub mod relations;
pub mod resources;
pub mod tasks;

use std::collections::BTreeSet;
use std::io::{Read, Seek};

use crate::container::MppContainer;
use crate::error::MppError;
use crate::field_map::FieldMap;
use crate::model::ProjectFile;

pub fn read_project_file<F: Read + Seek>(c: &mut MppContainer<F>) -> Result<ProjectFile, MppError> {
    let props = c.project_props()?;
    let properties = project_props::read(&props);
    let mpd = project_props::minutes_per_day(&properties);

    let task_fm = FieldMap::for_tasks(&props)?;
    let blocks = tasks::read_blocks(c)?;
    let mut tasks = tasks::read_tasks(&task_fm, &blocks, c.application_version, mpd);

    relations::attach_relations(c, &mut tasks, mpd)?;

    let resource_fm = FieldMap::for_resources(&props)?;
    let resources = resources::read_resources(c, &resource_fm)?;

    let task_uids: BTreeSet<u32> = tasks.iter().map(|t| t.uid).collect();
    let assignment_fm = FieldMap::for_assignments(&props)?;
    let assignments = resources::read_assignments(c, &assignment_fm, &task_uids)?;

    Ok(ProjectFile {
        properties,
        tasks,
        resources,
        assignments,
    })
}
