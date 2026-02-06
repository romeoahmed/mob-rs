# mob-rs Guidelines

> MO2 Build Tool - Rust + Tokio, 360+ tests

## Critical Rules

| Rule            | Forbidden                      | Required                           |
| --------------- | ------------------------------ | ---------------------------------- |
| Zero Panic      | `.unwrap()`, `.expect()`       | `.with_context(\|\| format!(...))` |
| Determinism     | `HashMap`                      | `BTreeMap`, `IndexMap`             |
| No Placeholders | `todo!()`, `println!("debug")` | Functional code only               |
| No Reinventing  | Custom glob/http/walk          | Use existing crates                |

## API Design

| Rule              | Forbidden                       | Required                        | Rationale                                                                                                                                                            |
| ----------------- | ------------------------------- | ------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| No Re-exports     | `pub use` in `mod.rs`           | Import from exact source module | Re-exports hide ownership, cause version conflicts, break semver ([RFC 3516](https://rust-lang.github.io/rfcs/3516-public-private-dependencies.html))                |
| Honest Signatures | Infallible fn → `Result<T, E>`  | Return `T` directly             | `Result` signals fallibility; always-Ok misleads callers ([clippy::unnecessary_wraps](https://rust-lang.github.io/rust-clippy/master/index.html#/unnecessary_wraps)) |
| Honest Signatures | Always-present fn → `Option<T>` | Return `T` directly             | `Option` signals absence; always-Some violates semantics                                                                                                             |
| Self-Documenting  | `fn check() -> bool`            | `fn find() -> Result<T, E>`     | Expose intermediate data; rich types > boolean ([C-INTERMEDIATE](https://rust-lang.github.io/api-guidelines/flexibility.html#c-intermediate))                        |
| No Suppressions   | `#[allow(...)]`                 | Fix the underlying issue        | Suppressions hide problems; Clippy warnings exist for good reasons                                                                                                   |

## Principles

- **Data > Logic**: Enums > boolean flags
- **Explicit > Implicit**: `.with_context()` > bare `?`, named imports > `*`
- **Private by Default**: Start private, widen only when needed

## Opaque Types Pattern

Structs should hide internal representation, exposing only behavior through methods.

| Rule           | Forbidden                        | Required                                 |
| -------------- | -------------------------------- | ---------------------------------------- |
| Private Fields | `pub field: T`                   | `field: T` (no `pub`)                    |
| Getters        | Direct field access              | `fn field(&self) -> &T`                  |
| Builders       | Field assignment                 | `fn with_field(self, val: T) -> Self`    |
| Name Conflicts | Same name for getter and builder | `field()` getter, `with_field()` builder |

### Exceptions

- **Serde/Clap structs**: Fields derived with `#[derive(Deserialize, Serialize)]` or `#[derive(Args)]` may remain `pub` for macro compatibility
- **FFI structs**: C-compatible structs may need `pub` fields
- **Simple data carriers**: Small `Copy` types used only internally

### Pattern

```rust
// ❌ BEFORE: Exposed internals
pub struct TaskContext {
    pub config: Arc<Config>,
    pub dry_run: bool,
}

// ✅ AFTER: Opaque type
pub struct TaskContext {
    config: Arc<Config>,   // private
    dry_run: bool,         // private
}

impl TaskContext {
    // Constructor
    pub fn new(config: Arc<Config>) -> Self {
        Self { config, dry_run: false }
    }

    // Builder (consumes self, returns Self)
    #[must_use]
    pub fn with_dry_run(self, val: bool) -> Self {
        Self { dry_run: val, ..self }
    }

    // Getter (borrows self)
    pub fn config(&self) -> &Arc<Config> {
        &self.config
    }

    pub fn is_dry_run(&self) -> bool {
        self.dry_run
    }
}
```

### Naming Conventions

| Type    | Pattern                   | Example                           |
| ------- | ------------------------- | --------------------------------- |
| Getter  | `field()` or `is_field()` | `config()`, `is_dry_run()`        |
| Builder | `with_field()`            | `with_config()`, `with_dry_run()` |
| Mutator | `set_field()`             | `set_config()` (if needed)        |

## Visibility

| Modifier     | Scope             | Use                   |
| ------------ | ----------------- | --------------------- |
| _(none)_     | Module + children | **Default**           |
| `pub(crate)` | Crate-wide        | Cross-module helpers  |
| `pub(super)` | Parent module     | Module family         |
| `pub`        | Public API        | `lib.rs` exports only |

## Dependencies

| Layer    | Crates                                         |
| -------- | ---------------------------------------------- |
| Error    | `anyhow` (app), `thiserror` (lib)              |
| CLI      | `clap`, `config`, `serde`, `toml`              |
| Async    | `tokio`, `tokio-util`, `futures-util`, `flume` |
| HTTP/Git | `reqwest`, `gix`                               |
| FS       | `ignore`, `tempfile`, `which`                  |
| Windows  | `windows` crate                                |
| Builder  | `bon`                                          |
| Test     | `nextest`, `insta`                             |

## Builder Generation (`bon`)

Use `bon` **selectively** for pure config/params/options structs. Keep hand-written builders for behavior-heavy fluent APIs.

### When to Use `bon`

| Use `bon`                          | Keep Manual                             |
| ---------------------------------- | --------------------------------------- |
| Pure config/options structs        | Behavior-heavy fluent APIs              |
| Many optional fields with defaults | Mixed builder + `&mut self` patterns    |
| "Required at runtime" → compile    | Fallible builder steps (`Result<Self>`) |
| Boilerplate-heavy `with_*` chains  | Complex domain logic in builder methods |

### Rules

| Rule               | Required                                           | Rationale                              |
| ------------------ | -------------------------------------------------- | -------------------------------------- |
| Setter naming      | `#[builder(setters(name = with_field))]` per field | Match `with_field()` project naming    |
| Required fields    | Non-`Option` types                                 | Compile-time enforcement via typestate |
| Explicit defaults  | `#[builder(default = expr)]`                       | No hidden `Default::default()` magic   |
| No global `into`   | Use `#[builder(into)]` per-field only              | Avoid type inference breakage          |
| Builder visibility | `pub(crate)` unless intentionally public           | Avoid semver commitment on internals   |
| Canonical derive   | `Debug, Clone, ..., Builder` (bon derives last)    | Consistent ordering                    |
| Default impl       | Delegate `Default` to `Self::builder().build()`    | Single source of truth for defaults    |

### Bon Example

```rust
use bon::Builder;

#[derive(Debug, Clone, Builder)]
pub struct WalkOptions {
    #[builder(setters(name = with_max_depth))]
    max_depth: Option<usize>,

    #[builder(setters(name = with_follow_links), default = false)]
    follow_links: bool,

    #[builder(setters(name = with_include_hidden), default = false)]
    include_hidden: bool,
}

impl Default for WalkOptions {
    fn default() -> Self {
        Self::builder().build()
    }
}

// Usage: WalkOptions::builder().with_max_depth(5).with_follow_links(true).build()
```

### Skip List (keep manual)

- `ProcessBuilder` — behavior-heavy fluent API (~19 methods)
- `ConfigLoader` — fallible multi-source assembly
- `TaskManager` — mixed builder + `&mut self` collection
- `Env` — COW `&mut self` mutation pattern
- `Downloader` — append semantics (`header()`), internal state, convenience methods

## Patterns

| Do                                  | Don't                         |
| ----------------------------------- | ----------------------------- |
| `tokio::spawn(io)`                  | `std::thread::sleep` in async |
| `spawn_blocking(cpu)`               | MutexGuard across `.await`    |
| `.with_context(\|\| ...)`           | `format!()` in `.context()`   |
| Enums for states                    | Boolean flags                 |
| `let Some(x) = opt else { return }` | Nested if-let                 |
| `flume::bounded::<T>(N)`            | Unbounded channels            |
| `gix::open_opts(...isolated())`     | Default gix options           |
| `repo.into_sync()`                  | Sharing non-sync repo         |

## Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum MyError {
    #[error("failed: {0}")]
    Failed(#[source] std::io::Error),
}
```

## Drop Rules

- Never panic/`.unwrap()` in drop
- Never `.await` in drop
- Ignore cleanup errors: `let _ = cleanup();`
- Use RAII guards for scope cleanup

## Testing

| Type      | Tool                          |
| --------- | ----------------------------- |
| Unit      | `cargo nextest run`           |
| Snapshot  | `insta` (**prefer**)          |
| Isolation | `tempfile`                    |
| Lint      | `cargo clippy -- -D warnings` |

**Structure**: `mod.rs` + separate `tests.rs` per module.

## ASCII Diagrams

Use only when: >3 entities, state machines, memory layout.
Chars: `+-|<>^v*/` preferred. Max 70w/25h in `///` docs.

## Commands

| Cmd         | Action                        |
| ----------- | ----------------------------- |
| `/build`    | `cargo build`                 |
| `/test`     | `cargo nextest run`           |
| `/lint`     | `cargo clippy -- -D warnings` |
| `/snapshot` | `cargo insta review`          |
