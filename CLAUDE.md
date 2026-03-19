# Claude Code Guidelines

- Always make changes with as little code as possible, preferring to remove code when possible. 

## Code Style

### Clippy
All code must pass `cargo clippy` with no warnings. The project's `clippy.toml` and `Cargo.toml` lint settings are authoritative. Key thresholds:
- Max function arguments: **5** (`too-many-arguments-threshold = 5`)
- Max cognitive complexity: **40** (`cognitive-complexity-threshold = 40`)

### Function Arguments
- Keep functions to **≤5 parameters**. If more are needed, group related data into a struct or pass an existing component/resource by reference.
- Prefer passing structs or Bevy query results over long argument lists.
- Do not add a parameter just to thread a value through layers — restructure instead.

### Function Length
- Aim for **≤40 lines** per function body. If a function grows longer, extract a clearly-named helper.
- Each function should do one thing. Name it after what it does, not how.

### Indentation / Control Flow
- **Avoid nesting beyond 2–3 levels.** Deeply nested `if`/`match` blocks must be refactored.
- Use **early returns** (guard clauses) to eliminate else branches:
  ```rust
  // Prefer
  let Some(x) = opt else { return; };

  // Over
  if let Some(x) = opt {
      // long body
  }
  ```
- If an `else` branch is non-trivial, extract it to a named function.
- Prefer `?` for error propagation over nested `match`/`if let`.

### Naming
- Follow Rust conventions: `snake_case` for functions/variables, `PascalCase` for types, `SCREAMING_SNAKE_CASE` for constants.
- Names must be descriptive — avoid abbreviations except well-known domain terms (e.g., `pos`, `vel`, `ai`).
- Boolean variables and functions should read as predicates: `is_idle`, `has_target`, `can_gather`.

### General Cleanliness
- No dead code. Remove unused functions, imports, and variables rather than suppressing warnings with `#[allow]`.
- Prefer `#[derive]` over manual `impl` when possible.
- Use `if let`/`while let` over `.unwrap()` or `.expect()` in game logic; reserve `expect` for invariants that should truly be impossible to violate.
- Keep `match` arms exhaustive without a catch-all `_ =>` unless genuinely needed.
- Do not use `clone()` to paper over borrow checker issues — restructure ownership instead.
- Avoid `mut` where immutability is sufficient.
- Use iterators and combinators (`map`, `filter`, `any`, `find`) over manual loops where it improves clarity.

### Bevy-Specific
- Systems should be small and focused. Split a system into helpers if it handles more than one logical concern.
- Use components/resources to share state; do not pass Bevy `World` or `Commands` deeper than one level into a call stack.
- **Named query structs (required):** Complex `Query` types must be defined as named `#[derive(SystemParam)]` structs outside of function arguments, not inline. A query is "complex" if it has a filter (`With<T>`, `Without<T>`, etc.) or fetches more than two components. Name the struct after what it represents, not what it queries (e.g., `IdleWorkers`, not `WorkerQuery`). Give the struct the minimum visibility needed to satisfy the compiler (`pub(crate)` for public systems).

### Event-Driven Architecture (Required)
Systems must communicate exclusively through Bevy events. Direct mutation of another system's components is prohibited.

**Rules:**
- Every cross-system state change must go through an `Event`. No system may write to a component it does not own.
- Each component has exactly one owning system responsible for writing it. All other systems are read-only on that component.
- Document ownership in `components.rs` with a comment: `// Owned by: FooSystem` above each component.
- Use `EventWriter<T>` to signal intent; use `EventReader<T>` to apply changes in the owning system.
- Component addition/removal via `Commands` is the correct way for a system to signal a state transition to other systems (e.g. inserting `HoveredEntity`, removing `Selected`).
- Do not use mutable flag fields (e.g. `is_dirty: bool`, `needs_update: bool`) as implicit message passing — use events instead.
- Do not use `unsafe` static state for inter-frame data; use `Local<T>` resources instead.

**Example — correct pattern:**
```rust
// Input system fires event, does NOT touch Selectable
events.send(SelectionChangedEvent { entity, is_selected: true });

// Selection system owns Selectable, applies changes
fn apply_selection(mut events: EventReader<SelectionChangedEvent>, mut q: Query<&mut Selectable>) { ... }
```
