# Checked node mutations benchmark

Measures the cost of the `checked-node-mutations` run-wide ancestor checks during
`NodeRef::append_children` at several target depths.

## Setup

- CPU: AMD Ryzen 9 6900HX
- Rust: 1.98.1
- Document: 11,005 nodes total
  - 10,000 nested `div` nodes
  - 1,000 sibling `span` nodes in a separate `<div id="run">`
  - root/html/head/body scaffolding
- Targets at chain depths `10`, `100`, `1,000`, and `10,000`
- One timed operation per case: `target.append_children(&first_run_span)`
- Each iteration inserts the 1,000 moved node ids into a bit set, then scans
  the target’s ancestor chain once
- `iter_batched_ref` clones the document outside the timed region; node IDs are
  precomputed, so cloning and selection are not measured
- Runs pinned to one CPU (`taskset -c 2`), 100 samples, 10 s measurement time

The 10,000-depth requirement makes the document larger than the earlier 8,000
node target.

## Commands

```bash
taskset -c 2 cargo bench -p dom-query-bench --bench safe_mutations \
  -- --save-baseline depth-off

taskset -c 2 cargo bench -p dom-query-bench --bench safe_mutations \
  --features dom_query/checked-node-mutations -- --baseline depth-off
```

## Results

| Chain depth | Feature off | `checked-node-mutations` before | `checked-node-mutations` now | Slowdown vs off |
|---:|---:|---:|---:|---:|
| 10 | 4.96 µs | 15.39 µs | 7.80 µs | 1.57× |
| 100 | 5.06 µs | 180.22 µs | 8.33 µs | 1.65× |
| 1,000 | 5.22 µs | 1.82 ms | 10.04 µs | 1.92× |
| 10,000 | 5.21 µs | 18.28 ms | 26.44 µs | 5.07× |

All comparisons report `p = 0.00`. The check is now `O(run length + depth)`.
The 1,000-node bit set costs about **3 µs**, and each ancestor step adds about
**1.9 ns**.

## Interpretation

The previous check was `O(run length × ancestor depth)`. The bit-set check
removes the multiplication: at depth 10,000 it drops from 18.28 ms to
26.44 µs, about 690× faster.

The fixed cost of building the 1,000-node bit set is small enough that the new
check is faster than the previous implementation at every measured depth,
including depth 10.

This is a synthetic worst case: real documents are shallower and usually move
much shorter runs. The check remains opt-in because even the linear version is
several times slower than an unchecked mutation at extreme depth.
