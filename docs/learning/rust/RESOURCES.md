# Rust resources

## Knowledge

Official sources checked during setup on 2026-10-06.

- [Rust Book: defining enums](https://doc.rust-lang.org/book/ch06-01-defining-an-enum.html). Use for understanding Measurement as a fixed set of alternatives.
- [Rust Book: match](https://doc.rust-lang.org/book/ch06-02-match.html). Primary source for lesson 1: patterns, arm expressions, and exhaustiveness.
- [Rust Book: references and borrowing](https://doc.rust-lang.org/book/ch04-02-references-and-borrowing.html). Use for shared and mutable references in validation and storage APIs.
- [Rust Book: recoverable errors with Result](https://doc.rust-lang.org/book/ch09-02-recoverable-errors-with-result.html). Use for Ok, Err, and propagation with `?`.
- [Rust Book: traits](https://doc.rust-lang.org/book/ch10-02-traits.html). Use for trait implementations and shared behavior.
- [std::str::FromStr](https://doc.rust-lang.org/std/str/trait.FromStr.html) and [std::fmt::Display](https://doc.rust-lang.org/std/fmt/trait.Display.html). Contracts to compare with exercise_types.rs, not proof that its implementation is ideal.
- [Rust Book: test organization](https://doc.rust-lang.org/book/ch11-03-test-organization.html). Use for distinguishing unit tests from integration tests and understanding binary testing.
- [Cargo Book: cargo bench](https://doc.rust-lang.org/cargo/commands/cargo-bench.html). Use for benchmark profiles and harness behavior; select measurement tooling only once the workload and question are defined.

## Wisdom (Communities)

- [Rust users forum](https://users.rust-lang.org/). Optional venue for a bounded design question with a minimal example and stated tradeoffs. No joining or posting is required; community preferences are unknown.

## Gaps

- Before storage lessons, consult official rusqlite and SQLite documentation for the exact APIs and transaction behavior being traced.
- Before performance work, verify official documentation for the chosen timing/profiling tools and reproducibility controls.
