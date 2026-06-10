//! Vuelca un `.mpp` al JSON estilo jirast, para inspección manual.
//!
//! ```bash
//! cargo run --example dump -- ../plan_anco.mpp
//! ```

fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "../plan_anco.mpp".into());
    let name = std::path::Path::new(&path)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    match mpxrust::read_mpp(&path) {
        Ok(project) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&project.to_jirast_json(&name)).unwrap()
            )
        }
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(1);
        }
    }
}
