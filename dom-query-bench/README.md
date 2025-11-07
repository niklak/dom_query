## dom_query benchmarks
### Selection

There are many ways to select elements with dom_query. Please choose the one you want to use.
I want to show the performance differences between them using benchmarks.
According to the benchmark results, I've made the table below:

CPU: AMD Ryzen 9 6900HX with Radeon Graphics

| Benchmark | Time | Corresponding methods |
|-----------|------|----------------------|
| dom_query/selection/simple | 36.572 µs | `Selection::select` |
| dom_query/selection/with_matcher | 36.439 µs | `Selection::select_matcher` |
| dom_query/selection/with_matcher_iter | 35.935 µs | `Selection::select_matcher_iter` |
| dom_query/selection/serial | 16.432 µs | `Selection::select_single`, `Selection::select`, one after another sequentially |
| dom_query/selection/find | 7.8959 µs | `NodeRef::find` |
| dom_query/selection/find_descendants | 7.4259 µs | `NodeRef::find_descendants`|

As you can see, the first three approaches (`simple`, `with_matcher`, `with_matcher_iter`) have almost the same performance.

The serial approach is twice as fast as the previous three. This is because `selectors` always performs *ascending matching*. It thoroughly checks every node in the selection and all of its ancestors, but this is not very efficient from a performance perspective.
When you divide the selection by statements into smaller parts, you reduce the number of nodes to match.

**Example**
```rust
// Checks every node in the tree.
doc.select("body td.title a[href]")

doc
    .select_single("body") // Stops on the first match, becomes a root for the following selection (td.title).
    .select("td.title") // Checks elements only within the body element.
    .select("a[href]"); // Checks elements only within the td.title elements.
```

`NodeRef::find` is a fast method, but it doesn't support CSS selectors at all (only element names). So the actual results differ slightly from the other approaches. Also, it has a less convenient interface (`find(&self, path: &[&str]) -> Vec<Self>`).

`NodeRef::find_descendants` (requires the `mini-selector` feature) is the fastest method in the benchmark. It has limited CSS support (tags, classes, IDs, attributes, basic combinators). It doesn't support pseudo-classes or the `,` combinator. It collects matching elements for each statement in the CSS selector, and these become the root selection for subsequent matching.