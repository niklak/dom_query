# Safe mutations benchmark

Measures the cost of the `safe-mutations` run-wide ancestor checks during
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
- Every iteration checks all 1,000 moved nodes against the target’s ancestors
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
  --features dom_query/safe-mutations -- --baseline depth-off
```

## Results

| Chain depth | Feature off | `safe-mutations` | Slowdown |
|---:|---:|---:|---:|
| 10 | 4.96 µs | 15.39 µs | 3.1× |
| 100 | 5.06 µs | 180.22 µs | 35.6× |
| 1,000 | 5.22 µs | 1.82 ms | 349× |
| 10,000 | 5.21 µs | 18.28 ms | 3,505× |

All comparisons report `p = 0.00`. The added cost is about **1.8 ns per
ancestor comparison**, so it scales approximately linearly with depth.

## Interpretation

The run-wide check is `O(run length × receiver ancestor depth)`. With a
1,000-node run, even a depth of 10 already costs about 3× the baseline; at
depth 10,000 it is roughly 3,500× slower.

This is a synthetic worst case: real documents are shallower and usually move
much shorter runs. The result shows why the check should be reduced to a
set-style test (`O(run + depth)`) before `safe-mutations` is treated as cheap
for large mutations.
