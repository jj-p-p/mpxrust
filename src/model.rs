//! L4 — Modelo de dominio público del crate.
//!
//! Modelo propio y lean (no es el modelo de MPXJ): solo el alcance v1
//! definido en `docs/03-diseno-crate.md` — lo que necesita el importador
//! de jirast. Todo serializable con serde.

use serde::{Deserialize, Serialize};

/// Resultado de leer un `.mpp`.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ProjectFile {
    pub properties: ProjectProperties,
    pub tasks: Vec<Task>,
    pub resources: Vec<Resource>,
    pub assignments: Vec<Assignment>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ProjectProperties {
    pub title: Option<String>,
    /// Fechas ISO-8601 (`YYYY-MM-DD`).
    pub start_date: Option<String>,
    pub finish_date: Option<String>,
    pub minutes_per_day: Option<u32>,
    pub minutes_per_week: Option<u32>,
    pub days_per_month: Option<u32>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Task {
    pub uid: u32,
    pub id: Option<u32>,
    pub name: Option<String>,
    pub wbs: Option<String>,
    pub outline_level: Option<u32>,
    /// UID de la tarea resumen padre (None = raíz).
    pub parent_uid: Option<u32>,
    pub is_summary: bool,
    pub is_milestone: bool,
    pub start_date: Option<String>,
    pub finish_date: Option<String>,
    pub duration_days: Option<f64>,
    pub work_hours: Option<f64>,
    pub percent_complete: Option<u32>,
    pub priority: Option<u32>,
    pub constraint_type: Option<ConstraintType>,
    pub constraint_date: Option<String>,
    pub deadline: Option<String>,
    pub cost: Option<f64>,
    pub notes: Option<String>,
    pub predecessors: Vec<Relation>,
}

/// Dependencia entre tareas.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    pub predecessor_uid: u32,
    pub kind: RelationType,
    pub lag_days: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationType {
    FinishFinish,
    FinishStart,
    StartFinish,
    StartStart,
}

/// Los 8 tipos de constraint de MS Project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConstraintType {
    AsSoonAsPossible,
    AsLateAsPossible,
    MustStartOn,
    MustFinishOn,
    StartNoEarlierThan,
    StartNoLaterThan,
    FinishNoEarlierThan,
    FinishNoLaterThan,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Resource {
    pub uid: u32,
    pub name: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Assignment {
    pub task_uid: u32,
    pub resource_uid: u32,
    pub units: Option<f64>,
    pub work_hours: Option<f64>,
}
