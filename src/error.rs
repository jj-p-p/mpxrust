//! Errores públicos del crate.

/// Error al leer un archivo `.mpp`.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum MppError {
    /// El archivo no es un Compound File Binary (OLE2) válido.
    #[error("el archivo no es un Compound File válido (¿es realmente un .mpp?): {0}")]
    NotACompoundFile(String),

    /// Es un CFB válido pero de una versión de Project que no soportamos.
    /// Solo se soporta MPP14 (Project 2010 a 365).
    #[error(
        "versión de Project no soportada: {found}. Solo se soporta MPP14 (Project 2010–365); guarda el archivo con una versión moderna de MS Project"
    )]
    UnsupportedVersion { found: String },

    /// El archivo está protegido con contraseña.
    #[error("el archivo está protegido con contraseña; guárdalo sin contraseña e intenta de nuevo")]
    PasswordProtected,

    /// Estructura interna corrupta o inesperada.
    #[error("estructura interna inválida en {context}: {detail}")]
    Corrupt {
        /// Stream o bloque donde se detectó el problema (p. ej. `TBkndTask/VarMeta`).
        context: String,
        detail: String,
    },

    #[error("error de E/S: {0}")]
    Io(#[from] std::io::Error),
}

impl MppError {
    pub(crate) fn corrupt(context: impl Into<String>, detail: impl Into<String>) -> Self {
        MppError::Corrupt {
            context: context.into(),
            detail: detail.into(),
        }
    }
}
