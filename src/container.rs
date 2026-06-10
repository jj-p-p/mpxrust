//! L0/L1 — Apertura del Compound File y detección de versión.
//!
//! La detección replica a MPXJ (`MPPReader.java` + `CompObj.java`): se lee el
//! stream `\x01CompObj` y se mira el string de formato (`MSProject.MPP14`).
//! Solo MPP14 está soportado; el resto produce `UnsupportedVersion` con un
//! mensaje accionable.

use std::io::{Read, Seek};

use crate::blocks::{FixedData, FixedMeta, Props, Var2Data, VarMeta};
use crate::error::MppError;

/// Storage raíz de los datos de proyecto en un MPP14.
const MPP14_ROOT: &str = "/   114";

/// Tamaño de item del FixedMeta de tareas en MPP14 (MPXJ `MPP14Reader`).
pub const TASK_FIXED_META_ITEM_SIZE: usize = 47;
/// Tamaño de item del FixedMeta de recursos en MPP14.
pub const RSC_FIXED_META_ITEM_SIZE: usize = 37;
/// Candidatos de tamaño para el Fixed2Meta de tareas (varía según qué
/// versión de Project escribió el archivo). Se consume en H3 junto a
/// `FixedMeta::parse_with_candidate_sizes`.
#[allow(dead_code)]
pub const TASK_FIXED2_META_ITEM_SIZES: &[usize] = &[92, 93, 94, 95, 96];

pub struct MppContainer<F> {
    comp: cfb::CompoundFile<F>,
    pub file_format: String,
    pub application_name: String,
}

/// Los cuatro streams estándar de un directorio `TBknd*`, ya parseados.
pub struct BlockSet {
    pub var_meta: VarMeta,
    // los readers de H3+ leen los campos var; hoy solo lo recorren los tests
    #[allow(dead_code)]
    pub var_data: Var2Data,
    pub fixed_meta: FixedMeta,
    pub fixed_data: FixedData,
}

impl<F: Read + Seek> MppContainer<F> {
    /// Abre el contenedor, valida que sea un MPP14 y deja listo el acceso
    /// a los streams del proyecto.
    pub fn open(inner: F) -> Result<Self, MppError> {
        let mut comp = cfb::CompoundFile::open(inner)
            .map_err(|e| MppError::NotACompoundFile(e.to_string()))?;

        let (application_name, file_format) = read_comp_obj(&mut comp)?;

        if file_format != "MSProject.MPP14" {
            let found = match file_format.as_str() {
                "MSProject.MPP12" => "MPP12 (Project 2007)".to_string(),
                "MSProject.MPP9" => "MPP9 (Project 2000–2003)".to_string(),
                "MSProject.MPP8" => "MPP8 (Project 98)".to_string(),
                "MSProject.MPP4" => "MPP4 (Project 4.0)".to_string(),
                "" => "desconocida (sin CompObj legible)".to_string(),
                other => other.to_string(),
            };
            return Err(MppError::UnsupportedVersion { found });
        }
        if !comp.is_storage(MPP14_ROOT) {
            return Err(MppError::corrupt(
                "raíz",
                "CompObj dice MPP14 pero falta el storage '   114'",
            ));
        }

        Ok(MppContainer {
            comp,
            file_format,
            application_name,
        })
    }

    /// Lee completo un stream relativo al storage del proyecto
    /// (p. ej. `TBkndTask/VarMeta`).
    pub fn stream(&mut self, relative: &str) -> Result<Vec<u8>, MppError> {
        let path = format!("{MPP14_ROOT}/{relative}");
        let mut buf = Vec::new();
        self.comp
            .open_stream(&path)
            .map_err(|e| MppError::corrupt(relative, e.to_string()))?
            .read_to_end(&mut buf)?;
        Ok(buf)
    }

    /// Streams opcionales (p. ej. `TBkndTask/Props` con custom fields) — H3+.
    #[allow(dead_code)]
    pub fn has_stream(&self, relative: &str) -> bool {
        self.comp.is_stream(format!("{MPP14_ROOT}/{relative}"))
    }

    /// `Props` del proyecto (storage raíz `   114/Props`).
    pub fn project_props(&mut self) -> Result<Props, MppError> {
        Props::parse(&self.stream("Props")?, "Props")
    }

    /// Parsea los cuatro streams estándar de un directorio `TBknd*`.
    ///
    /// `max_fixed_item_size = 0` significa sin límite (en el crate completo
    /// el límite sale del FieldMap — `getMaxFixedDataSize`, ver docs/03 H2).
    pub fn block_set(
        &mut self,
        dir: &str,
        fixed_meta_item_size: usize,
        max_fixed_item_size: usize,
    ) -> Result<BlockSet, MppError> {
        let var_meta = VarMeta::parse(&self.stream(&format!("{dir}/VarMeta"))?, dir)?;
        let var_data = Var2Data::parse(&var_meta, &self.stream(&format!("{dir}/Var2Data"))?);
        let fixed_meta = FixedMeta::parse(
            &self.stream(&format!("{dir}/FixedMeta"))?,
            fixed_meta_item_size,
            dir,
        )?;
        let fixed_data = FixedData::parse(
            &fixed_meta,
            &self.stream(&format!("{dir}/FixedData"))?,
            max_fixed_item_size,
        );
        Ok(BlockSet {
            var_meta,
            var_data,
            fixed_meta,
            fixed_data,
        })
    }
}

/// Port de `CompObj.java`: skip 28 bytes; luego `len:i32` + string ANSI
/// (el largo incluye el terminador nul) tres veces: applicationName,
/// fileFormat, applicationID. Devuelve (applicationName, fileFormat).
fn read_comp_obj<F: Read + Seek>(
    comp: &mut cfb::CompoundFile<F>,
) -> Result<(String, String), MppError> {
    let mut buf = Vec::new();
    comp.open_stream("/\u{1}CompObj")
        .map_err(|e| MppError::corrupt("CompObj", e.to_string()))?
        .read_to_end(&mut buf)?;

    let mut pos = 28usize;
    let mut next_string = || -> Option<String> {
        let len = crate::util::get_i32(&buf, pos)? as usize;
        let s = buf.get(pos + 4..pos + 4 + len.checked_sub(1)?)?;
        pos += 4 + len;
        Some(s.iter().map(|&b| b as char).collect())
    };

    let application_name = next_string()
        .ok_or_else(|| MppError::corrupt("CompObj", "no se pudo leer applicationName"))?;
    // "Microsoft Project 4.0" no trae fileFormat; cualquier otro sí
    let file_format = if application_name == "Microsoft Project 4.0" {
        "MSProject.MPP4".to_string()
    } else {
        next_string().unwrap_or_default()
    };
    Ok((application_name, file_format))
}
