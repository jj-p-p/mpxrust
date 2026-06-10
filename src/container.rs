//! L0/L1 — Apertura del Compound File, detección de versión y protección.
//!
//! La detección replica a MPXJ (`MPPReader.java` + `CompObj.java`): se lee el
//! stream `\x01CompObj` y se mira el string de formato (`MSProject.MPP14`).
//! Solo MPP14 está soportado; el resto produce `UnsupportedVersion` con un
//! mensaje accionable.
//!
//! Protección (port de `MPP14Reader.populateMemberData` +
//! `DocumentInputStreamFactory`): el stream raíz `Props14` trae
//! `PASSWORD_FLAG` (0x01 = password de lectura, 0x02 = de escritura) y
//! `ENCRYPTION_CODE`. Con password de lectura + hash presente el archivo es
//! ilegible (ni MPXJ sabe descifrarlo) → `PasswordProtected`. Con cualquier
//! flag, algunos streams van ofuscados con XOR de un byte — eso sí lo
//! manejamos (`stream_decrypted`).

use std::io::{Read, Seek};

use crate::blocks::Props;
use crate::error::MppError;

/// Storage raíz de los datos de proyecto en un MPP14.
const MPP14_ROOT: &str = "/   114";

/// Claves del Props14 raíz (PropsKey.java).
const PASSWORD_FLAG: i32 = 893386752;
const PROTECTION_PASSWORD_HASH: i32 = 893386756;
const ENCRYPTION_CODE: i32 = 893386759;

pub struct MppContainer<F> {
    comp: cfb::CompoundFile<F>,
    pub file_format: String,
    pub application_name: String,
    /// Versión interna de la app que escribió el archivo (14 = Project 2010,
    /// 15 = 2013, 16 = 2016+). Decide variantes de layout (bits de metadata,
    /// offsets de lag en relaciones).
    pub application_version: u32,
    /// Máscara XOR de la ofuscación por password (0 = sin ofuscar).
    encryption_mask: u8,
}

impl<F: Read + Seek> MppContainer<F> {
    /// Abre el contenedor, valida que sea un MPP14 legible.
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

        // "Microsoft Project 14.0" → 14 (CompObj.java extrae el entero del nombre)
        let application_version = application_name
            .split(|c: char| !c.is_ascii_digit())
            .find(|s| !s.is_empty())
            .and_then(|s| s.parse().ok())
            .unwrap_or(14);

        // Protección: Props14 del storage RAÍZ (no confundir con `   114/Props`)
        let mut encryption_mask = 0u8;
        if comp.is_stream("/Props14") {
            let mut buf = Vec::new();
            comp.open_stream("/Props14")
                .map_err(|e| MppError::corrupt("Props14", e.to_string()))?
                .read_to_end(&mut buf)?;
            let props = Props::parse(&buf, "Props14")?;

            let flag = props
                .byte_array(PASSWORD_FLAG)
                .and_then(|b| b.first().copied())
                .unwrap_or(0);
            let read_password = flag & 0x1 != 0;
            let encryption_xml = props.byte_array(PROTECTION_PASSWORD_HASH).is_some();
            // MPXJ: flag de lectura sin el XML de cifrado = archivo abrible
            if read_password && encryption_xml {
                return Err(MppError::PasswordProtected);
            }
            if flag != 0 {
                let code = props
                    .byte_array(ENCRYPTION_CODE)
                    .and_then(|b| b.first().copied())
                    .unwrap_or(0);
                encryption_mask = if code == 0 { 0 } else { 0xFF - code };
            }
        }

        Ok(MppContainer {
            comp,
            file_format,
            application_name,
            application_version,
            encryption_mask,
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

    /// Igual que [`stream`](Self::stream) pero aplicando la máscara XOR de la
    /// ofuscación por password. MPXJ solo la aplica a los streams que abre
    /// vía `DocumentInputStreamFactory`: `Props` del proyecto, FixedData de
    /// recursos/asignaciones/relaciones — NUNCA a los de tareas.
    pub fn stream_decrypted(&mut self, relative: &str) -> Result<Vec<u8>, MppError> {
        let mut buf = self.stream(relative)?;
        if self.encryption_mask != 0 {
            for b in &mut buf {
                *b ^= self.encryption_mask;
            }
        }
        Ok(buf)
    }

    pub fn has_stream(&self, relative: &str) -> bool {
        self.comp.is_stream(format!("{MPP14_ROOT}/{relative}"))
    }

    /// `Props` del proyecto (`   114/Props` — ofuscable).
    pub fn project_props(&mut self) -> Result<Props, MppError> {
        Props::parse(&self.stream_decrypted("Props")?, "Props")
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
