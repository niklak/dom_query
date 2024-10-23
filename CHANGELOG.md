# Changelog

All notable changes to the `dom_query` crate will be documented in this file.

## [Unreleased]

### Added
- Enable support for `:is()` and `:where()` pseudo-classes.

### Changed
- Exposed `Matcher::match_element` since it can be useful outside the crate.
- Changed `impl<'a> selectors::Element for Node<'a>::opaque` to work with `:has` pseudo-element from `selectors` crate.
- Switched to `:has` implementation from `selectors` crate.
- Internal changes due to switch to `selectors` v0.26.0 switch.

### Added
- Added `Node::ancestors` method, that allows to get all or limited number of ancestors of a node.
- Added `From<Vec<NodeRef<'a, NodeData>>> for Selection<'a>`

## [0.6.0] - 2024-10-19

### Changed
- Exposed `Document::tree`.
- `Selection` methods that required `&mut` now doesn't require `&mut`.
- Changed the project structure, now modules are divided based on the `struct` implementations.

### Added
- Added `Node::append_html` and `Node::set_html` methods for creating children nodes of a single selected node.
- Added `Tree<NodeData>::new_element`, an easy way to create an empty element with a given name.
- Added `NodeRef::last_child`.
- Added `Node::has_attr` method, which returns `true` if an attribute exists on the node element. 
`Selection::has_attr` does the same thing for the **first** node inside selection.
- Added `Node::remove_all_attrs` method for removing all attributes of a node. 
`Selection::remove_all_attrs` does the same thing for the **every** node inside selection.
- Added `Node::remove_attrs` method, a convenient way to remove multiple attributes from the node element. 
`Selection::remove_attrs` does the same thing for the **every** node inside selection.
- Added `Node::rename` method, which allows to change node's name. 
`Selection::rename` does the same thing for the **every** node inside selection.


## [0.5.0] - 2024-10-10

### Added
- Added `select_single_matcher` and `select_single` methods for `Document` and `Selection`.
- Added `Document::fragment` which allows to create a document fragment.

### Changed
- Update documentation
- *A small breaking change*: `From` implementation for `Document`, now it is based on `Into<StrTendril>` and because of that, previous `From<&String>` implementation will not work anymore (they are in config). If your code consumes `&String`, you should use `String::as_str()` instead.
- Refactored the code (`NodeData::Element`).

## [0.4.2] - 2024-10-05

### Removed
- Removed a `readability` examples and related dev-dependencies.

### Added
- Added `Selection::inner_html`, `Selection::try_html`, and `Selection::try_inner` methods.
- Added more examples and doc-tests.

### Changed
- Improved the documentation.

## [0.4.1] - 2024-10-04

### Fixed
- Fixed `Iterator::next` for `Matches`

### Added
- Added doc-tests

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
