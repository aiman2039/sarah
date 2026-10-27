# Rust Policy

## Structs

- Default to public fields and methods.
- When a type has invariants to protect, use private fields with constructors/builders and accessor methods.

## Functions

- Default to private for non-method functions; make public only when they are API.
- If there are more than three arguments, use an input struct
- If two or more arguments share a type and swapping them would silently change behavior, use an input struct.
  Symmetric arguments (`max(a, b)`) are exempt.
- Always fully destructure input structs (the ones created for a function's arguments), so every field is used.
  This does not apply to other structs a function reads.

## Newtypes

- Domain values whose meaning goes beyond their representation get a newtype so they can't be mixed up:
  identifiers (`UserId(u64)` vs `OrderId(u64)`), units and amounts (`Lamports(u64)`, `Tokens(usize)`, `Millis(u64)`),
  and validated data (`Email(String)`).
- When the value has invariants, use a private field with a checked constructor (see Structs).
- Use `derive_more` for the boilerplate (`From`, `Into`, `Display`, arithmetic) instead of writing impls by hand.
- Plain primitives stay plain where nothing can be mixed up (loop indices, short-lived locals, symmetric arguments).

## Format

- Use `cargo fmt`

## Lint

- Use `cargo clippy`
- Every Rust project MUST have pedantic clippy lints enabled in `Cargo.toml`
- For workspace projects, add to the workspace root `Cargo.toml`:

```toml
[workspace.lints.clippy]
pedantic = { level = "warn", priority = -1 }

# Pedantic overrides - too noisy or not applicable
missing_errors_doc = "allow"
missing_panics_doc = "allow"
must_use_candidate = "allow"
doc_markdown = "allow"
similar_names = "allow"
too_many_lines = "allow"
module_name_repetitions = "allow"
struct_excessive_bools = "allow"
fn_params_excessive_bools = "allow"
items_after_statements = "allow"
return_self_not_must_use = "allow"
cast_precision_loss = "allow"
default_trait_access = "allow"
match_same_arms = "allow"
unused_async = "allow"
manual_let_else = "allow"
used_underscore_binding = "allow"
ref_option_ref = "allow"
ref_option = "allow"
format_push_string = "allow"
case_sensitive_file_extension_comparisons = "allow"
unnecessary_wraps = "allow"
needless_pass_by_value = "allow"
trivially_copy_pass_by_ref = "allow"
no_effect_underscore_binding = "allow"
redundant_closure_for_method_calls = "allow"
wildcard_imports = "allow"

# Panic safety - deny operations that could panic
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
# Out-of-bounds indexing/slicing: use .get(..) and handle the None
indexing_slicing = "deny"
# Slicing a str inside a multi-byte character: use str::get(..)
string_slice = "deny"
# Integer overflow (panics in debug, wraps in release): use checked_*/saturating_*
arithmetic_side_effects = "deny"
# Silent value corruption: cast_possible_truncation, cast_sign_loss and
# cast_possible_wrap stay at pedantic's warn (denied by -D warnings); use T::try_from(x)?

# Prohibit allow attributes: use #[expect(lint, reason = "...")], which errors once the
# lint stops firing, so suppressions can't go stale. Reasons are mandatory.
# Platform-specific lints: #[cfg_attr(target_os = "...", expect(lint, reason = "..."))]
allow_attributes = "deny"
allow_attributes_without_reason = "deny"

# Enforce the Format Strings rule: `format!("{var}")`, never `format!("{}", var)`
uninlined_format_args = "deny"

# Allow large error types - error types need rich context for good UX
result_large_err = "allow"

# No debug macros in production code
dbg_macro = "deny"
# No incomplete code markers
todo = "deny"
unimplemented = "deny"
# No println!/eprintln!. Libraries return data or log with tracing. A binary routes
# user-facing output through one module marked
# #[expect(clippy::print_stdout, reason = "...")]; diagnostics go through tracing.
print_stdout = "deny"
print_stderr = "deny"

# Module roots are topic/mod.rs, never topic.rs + topic/ (see Module Organization)
self_named_module_files = "deny"
```

- For single-crate projects, use `[lints.clippy]` (no workspace table) and do not set `lints.workspace = true`.
- Tests may index fixtures and unwrap. Allow that in `clippy.toml` at the project root:

  ```toml
  allow-indexing-slicing-in-tests = true
  allow-unwrap-in-tests = true
  allow-expect-in-tests = true
  allow-panic-in-tests = true
  ```

  Arithmetic and string slicing in tests have no such switch; use checked/`get` there too.
- In workspaces, each member crate should inherit via:

  ```toml
  [lints]
  workspace = true
  ```

## Format Strings

- Always inline variables in format strings
- Bad: `format!("{}", var)`, `println!("{:?}", var)`
- Good: `format!("{var}")`, `println!("{var:?}")`
- For width/precision: use `format!("{var:width$}")` not `format!("{0:1$}", var, width)`

## Helpers (stateless utilities)

- Make folders per topic; modules organize the files.
- Inside the module, put stateless helpers on a struct with static methods (e.g. `CircuitHelpers::validate`).
  A free function can be imported bare (`use circuit::validate;`), which hides where it comes from at the call site;
  an associated function always needs the struct prefix, so every call stays self-describing and greppable.
- This pattern is for stateless utilities only (pure functions, no owned state).
- If a type owns resources (clients, connections, config), use instance methods (`&self`) on that type rather than
  passing owned resources to static methods.

## Method Structure

- Flow methods (orchestrators) are flat, linear sequences of clearly named steps — no inline logic or deep nesting. A
  reader should grok the whole flow at a glance.
- Extract each step into a small private method with a descriptive name. Take `&mut self` only when mutating; otherwise
  prefer `&self` and return values.
- Use `?` and early returns for validation and error propagation; add context at the error site. Keep the happy path
  flat.
- Hide branching inside step methods so the orchestrator reads linearly. Keep concurrency/spawn/join decisions at the
  orchestrator boundary when it aids clarity.
- Validate inputs first and outputs last.
- Separate mutation from computation: finish `&mut self` writes before `&self` reads. Compute into locals, then assign
  once. This avoids borrow conflicts and clones.
- Optional: instrument orchestrators with `#[tracing::instrument]` for visibility; keep leaf steps mostly quiet.

Example (generic flow)

```rust
impl Service {
    pub fn run(&mut self, req: Request) -> Result<Response, Error> {
        self.validate_input(&req)?;          // read
        let plan = self.plan(&req)?;         // pure, &self
        self.apply_plan(&plan)?;             // side effects, &mut self
        let resp = self.render(&plan)?;      // pure, &self
        self.validate_output(&resp)?;        // read
        Ok(resp)
    }

    fn validate_input(&self, req: &Request) -> Result<(), Error> { /* ... */ }
    fn plan(&self, req: &Request) -> Result<Plan, Error> { /* ... */ }
    fn apply_plan(&mut self, plan: &Plan) -> Result<(), Error> { /* ... */ }
    fn render(&self, plan: &Plan) -> Result<Response, Error> { /* ... */ }
    fn validate_output(&self, resp: &Response) -> Result<(), Error> { /* ... */ }
}
```

Example (one-shot builder as flow)

```rust
impl Builder {
    pub fn build(self) -> Result<Product, Error> {
        let cfg = self.validate()?;           // pure
        let deps = Self::init_deps(&cfg)?;    // side effects
        let product = Product::new(cfg, deps); // pure
        Ok(product)
    }
}
```

## Code Hygiene

- Never write speculative code — only write functions that have a caller
- If a function is removed, remove its input struct too
- Prefer structural solutions over `.clone()` workarounds for borrow checker issues (e.g., collect into a local vec then
  assign, instead of cloning inside a loop)

## Errors

- Use `thiserror` for all error enums.
- Library code returns `thiserror` enums so callers can match on the failure.
- `anyhow` is allowed only in binaries, at the top level (`main`, command handlers), for context and reporting.
- Enforce it in library crates with clippy `disallowed_types` (`disallowed_types = "deny"` in `[lints.clippy]`)
  and in that crate's `clippy.toml`:

  ```toml
  disallowed-types = [
      { path = "anyhow::Error", reason = "library errors must be thiserror enums" },
      { path = "anyhow::Result", reason = "library errors must be thiserror enums" },
  ]
  ```
- Reserve a `source` field for real error sources (`impl std::error::Error`) and use `#[source]`/`#[from]` as
  appropriate. For string context, use fields like `reason` or `details` instead of `source`.
- Each error variant should carry enough context to debug without looking at logs.

## Lifetimes

- Don't write explicit lifetimes when they can be elided — use `&Bundle<'_>` not `&Bundle<'a>` when `'a` appears only
  once in the signature
- When a method borrows from both `&self` and a parameter to produce output, unify under one lifetime:
  `fn method<'a>(&'a self, input: Input<'a>) -> Output<'a>`

## Module Organization

- Directories mirror the architecture: one directory per topic (`config/`, `client/`, `bundler/`), so `src/` stays
  a small set of topics instead of a flat list of files. A topic directory holds as many files as it needs.
- Each topic directory has a `mod.rs` that declares and re-exports its submodules (enforced by
  `self_named_module_files = "deny"`, which forbids the `topic.rs` + `topic/` style).
- Large `impl` blocks can be split across multiple files in the same directory (e.g., `impl JitoBundler` in `send.rs`,
  `simulate.rs`, `status.rs`)
