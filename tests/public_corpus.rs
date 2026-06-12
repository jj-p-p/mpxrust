//! Corpus público: archivos de prueba MPP14 del repo de MPXJ
//! (`junit/data`, LGPL — misma licencia que este crate).
//!
//! Mitigación del riesgo R6 (overfitting al único .mpp real): cada archivo
//! debe abrirse sin error y producir tareas con nombre. Escritos por
//! Project 2010 y 2013 — cubren las variantes de metadata/lag por versión.

use std::path::PathBuf;

fn corpus() -> Vec<PathBuf> {
    let dir = PathBuf::from("tests/data/public");
    let Ok(entries) = std::fs::read_dir(&dir) else {
        eprintln!("AVISO: corpus público no descargado; test saltado");
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "mpp"))
        .collect();
    files.sort();
    files
}

#[test]
fn public_corpus_parses_with_tasks() {
    let files = corpus();
    let mut failures = Vec::new();

    for path in &files {
        let name = path.file_name().unwrap().to_string_lossy().into_owned();
        match mpxrust::read_mpp(path) {
            Ok(project) => {
                let named = project.tasks.iter().filter(|t| t.name.is_some()).count();
                let with_dates = project
                    .tasks
                    .iter()
                    .filter(|t| t.start_date.is_some() && t.finish_date.is_some())
                    .count();
                println!(
                    "{name}: {} tareas ({named} con nombre, {with_dates} con fechas), \
                     {} recursos, {} asignaciones",
                    project.tasks.len(),
                    project.resources.len(),
                    project.assignments.len()
                );
                if named == 0 {
                    failures.push(format!("{name}: 0 tareas con nombre"));
                }
            }
            Err(e) => failures.push(format!("{name}: {e}")),
        }
    }

    assert!(
        failures.is_empty(),
        "fallas en corpus público:\n{}",
        failures.join("\n")
    );
}
