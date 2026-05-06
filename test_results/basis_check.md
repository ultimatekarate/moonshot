# Basis architectural check

**Date:** 2026-05-06
**Command:** `basis-cli check --spec basis.yaml .`
**Source:** [basis.yaml](../basis.yaml)

## Result

```
basis check passed.
```

Zero violations across all five governance axes (placement, values, completeness, purity, granularity).

## What this proves

- ✓ No `use` statement in any source file crosses a layer boundary that's denied by [basis.yaml](../basis.yaml). Specifically: `kernel → embassy` deny rule holds (pure math doesn't pull in async runtime), `codegen → embassy` deny rule holds (proc-macros run on the host), `autodiff → embassy` deny rule holds (gradient math is host-side only), `kernel → proc-macro-utils` deny rule holds (runtime kernel doesn't depend on macro infrastructure).
- ✓ Every layer's purity rule (`strict` for spec/kernel/autodiff, `relaxed` for codegen/shell) is intact — no IO operations leaked into strict-purity layers.
- ✓ Every variant of every union type with `exhaustive_matching: true` is handled at every consumer.
- ✓ Every newtype is used in the parameter slots that require it.
- ✓ No file exceeds the granularity cap (1009 lines).

## Layer-name aliases needed in basis.yaml

basis-cli matches `use` paths as literal strings against each layer's `packages:` list. Rust converts hyphens in Cargo crate names to underscores in `use` paths, so the spec has to list both forms. Examples:

- `linalg` layer needs both `num-dual` (Cargo name) and `num_dual` (Rust import path).
- `proc-macro-utils` layer needs both `proc-macro2` and `proc_macro2`.
- `rust-internal` layer needs `proc_macro` (compiler-provided crate, no Cargo entry).
- Internal layers (`spec`, `kernel`, `codegen`, etc.) need both `crates/foo` (source-tree path) and `foo` (crate name as it appears in `use foo::...`).
- Same-crate submodule declarations (`mod expand;`, `mod parse;`) read by basis-cli as imports; listed under the `codegen` layer to prevent spurious all-external fallback matches.

These aren't architectural concessions — they're a workaround for basis-cli matching imports as literal strings without language-aware normalization. Documented in [basis.yaml](../basis.yaml) inline.

## What this does not prove

- ✗ basis is a string-matching tool, not a semantic analyzer. It catches layer-crossing imports and missing match arms. It does not catch a function that accepts a `Timestep` newtype and treats it as `f32` internally, or a side effect hidden behind three layers of indirection. See the [basis README](https://github.com/ultimatekarate/basis) for the explicit list of what's in scope.
- ✗ Architectural rules are only as good as the rules in `basis.yaml`. If a deny edge is missing from the spec, basis won't catch traffic across it.

## Reproducing this

```bash
just basis
# or directly:
basis-cli check --spec basis.yaml .
```
