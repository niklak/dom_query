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
- One timed operation per case: `target.append_children(&run_start_id)`
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

| Chain depth | Feature off | `checked-node-mutations` | Slowdown vs off |
|---:|---:|---:|---:|
| 10 | 4.93 µs | 7.76 µs | 1.57× |
| 100 | 5.17 µs | 8.25 µs | 1.60× |
| 1,000 | 5.14 µs | 9.83 µs | 1.91× |
| 10,000 | 5.18 µs | 26.56 µs | 5.13× |

The `Feature off` and `checked-node-mutations` columns are from the same session.
The benchmark passes a precomputed `&NodeId`, so the always-on cross-tree check
takes its unknown-tree path; the pointer comparison itself is not measured. A
back-to-back run against the parent commit in a separate worktree stayed between
−1.1% and +2.5% across the four depths, with mixed signs, so the check is below
run-to-run noise.

All feature-on/off comparisons report `p = 0.00`. The check is
`O(run length + depth)`. The 1,000-node bit set costs about **3 µs**, and each
ancestor step adds about **1.9 ns**.

## Interpretation

This is a synthetic worst case: real documents are shallower and usually move
much shorter runs. The check remains opt-in because even the linear version is
several times slower than an unchecked mutation at extreme depth.
