# RustGuard Anti-Slop Constitution

## Rules

1. **All errors must be handled or propagated** — use `?` + `thiserror` or `anyhow`. Never silence with `let _ =` or `.ok()`.
2. **No panics in library code** — `unwrap()`, `expect()`, `panic!()` banned outside `#[cfg(test)]`.
3. **Tests must test real behavior** — no mock-echo tests, no `assert!(result.is_ok())` without variant checks.
4. **No `dbg!`, `todo!`, `unimplemented!()`** in non-test code.
5. **Prefer `&str`/`&[T]` over owned types** unless ownership is genuinely needed.
6. **Zero `unsafe`** unless accompanied by a `// SAFETY:` comment explaining the invariant.
7. **No wildcard imports** — `use foo::*` is banned.
8. **No excessive `.clone()`** — prefer borrowing. Every `clone()` must justify itself.
9. **No placeholder names** — `foo`, `bar`, `baz`, `temp` are rejected by lints.
10. **No indexing without bounds check** — use `.get()` or iterators, never `[i]`.

## Enforcement

- `cargo clippy --all-targets --all-features -- -D warnings` — must pass in CI.
- `cargo fmt -- --check` — must pass in CI.
- Workspace `Cargo.toml` defines `[workspace.lints.clippy]` with all rules.
- Individual crates opt in via `[lints] workspace = true`.

## Test Exceptions

`unwrap()`, `expect()`, `dbg!()`, `todo!()` are allowed inside `#[cfg(test)]` modules for rapid iteration. The `#![cfg_attr(not(test), deny(...))]` gate enforces this automatically.
