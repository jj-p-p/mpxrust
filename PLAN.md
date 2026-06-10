# PLAN — mpxrust (plan vivo)

> Crate F6 del plan maestro de jirast. Diseño completo en `../docs/03-diseno-crate.md`;
> estrategia de validación en `../docs/04-corpus-y-golden-tests.md`.
> Convención: un hito se cierra solo con `cargo test` verde y sin regresiones.

## Estado por hito

| Hito | Entrega | Estado |
|---|---|---|
| **H0** | Spike: cfb + VarMeta/Var2Data a mano, 90/90 nombres del plan_anco | ✅ 2026-06-10 (`../spike/`) |
| **H1** | Crate + L1 bloques (Props/VarMeta/Var2Data/FixedMeta/FixedData) + CompObj/versión + tests | ✅ 2026-06-10 — 23 tests verdes, integración contra plan_anco.mpp |
| **H2** | FieldMap14 dinámico desde Props (var keys por archivo) | ✅ 2026-06-10 — `field_map.rs`, entradas de 28 bytes, bloques 0/1 |
| **H3** | Tareas: nombres, jerarquía, fechas (regla scheduled→actual), hitos (bits por versión), % | ✅ 2026-06-10 — `reader/tasks.rs` con createTaskMap completo |
| **H4** | Dependencias FS/SS/FF/SF + lag (offsets por versión); duration_days/work_hours | ✅ 2026-06-10 — `reader/relations.rs`, validado vs mpp14relations/duration |
| **H5** | Recursos + asignaciones → **paridad con el JSON del analizador: 90/90 issues, 85/85 deps, 0 diffs** | ✅ 2026-06-10 — `tests/parity_plan_anco.rs` + corpus público (8 archivos, Project 2010 y 2013) |
| **H6** | Integración jirast: comando `import_load_mpp` (drag & drop del .mpp) | ⬜ |
| **H7** | Publicación: crates.io (`mpxrust` — verificado libre 2026-06-10), GitHub, README EN | ⬜ |

## Decisiones tomadas

- **Nombre**: paquete `mpxrust` (en crates.io `mpxrs` está squatteado; `mpxrust` libre). Marca/repo: MPXRust.
- **Licencia**: LGPL-2.1-or-later (obra derivada de MPXJ; mismo "or later" que los headers de MPXJ).
- **Solo MPP14**; otras versiones → `MppError::UnsupportedVersion` con nombre legible.
- Port **fiel** de MPXJ en L1–L3 (mismos hacks y tolerancias a corrupción, citando el original); modelo público propio y lean en L4.
- Corpus privado gitignoreado (`tests/data/private/`); el corpus público de MPXJ entra en H2+.
- Oráculo MPXJ (Java) solo dev-time, en `tools/oracle/` (pendiente, H3).

## Hallazgos y desviaciones documentadas (H2–H5)

- **Password (R4) resuelto como MPXJ**: `Props14` raíz → `PASSWORD_FLAG & 0x1`
  + hash presente = `PasswordProtected` (ni MPXJ sabe descifrar lectura).
  La ofuscación XOR de archivos con password de escritura SÍ está implementada
  (`stream_decrypted`, aplica a Props/recursos/asignaciones/relaciones).
- **MPP14 remapea índices**: 29/35/36 = SCHEDULED_DURATION/START/FINISH;
  START/FINISH "reales" en 1283/1284 (solo tareas manuales). Regla portada.
- **Tablas default de FieldMap14 NO portadas**: todos los archivos reales
  traen el mapa serializado en Props; si faltara → error claro. (Port mecánico
  pendiente si el corpus lo exige.)
- **maxFixedDataSize subestimado**: solo conocemos el tamaño de los campos
  portados → FixedData sin clamp (max=0) y heurística 75% con cota inferior.
  Idéntico resultado en corpus sano; revisar si aparece un archivo corrupto.
- **Unidades elapsed**: `duration_days` normaliza a días laborales; un
  "1 elapsed day" (24h) sale como 3.0 días de 8h. Decisión consciente
  (el modelo expone una sola moneda); revisar si jirast necesita el matiz.
- **duration_days conserva decimales** (0.5 días); el analizador del
  compañero truncaba a entero. mpxrust es más fiel; test compara truncado.
- **El analizador filtraba niveles 0–2** (raíz/proyecto/fases) y emitía 90
  issues; mpxrust entrega TODAS las tareas — el filtro es política de jirast.

## Pendientes (H6+)

- Comando `import_load_mpp` en jirast (`read_mpp_bytes` + `to_jirast_json`).
- Calendarios con excepciones (hoy: `minutes_per_day` de Props, suficiente
  para paridad); notas RTF (se omiten); custom fields; baselines.
- Oráculo Java (`tools/oracle/`) para regenerar goldens al ampliar el subset
  — hoy el golden es el JSON del analizador + valores conocidos del corpus
  público de MPXJ.
