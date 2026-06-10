# MPXRust (`mpxrust`)

Lector **en Rust puro** de archivos `.mpp` de Microsoft Project (formato
**MPP14**: Project 2010 → 365). Port del subconjunto de lectura de
[MPXJ](https://www.mpxj.org/) — sin Java, sin sidecars, sin inflar tu binario.

> Estado: **en desarrollo (H1)** — contenedor CFB, detección de versión y capa
> de bloques (Props / VarMeta / Var2Data / FixedMeta / FixedData) funcionando
> contra archivos reales. La población del modelo de tareas llega en H2–H5.
> Roadmap completo: `../docs/03-diseno-crate.md`.

## Uso

```rust
// desde disco
let project = mpxrust::read_mpp("plan.mpp")?;

// desde memoria (p. ej. bytes que llegan a un comando Tauri)
let project = mpxrust::read_mpp_bytes(&bytes)?;

// diagnóstico estructural (qué trae el archivo, sin semántica)
let summary = mpxrust::inspect_mpp("plan.mpp")?;
println!("{}", serde_json::to_string_pretty(&summary)?);
```

Errores accionables: `UnsupportedVersion` (con la versión detectada, p. ej.
"MPP12 (Project 2007)"), `PasswordProtected`, `NotACompoundFile`, `Corrupt`.

## Cómo se consume el crate

Es una dependencia Cargo común y corriente — **se compila y linkea estático
dentro de tu binario**, no hay nada que instalar ni cargar en runtime:

```toml
# mientras no esté publicado: por path (mismo disco)
[dependencies]
mpxrust = { path = "../MPXRust/MPXRust" }

# o por git
mpxrust = { git = "https://github.com/<org>/MPXRust" }

# cuando se publique en crates.io
mpxrust = "0.1"
```

Dependencias transitivas: `cfb` (contenedor OLE2), `serde`/`serde_json`,
`thiserror`. Impacto total en el binario: **menos de 1 MB**.

## Licencia: LGPL-2.1-or-later — qué significa en la práctica

Este crate es un **port (obra derivada) de MPXJ**, que es LGPL 2.1. Por eso
el crate **debe ser y es LGPL-2.1-or-later** — no es elegible para MIT/Apache.

Qué implica para una app que lo usa (p. ej. jirast):

- **Tu app NO se vuelve LGPL.** La LGPL es copyleft débil: solo cubre esta
  librería y sus modificaciones, no el programa que la consume.
- **Uso interno (no distribuyes la app a terceros): cero obligaciones.**
- **Si distribuyes la app a terceros**, debes: (1) avisar que usa mpxrust/LGPL
  e incluir el texto de la licencia, (2) dar acceso al fuente de la versión
  exacta de mpxrust usada (un link a este repo basta), y (3) permitir que el
  usuario relinke la app con una versión modificada de la librería — con
  linking estático de Rust, la vía práctica es ofrecer los objetos compilados
  de tu app o, más simple, no modificar el crate en privado: cualquier cambio
  se hace aquí, en el repo público.
- **Modificaste mpxrust**: esas modificaciones sí deben publicarse bajo LGPL.

## Tests

```bash
cargo test                      # unitarios (fixtures sintéticos) — no requieren corpus
MPXRUST_PRIVATE_MPP=/ruta/plan.mpp cargo test   # + integración con un MPP14 real
```

El corpus privado (archivos `.mpp` reales con datos de personas/proyectos)
vive en `tests/data/private/` y está **gitignoreado**: nunca se commitea.
El corpus público (archivos de prueba del repo de MPXJ) se incorpora en H2+
(ver `../docs/04-corpus-y-golden-tests.md`).

## Créditos

El conocimiento del formato `.mpp` (no documentado por Microsoft) proviene
íntegramente de [MPXJ](https://github.com/joniles/mpxj) de Jon Iles y
colaboradores — ~20 años de ingeniería inversa bajo LGPL. Cada módulo de
`src/blocks/` cita el archivo Java del que es port.
