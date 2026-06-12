# mpxrust

Lector **en Rust puro** de archivos `.mpp` de Microsoft Project (formato
**MPP14**: Project 2010 → 365). Port del subconjunto de lectura de
[MPXJ](https://www.mpxj.org/) — sin JVM, sin procesos externos, sin peso
extra en el binario.

**Cobertura actual**: tareas (jerarquía, fechas, duración, trabajo,
% completado, constraints, hitos), dependencias FS/SS/FF/SF con lag,
recursos y asignaciones. Validado contra el corpus de pruebas de MPXJ
(archivos escritos por Project 2010 y 2013) y contra proyectos reales.
Fuera de alcance por ahora: calendarios con excepciones, custom fields y
baselines. Detalle de hitos y desviaciones conocidas en [`PLAN.md`](PLAN.md).

## Uso

```rust
// desde disco
let project = mpxrust::read_mpp("plan.mpp")?;

// desde memoria (p. ej. bytes recibidos por un comando Tauri)
let project = mpxrust::read_mpp_bytes(&bytes)?;

for task in &project.tasks {
    println!("{:>4} {:?} {:?}", task.uid, task.name, task.start_date);
}
```

Los errores distinguen las causas que el usuario final puede accionar:
`UnsupportedVersion` (incluye la versión detectada, p. ej. "MPP12 —
Project 2007"), `PasswordProtected`, `NotACompoundFile` y `Corrupt`.

## Instalación

Dependencia Cargo estándar, linkeada estáticamente — nada que instalar ni
cargar en runtime:

```toml
[dependencies]
mpxrust = { git = "https://github.com/jj-p-p/mpxrust" }
```

Dependencias transitivas mínimas: `cfb` (contenedor OLE2), `serde`,
`serde_json`, `thiserror`. Impacto total en el binario final: < 1 MB.

## Licencia

**LGPL-2.1-or-later.** Este crate es una obra derivada de MPXJ (LGPL 2.1),
y conserva su licencia. En la práctica:

- Una aplicación que **usa** el crate no queda cubierta por la LGPL y puede
  ser propietaria; al distribuirla debe incluir el aviso de licencia y un
  enlace al código fuente de la versión usada de este crate.
- Las **modificaciones al crate** sí deben publicarse bajo LGPL — los
  cambios se hacen en este repositorio, no en forks privados.

## Desarrollo

```bash
cargo test
```

La suite combina tests unitarios (fixtures binarios sintéticos por cada
estructura del formato) e integración contra el corpus de `tests/data/`.
Para validar contra un proyecto propio: `MPXRUST_PRIVATE_MPP=/ruta/plan.mpp
cargo test` — los tests que dependen de archivos no incluidos en el repo se
omiten automáticamente.

CI: `cargo fmt --check`, `clippy -D warnings` y la suite completa en cada
push.

## Créditos

El formato `.mpp` no está documentado por Microsoft; todo el conocimiento
del formato proviene de [MPXJ](https://github.com/joniles/mpxj), de Jon
Iles y colaboradores. Cada módulo de este crate cita el archivo Java del
que es port.
