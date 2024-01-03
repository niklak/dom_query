# Changelog


## [0.3.1] - 2023-12-31
### Changed
- Minor refactoring.
### Fixed
- Fix spelling.
## [0.3.0] - 2023-12-31
### Changed
- Refactor dom_tree.rs -- remove internal macro, revise some functions for better performance.
- Improve performance for some functions inside, traversal.rs (switching from  hashmaps to vecs).
- Add optional feature `hashbrown` -- which replace standard `HashSet` and `HashMap` with it's own implementation.

## [0.2.1] - 2023-12-30
### Changed
- `CssString` now wraps a `StrTendril` instead of a `String`.
- Use `rustc_hash::FxHashSet` instead of `std::collections::HashSet`.
- Use `rustc_hash::FxHashMap` instead of `std::collections::HashMap`.

## [0.2.0] - 2023-12-26
### Changed
- Replace unsafe code with safe code.
