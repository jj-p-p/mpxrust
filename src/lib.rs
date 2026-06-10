//! # mpxrust
//!
//! Lector **en Rust puro** de archivos `.mpp` de Microsoft Project
//! (formato **MPP14**: Project 2010 a 365). Port del subconjunto de lectura
//! de [MPXJ](https://www.mpxj.org/) — sin Java, sin sidecars, < 1 MB.
//!
//! ```no_run
//! let project = mpxrust::read_mpp("plan.mpp").unwrap();
//! for task in &project.tasks {
//!     println!("{:?} {:?}", task.uid, task.name);
//! }
//! ```
//!
//! ## Estado (roadmap en docs/03-diseno-crate.md)
//!
//! - **H1 (actual):** contenedor + capa de bloques (`Props`, `VarMeta`,
//!   `Var2Data`, `FixedMeta`, `FixedData`) y detección de versión vía CompObj.
//!   [`read_mpp`] valida el archivo y devuelve el modelo aún sin poblar;
//!   [`inspect_mpp`] expone las estadísticas de bloques.
//! - **H2:** FieldMap14 (offsets dinámicos por archivo).
//! - **H3+:** población de tareas, dependencias, recursos, calendarios.
//!
//! ## Licencia
//!
//! LGPL-2.1-or-later — este crate es una obra derivada de MPXJ (LGPL 2.1).

mod container;
mod error;
mod util;

pub mod blocks;
pub mod model;

use std::io::{Cursor, Read, Seek};
use std::path::Path;

pub use error::MppError;
pub use model::{
    Assignment, ConstraintType, ProjectFile, ProjectProperties, Relation, RelationType, Resource,
    Task,
};

use container::{MppContainer, RSC_FIXED_META_ITEM_SIZE, TASK_FIXED_META_ITEM_SIZE};

/// Lee un `.mpp` desde disco.
pub fn read_mpp(path: impl AsRef<Path>) -> Result<ProjectFile, MppError> {
    read_project(MppContainer::open(std::fs::File::open(path)?)?)
}

/// Lee un `.mpp` desde memoria (p. ej. bytes recibidos por un comando Tauri).
pub fn read_mpp_bytes(data: &[u8]) -> Result<ProjectFile, MppError> {
    read_project(MppContainer::open(Cursor::new(data))?)
}

fn read_project<F: Read + Seek>(mut container: MppContainer<F>) -> Result<ProjectFile, MppError> {
    // H1: validar estructura completa (todos los bloques parsean).
    // La población del modelo llega con el FieldMap (H2) y los readers (H3+).
    let _props = container.project_props()?;
    let _tasks = container.block_set("TBkndTask", TASK_FIXED_META_ITEM_SIZE, 0)?;
    let _resources = container.block_set("TBkndRsc", RSC_FIXED_META_ITEM_SIZE, 0)?;

    Ok(ProjectFile::default())
}

/// Resumen estructural de un `.mpp` — para diagnóstico y tests de la capa
/// de bloques. No interpreta semántica de campos.
#[derive(Debug, serde::Serialize)]
pub struct MppSummary {
    /// Nombre de la aplicación que escribió el archivo (CompObj).
    pub application_name: String,
    /// Identificador de formato (`MSProject.MPP14`).
    pub file_format: String,
    /// Cantidad de propiedades del Props del proyecto.
    pub project_props_count: usize,
    pub tasks: BlockSetSummary,
    pub resources: BlockSetSummary,
}

#[derive(Debug, serde::Serialize)]
pub struct BlockSetSummary {
    /// UIDs distintos en el VarMeta.
    pub var_uid_count: usize,
    /// Entradas (uid, tipo) en el VarMeta.
    pub var_entry_count: usize,
    /// Items declarados por el FixedMeta.
    pub fixed_item_count: usize,
    /// Items del FixedData realmente legibles.
    pub fixed_populated_count: usize,
}

/// Abre un `.mpp` y devuelve estadísticas de sus bloques.
pub fn inspect_mpp(path: impl AsRef<Path>) -> Result<MppSummary, MppError> {
    let mut container = MppContainer::open(std::fs::File::open(path)?)?;

    let props = container.project_props()?;
    let tasks = container.block_set("TBkndTask", TASK_FIXED_META_ITEM_SIZE, 0)?;
    let resources = container.block_set("TBkndRsc", RSC_FIXED_META_ITEM_SIZE, 0)?;

    let summarize = |b: &container::BlockSet| BlockSetSummary {
        var_uid_count: b.var_meta.uid_count(),
        var_entry_count: b.var_meta.entry_count(),
        fixed_item_count: b.fixed_meta.item_count(),
        fixed_populated_count: b.fixed_data.iter().count(),
    };

    Ok(MppSummary {
        application_name: container.application_name.clone(),
        file_format: container.file_format.clone(),
        project_props_count: props.len(),
        tasks: summarize(&tasks),
        resources: summarize(&resources),
    })
}
