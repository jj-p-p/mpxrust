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

impl RelationType {
    /// Sigla estándar de MS Project (FS/SS/FF/SF).
    pub fn code(&self) -> &'static str {
        match self {
            RelationType::FinishFinish => "FF",
            RelationType::FinishStart => "FS",
            RelationType::StartFinish => "SF",
            RelationType::StartStart => "SS",
        }
    }
}

impl ProjectFile {
    /// Serializa al shape `{project, issues[]}` que consume el módulo
    /// Importar Plan de jirast (`import/parse.rs`) — el mismo que emitía el
    /// analizador upstream. Los campos propios de esa herramienta que no
    /// salen del `.mpp` (tier, labels, jira_key) los deriva jirast.
    pub fn to_jirast_json(&self, mpp_name: &str) -> serde_json::Value {
        let date = |s: &Option<String>| -> serde_json::Value {
            s.as_deref().map(|d| d[..10].to_string()).into()
        };

        // título: la primera tarea resumen de nivel 1 (uid 0 es la raíz
        // artificial con el nombre del archivo); fallback al Props/raíz
        let title = self
            .tasks
            .iter()
            .find(|t| t.outline_level == Some(1) && t.name.is_some())
            .or_else(|| self.tasks.iter().find(|t| t.uid == 0))
            .and_then(|t| t.name.clone())
            .or_else(|| self.properties.title.clone());

        let resource_name = |uid: u32| -> Option<String> {
            self.resources
                .iter()
                .find(|r| r.uid == uid)
                .and_then(|r| r.name.clone())
        };

        let issues: Vec<serde_json::Value> = self
            .tasks
            .iter()
            .filter(|t| t.uid != 0 && t.name.is_some())
            .map(|t| {
                let assignees: Vec<String> = self
                    .assignments
                    .iter()
                    .filter(|a| a.task_uid == t.uid)
                    .filter_map(|a| resource_name(a.resource_uid))
                    .collect();
                let dependencies: Vec<serde_json::Value> = t
                    .predecessors
                    .iter()
                    .map(|r| {
                        serde_json::json!({
                            "predecessor_uid": r.predecessor_uid,
                            "type": r.kind.code(),
                            "lag_days": r.lag_days,
                        })
                    })
                    .collect();
                serde_json::json!({
                    "uid": t.uid,
                    "id": t.id,
                    "wbs": t.wbs,
                    "parent_uid": t.parent_uid.filter(|&p| p != 0),
                    "outline_level": t.outline_level,
                    "summary": t.name,
                    "is_milestone": t.is_milestone,
                    "is_summary": t.is_summary,
                    "start_date": date(&t.start_date),
                    "due_date": date(&t.finish_date),
                    "duration_days": t.duration_days,
                    "work_hours": t.work_hours,
                    "percent_complete": t.percent_complete,
                    "priority": t.priority,
                    "constraint_type": t.constraint_type,
                    "constraint_date": date(&t.constraint_date),
                    "deadline": date(&t.deadline),
                    "cost": t.cost,
                    "notes": t.notes,
                    "assignees": assignees,
                    "resources": assignees,
                    "dependencies": dependencies,
                })
            })
            .collect();

        let dep_count: usize = self
            .tasks
            .iter()
            .filter(|t| t.uid != 0 && t.name.is_some())
            .map(|t| t.predecessors.len())
            .sum();

        serde_json::json!({
            "project": {
                "title": title,
                "mpp_name": mpp_name,
                "start": date(&self.properties.start_date),
                "finish": date(&self.properties.finish_date),
                "dep_count": dep_count,
            },
            "issues": issues,
        })
    }
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Assignment {
    pub task_uid: u32,
    pub resource_uid: u32,
    pub units: Option<f64>,
    pub work_hours: Option<f64>,
}
