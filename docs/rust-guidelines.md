# Rust Guidelines

These are design preferences, not a checklist to apply mechanically. Follow the existing code when it is clearer or more consistent.

## Interfaces first

- For non-trivial work, the plan should identify the modules and types involved, the behavior each owns, and the intended public or internal interface.
- Keep interfaces small and deliberate. Start private; widen visibility only when another module or crate genuinely needs access.
- Treat `pub(crate)` as an internal sharing mechanism and `pub` as an intentional external API.

## Put behavior on its owner

- Prefer methods and associated functions on the struct, enum, or domain type that owns the data or concept.
- Use free functions for utilities, stateless operations, or behavior that naturally combines multiple unrelated types.
- Do not create a free function merely because it is convenient when the behavior clearly belongs to a type.

## Modules and types

- Prefer small, focused modules organized by domain or concept.
- Split modules when a boundary improves ownership, discoverability, or the public interface; avoid splitting code without a meaningful boundary.
- Prefer `Type::new(...)` when construction has meaningful semantics, defaults, validation, or multiple fields.
- Struct literals are fine for small data/config structs and simple test values.
- Introduce a domain-specific newtype or enum only when it provides clear value by preventing confusion, enforcing an invariant, or clarifying an interface.

## Comments and documentation

- Keep comments short and useful. Explain why, constraints, or non-obvious behavior—not what obvious code already says.
- Avoid long narrative comments. We can read the code.
- Document public interfaces when their purpose, contract, invariants, errors, or side effects are not obvious. Private implementation details usually do not need comments.

## Refactoring

- Nearby refactoring is welcome when it improves type ownership, module boundaries, or clarity.
- Keep unrelated cleanup out of the change, and rely on the repository's CI and test gate to validate structural refactors.
