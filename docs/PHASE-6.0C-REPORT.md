# Phase 6.0c — Surface finalization

**Base:** `4e193903cc202a6f9fa7187847c3c25df2caaf6d` (`phase-6.0b-extensible-surfaces`)

## Final corrections

The Entity Surface foundation now uses a generic `SurfaceLoadResult<T, Context>` containing only an entity and optional descriptor-owned context. Quest attachment data and Concept relationship data remain in their entity-specific loaders. The registry uses typed `defineSurface` registration boundaries, so the generic provider does not depend on Concept, Quest, Effect, or other concrete entity data.

Unsupported kinds are explicit. If `openSurface('skill', id)` is requested before a Skill descriptor exists, the surface shows `Surface not available`, identifies the requested kind, explains that the type has no contextual viewer, and provides Close plus Back when nested. It never invokes the Effect, Quest, or any other unrelated loader.

The extensibility test now registers a real future-note descriptor, opens it through the actual provider, verifies descriptor-owned context and rendered content, opens a nested child, uses Back, and closes the stack. The supported Player, Quest, Concept, and Effect tests remain intact.

## Validation

- `npm run typecheck` — passed.
- `npm test -- --run` — passed: 7 files, 53 tests, including 7 surface tests with unsupported and future-kind end-to-end coverage.
- `npm run build:vite` — passed.
- `git diff --check` — passed.
- No `lint` or `format` script is configured in `package.json`.
- `cargo` and `rustc` are unavailable in this sandbox, so Rust/native validation was not run and is not claimed.

No Phase 6.1 product work, route redesign, global styling redesign, or unrelated manager/page work is included.
