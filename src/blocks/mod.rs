//! L1 — Bloques genéricos del formato MPP (sin semántica de dominio).
//!
//! Port fiel de las estructuras de MPXJ (`org.mpxj.mpp`); cada módulo cita
//! su archivo Java de origen. La semántica (qué campo vive en qué offset)
//! pertenece a L2 (`field_map`) y L3 (`reader`), no a esta capa.

pub mod fixed_data;
pub mod fixed_meta;
pub mod props;
pub mod var2_data;
pub mod var_meta;

pub use fixed_data::FixedData;
pub use fixed_meta::FixedMeta;
pub use props::Props;
pub use var_meta::VarMeta;
pub use var2_data::Var2Data;
