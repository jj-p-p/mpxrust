//! # mpxrust
//!
//! Lector **en Rust puro** de archivos `.mpp` de Microsoft Project
//! (formato **MPP14**: Project 2010 a 365). Port del subconjunto de lectura
//! de [MPXJ](https://www.mpxj.org/) — sin Java, sin sidecars, < 1 MB.
//!
//! ```no_run
//! let project = mpxrust::read_mpp("plan.mpp").unwrap();
//! for task in &project.tasks {
//!     println!("{} {:?}", task.uid, task.name);
//! }
//! ```
//!
//! ## Estado (roadmap en docs/03-diseno-crate.md)
//!
//! - **H1–H5:** contenedor + bloques + FieldMap dinámico + tareas (jerarquía,
//!   fechas, duración, trabajo, %, constraints, hitos), dependencias con lag,
//!   recursos y asignaciones. [`ProjectFile::to_jirast_json`] emite el shape
//!   `{project, issues[]}` que consume jirast.
//! - **Pendiente:** calendarios con excepciones, custom fields, baselines.
//!
//! ## Licencia
//!
//! LGPL-2.1-or-later — este crate es una obra derivada de MPXJ (LGPL 2.1).

mod container;
mod dec;
mod error;
mod field_map;
mod reader;
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

use container::MppContainer;

/// Lee un `.mpp` desde disco.
pub fn read_mpp(path: impl AsRef<Path>) -> Result<ProjectFile, MppError> {
    let mut container = MppContainer::open(std::fs::File::open(path)?)?;
    reader::read_project_file(&mut container)
}

/// Lee un `.mpp` desde memoria (p. ej. bytes recibidos por un comando Tauri).
pub fn read_mpp_bytes(data: &[u8]) -> Result<ProjectFile, MppError> {
    let mut container = MppContainer::open(Cursor::new(data))?;
    reader::read_project_file(&mut container)
}

/// Resumen estructural de un `.mpp` — para diagnóstico. No interpreta
/// semántica más allá de la necesaria para contar.
#[derive(Debug, serde::Serialize)]
pub struct MppSummary {
    /// Nombre de la aplicación que escribió el archivo (CompObj).
    pub application_name: String,
    /// Identificador de formato (`MSProject.MPP14`).
    pub file_format: String,
    /// Versión interna de la app (14 = Project 2010, 15 = 2013, 16 = 2016+).
    pub application_version: u32,
    /// Cantidad de propiedades del Props del proyecto.
    pub project_props_count: usize,
    pub tasks: BlockSetSummary,
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

/// Abre un `.mpp` y devuelve estadísticas de sus bloques de tareas.
pub fn inspect_mpp(path: impl AsRef<Path>) -> Result<MppSummary, MppError> {
    inspect(MppContainer::open(std::fs::File::open(path)?)?)
}

fn inspect<F: Read + Seek>(mut container: MppContainer<F>) -> Result<MppSummary, MppError> {
    let props = container.project_props()?;
    let blocks = reader::tasks::read_blocks(&mut container)?;

    Ok(MppSummary {
        application_name: container.application_name.clone(),
        file_format: container.file_format.clone(),
        application_version: container.application_version,
        project_props_count: props.len(),
        tasks: BlockSetSummary {
            var_uid_count: blocks.var_meta.uid_count(),
            var_entry_count: blocks.var_meta.entry_count(),
            fixed_item_count: blocks.fixed_meta.item_count(),
            fixed_populated_count: blocks.fixed_data.iter().count(),
        },
    })
}
