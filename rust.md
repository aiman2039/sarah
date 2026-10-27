# Rust Policy

## Structs

- Default to public fields and methods.
- When a type has invariants to protect, use private
  fields                                                                                                                                             
  with constructors/builders and accessor methods.

## Functions

- Default to private for non-method functions; make public only when they are API.
- If there is more than one argument of the same type, use an input struct
- If there are more than three arguments, use an input struct
- Always fully destruct the input struct, to ensure we always use all of the fields inside the method

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
uninlined_format_args = "allow"
return_self_not_must_use = "allow"
cast_possible_truncation = "allow"
cast_sign_loss = "allow"
cast_precision_loss = "allow"
cast_possible_wrap = "allow"
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

# Prohibit bare allow attributes - require reasons
allow_attributes_without_reason = "deny"

# Allow large error types - error types need rich context for good UX
result_large_err = "allow"

# No debug macros in production code
dbg_macro = "deny"
# No incomplete code markers
todo = "deny"
unimplemented = "deny"
# Use tracing instead of println/eprintln in library code
print_stdout = "deny"
print_stderr = "deny"
```

- For single-crate projects, use `[lints.clippy]` (no workspace table) and do not set `lints.workspace = true`.
- In workspaces, each member crate should inherit via:

  ```toml
  [lints]
  workspace = true
  ```

- Optionally scope production-only denies to non-test code in `lib.rs`:

  ```rust
  #![cfg_attr(not(test), deny(
      clippy::unwrap_used,
      clippy::expect_used,
      clippy::dbg_macro,
      clippy::print_stdout,
      clippy::print_stderr,
      clippy::panic,
  ))]
  ```

## Format Strings

- Always inline variables in format strings
- Bad: `format!("{}", var)`, `println!("{:?}", var)`
- Good: `format!("{var}")`, `println!("{var:?}")`
- For width/precision: use `format!("{var:width$}")` not `format!("{0:1$}", var, width)`

## Helpers (stateless utilities)

- Make folders per topic; use modules as namespaces.
- Prefer using a struct with static methods
- The helper name is helpful for namespacing for example CircuitHelpers
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
- Reserve a `source` field for real error sources (`impl std::error::Error`) and use `#[source]`/`#[from]` as
  appropriate. For string context, use fields like `reason` or `details` instead of `source`.
- Each error variant should carry enough context to debug without looking at logs.

## Lifetimes

- Don't write explicit lifetimes when they can be elided — use `&Bundle<'_>` not `&Bundle<'a>` when `'a` appears only
  once in the signature
- When a method borrows from both `&self` and a parameter to produce output, unify under one lifetime:
  `fn method<'a>(&'a self, input: Input<'a>) -> Output<'a>`

## Module Organization

- One directory per topic (`config/`, `client/`, `bundler/`), each with a `mod.rs` that re-exports submodules
- Large `impl` blocks can be split across multiple files in the same directory (e.g., `impl JitoBundler` in `send.rs`,
  `simulate.rs`, `status.rs`)
