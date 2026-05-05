// Stub: macro expansion logic.
//
// Real implementation (see docs/plan.md Lab 4) takes a parsed (PathBuf, Ident),
// loads the ModelSpec via spec_loader, computes the steady-state gain,
// and emits a struct + update() body that's straight-line arithmetic with
// f32 literals (use proc_macro2::Literal::f32_suffixed for precision).
// Must NOT emit any `use nalgebra::*` so the embedded crate stays light.
