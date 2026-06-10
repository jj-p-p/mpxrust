//! Test de integración H1: la capa de bloques parsea un MPP14 real completo.
//!
//! Usa el corpus PRIVADO (`plan_anco.mpp`, datos reales de Sonda — NO se
//! commitea a un repo público). Si el archivo no está, el test se da por
//! pasado con un aviso: así CI corre sin el corpus privado.
//!
//! Los valores esperados provienen del spike de la etapa de exploración
//! (docs/01-viabilidad.md §6), verificados contra la salida del analizador
//! de referencia.

use std::path::PathBuf;

fn private_mpp() -> Option<PathBuf> {
    let candidates = [
        std::env::var("MPXRUST_PRIVATE_MPP").ok().map(PathBuf::from),
        Some(PathBuf::from("../plan_anco.mpp")),
        Some(PathBuf::from("tests/data/private/plan_anco.mpp")),
    ];
    let found = candidates.into_iter().flatten().find(|p| p.exists());
    if found.is_none() {
        eprintln!("AVISO: corpus privado no disponible; test saltado (set MPXRUST_PRIVATE_MPP)");
    }
    found
}

#[test]
fn plan_anco_blocks_parse_with_expected_counts() {
    let Some(path) = private_mpp() else { return };

    let summary = mpxrust::inspect_mpp(&path).expect("inspect plan_anco.mpp");

    assert_eq!(summary.file_format, "MSProject.MPP14");
    assert!(
        summary.application_name.starts_with("Microsoft"),
        "{}",
        summary.application_name
    );
    assert!(summary.project_props_count > 0);

    // números medidos en el spike (90 tareas reales + resumen raíz + nulas)
    assert_eq!(summary.tasks.var_uid_count, 106);
    assert_eq!(summary.tasks.var_entry_count, 1124);
    assert_eq!(summary.tasks.fixed_item_count, 119);
    assert!(summary.tasks.fixed_populated_count >= 90);
}

#[test]
fn plan_anco_read_mpp_validates_ok() {
    let Some(path) = private_mpp() else { return };
    // H1: read_mpp valida la estructura completa sin error (modelo aún vacío)
    mpxrust::read_mpp(&path).expect("read_mpp");
}

#[test]
fn non_mpp_bytes_give_clear_error() {
    let err = mpxrust::read_mpp_bytes(b"esto no es un compound file").unwrap_err();
    assert!(
        matches!(err, mpxrust::MppError::NotACompoundFile(_)),
        "{err:?}"
    );
}
