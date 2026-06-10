//! Golden test H5: paridad campo a campo contra la salida del analizador
//! upstream (la herramienta cerrada que mpxrust reemplaza).
//!
//! Corpus privado: requiere `plan_anco.mpp` + el JSON del analizador en la
//! carpeta madre. Si no están, el test se salta (CI sin corpus privado).
//!
//! Nota de alcance: el analizador filtraba los niveles 0–2 (raíz, proyecto,
//! fases "Hito N") y emitía 90 issues; mpxrust entrega TODAS las tareas y el
//! filtro es política del consumidor. Por eso comparamos sobre la
//! intersección de uids, que debe cubrir el 100% de los suyos.

use serde_json::Value;
use std::collections::BTreeMap;
use std::path::Path;

fn load() -> Option<(Value, Value)> {
    let mpp = Path::new("../plan_anco.mpp");
    let json = Path::new("../planificacion_proyecto_ancov3/planificacion_proyecto_ancov3.json");
    if !mpp.exists() || !json.exists() {
        eprintln!("AVISO: corpus privado no disponible; test de paridad saltado");
        return None;
    }
    let ours = mpxrust::read_mpp(mpp)
        .expect("read_mpp")
        .to_jirast_json("plan_anco.mpp");
    let theirs: Value =
        serde_json::from_str(&std::fs::read_to_string(json).expect("leer json")).expect("parsear");
    Some((ours, theirs))
}

fn by_uid(issues: &Value) -> BTreeMap<u64, &Value> {
    issues
        .as_array()
        .unwrap()
        .iter()
        .map(|i| (i["uid"].as_u64().unwrap(), i))
        .collect()
}

fn date_only(v: &Value) -> Option<&str> {
    v.as_str().map(|s| &s[..10.min(s.len())])
}

#[test]
fn parity_with_upstream_analyzer() {
    let Some((ours, theirs)) = load() else { return };

    // proyecto
    assert_eq!(ours["project"]["title"], theirs["project"]["title"]);
    assert_eq!(ours["project"]["start"], theirs["project"]["start"]);
    assert_eq!(ours["project"]["finish"], theirs["project"]["finish"]);

    let om = by_uid(&ours["issues"]);
    let tm = by_uid(&theirs["issues"]);
    assert_eq!(tm.len(), 90, "el golden de referencia trae 90 issues");

    let mut diffs: Vec<String> = Vec::new();
    for (uid, t) in &tm {
        let Some(o) = om.get(uid) else {
            diffs.push(format!("uid {uid}: ausente en mpxrust"));
            continue;
        };
        let mut check = |field: &str, ov: String, tv: String| {
            if ov != tv {
                diffs.push(format!("uid {uid} {field}: mpxrust={ov} analizador={tv}"));
            }
        };
        check(
            "summary",
            o["summary"].to_string(),
            t["summary"].to_string(),
        );
        check(
            "start_date",
            format!("{:?}", date_only(&o["start_date"])),
            format!("{:?}", date_only(&t["start_date"])),
        );
        check(
            "due_date",
            format!("{:?}", date_only(&o["due_date"])),
            format!("{:?}", date_only(&t["due_date"])),
        );
        // el analizador truncaba a entero (0.5 días → 0); mpxrust conserva
        // el decimal real, que es lo correcto — comparamos con su truncación
        check(
            "duration_days",
            format!("{}", o["duration_days"].as_f64().unwrap_or(-1.0).trunc()),
            format!("{}", t["duration_days"].as_f64().unwrap_or(-1.0).trunc()),
        );
        check(
            "work_hours",
            format!("{:.1}", o["work_hours"].as_f64().unwrap_or(-1.0)),
            format!("{:.1}", t["work_hours"].as_f64().unwrap_or(-1.0)),
        );
        check(
            "percent_complete",
            format!("{}", o["percent_complete"].as_u64().unwrap_or(0)),
            format!("{}", t["percent_complete"].as_u64().unwrap_or(0)),
        );
        check(
            "priority",
            o["priority"].to_string(),
            t["priority"].to_string(),
        );
        check(
            "is_milestone",
            o["is_milestone"].to_string(),
            t["is_milestone"].to_string(),
        );
        check(
            "is_summary",
            o["is_summary"].to_string(),
            t["is_summary"].to_string(),
        );
        check(
            "outline_level",
            o["outline_level"].to_string(),
            t["outline_level"].to_string(),
        );
        check(
            "constraint_type",
            o["constraint_type"].to_string(),
            t["constraint_type"].to_string(),
        );

        let deps = |v: &Value| -> Vec<(u64, String, i64)> {
            let mut d: Vec<_> = v["dependencies"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| {
                    (
                        r["predecessor_uid"].as_u64().unwrap(),
                        r["type"].as_str().unwrap_or("FS").to_string(),
                        r["lag_days"].as_f64().unwrap_or(0.0).round() as i64,
                    )
                })
                .collect();
            d.sort();
            d
        };
        check(
            "dependencies",
            format!("{:?}", deps(o)),
            format!("{:?}", deps(t)),
        );
    }

    assert!(
        diffs.is_empty(),
        "{} diferencias:\n{}",
        diffs.len(),
        diffs.join("\n")
    );

    // las 85 dependencias del analizador están íntegras
    let their_deps: usize = tm
        .values()
        .map(|t| t["dependencies"].as_array().unwrap().len())
        .sum();
    assert_eq!(their_deps, 85);
}
