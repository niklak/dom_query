# Changelog

## [0.4.2] - 2024-10-05

### Removed
- Removed a `readability` examples and related dev-dependencies.

### Added
- Added `Selection::inner_html`, `Selection::try_html`, and `Selection::try_inner` methods

## [0.4.1] - 2024-10-04

### Fixed
- Fix `Iterator::next` for `Matches`

### Added
- Add doc-tests

## [0.4.0] - 2024-10-01

### Changed
- Update dependencies.
- Refactor code due to major changes in html5ever 0.29.0


## [0.3.6] - 2024-07-19

### Changed
- Update dependencies.

## [0.3.5] - 2024-04-04

### Changed
- Update dependencies.

## [0.3.4] - 2024-02-17

### Added
- Add support for `:has-text` and `:contains` pseudo-classes, which allow to search elements by their text contents. Some example are [here](./tests/pseudo-class.rs).

## [0.3.3] - 2024-02-10

### Fixed
- Fix `:has` selector name comparison. Previously `:has` behavior worked, even if pseudo-class just started from `:has`. 

## [0.3.2] - 2024-01-19

### Added

- add `InnerNode<NodeData>.is_comment`.

### Changed

- expose `dom_tree::NodeData`.
- minor adjustments.
- revise `matcher::Matcher` usage, now `selection::Selection`'s underlying `select` methods uses `matcher::Matcher` reference if it possible.
- `css::CssString` now wraps `String` instead of `StrTendril`. Because `StrTendril` can't be shared between multiple threads without mutex.

## [0.3.1] - 2024-01-03

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
