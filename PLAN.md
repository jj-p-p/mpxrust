# PLAN — mpxrust (plan vivo)

> Crate F6 del plan maestro de jirast. Diseño completo en `../docs/03-diseno-crate.md`;
> estrategia de validación en `../docs/04-corpus-y-golden-tests.md`.
> Convención: un hito se cierra solo con `cargo test` verde y sin regresiones.

## Estado por hito

| Hito | Entrega | Estado |
|---|---|---|
| **H0** | Spike: cfb + VarMeta/Var2Data a mano, 90/90 nombres del plan_anco | ✅ 2026-06-10 (`../spike/`) |
| **H1** | Crate + L1 bloques (Props/VarMeta/Var2Data/FixedMeta/FixedData) + CompObj/versión + tests | ✅ 2026-06-10 — 23 tests verdes, integración contra plan_anco.mpp |
| **H2** | FieldMap14 dinámico desde Props (`getMaxFixedDataSize`, var keys por archivo) | ⬜ |
| **H3** | MVP tareas: nombres, jerarquía (outline), fechas, hitos, % — golden plan_anco (subset) | ⬜ |
| **H4** | Dependencias FS/SS/FF/SF + lag; calendario estándar; duration_days/work_hours | ⬜ |
| **H5** | Recursos + asignaciones + notas → paridad TOTAL con el JSON del analizador | ⬜ |
| **H6** | Integración jirast: comando `import_load_mpp` (drag & drop del .mpp) | ⬜ |
| **H7** | Publicación: crates.io (`mpxrust` — verificado libre 2026-06-10), GitHub, README EN | ⬜ |

## Decisiones tomadas

- **Nombre**: paquete `mpxrust` (en crates.io `mpxrs` está squatteado; `mpxrust` libre). Marca/repo: MPXRust.
- **Licencia**: LGPL-2.1-or-later (obra derivada de MPXJ; mismo "or later" que los headers de MPXJ).
- **Solo MPP14**; otras versiones → `MppError::UnsupportedVersion` con nombre legible.
- Port **fiel** de MPXJ en L1–L3 (mismos hacks y tolerancias a corrupción, citando el original); modelo público propio y lean en L4.
- Corpus privado gitignoreado (`tests/data/private/`); el corpus público de MPXJ entra en H2+.
- Oráculo MPXJ (Java) solo dev-time, en `tools/oracle/` (pendiente, H3).

## Pendientes / notas para el próximo hito (H2)

- `FieldMap14.createTaskFieldMap(props)`: los offsets salen del Props del
  proyecto (claves `TASK_FIELD_MAP*`); las tablas estáticas de FieldMap14.java
  (140 KB) son port mecánico — evaluar generarlas con un script desde el Java.
- `MppContainer::block_set` ya recibe `max_fixed_item_size`: conectarlo a
  `fieldMap.getMaxFixedDataSize(0)` cuando exista FieldMap.
- Detección de password (R4): localizar la clave en Props14 y mapear a
  `MppError::PasswordProtected` (hoy un archivo protegido caerá en `Corrupt`).
- `Fixed2Meta/Fixed2Data` de tareas (heurística 92–96 ya implementada en
  `FixedMeta::parse_with_candidate_sizes`) se consumen recién en H3.
