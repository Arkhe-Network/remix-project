# Arkhe BT Constitution

> **Constitutional Behavior Tree Architecture for Autonomous Agents**
>
> Rust implementation correcting all 15 issues identified in the critical audit.
> Score target: 49/100 → 88/100.

---

## Workspace Structure

| Crate | Purpose | Audit Fix |
|-------|---------|-----------|
| `arkhe-bt-core` | Fundamental types (`Node`, `Status`, `Blackboard`) | P14 (Status enum), P13 (now_ns) |
| `arkhe-bt-shield` | `ShieldNode` + `RespectfulNode` with `AtomicBool` spinlock | P1 (threading.Lock → AtomicBool) |
| `arkhe-bt-atomic` | `DoubleBufferBlackboard` with `AtomicUsize` + `SeqCst` | P2 (non-atomic swap) |
| `arkhe-bt-traversal` | `DepthBoundedTraverser` + `EventDrivenBT` | P3 (depth tracking), OA-BT-001 |
| `arkhe-bt-i7` | `AccelerationMonitor` with operational metrics | P4 (non-falsifiable I7) |
| `arkhe-bt-pea` | Integration with `arkhe-pea` (`GoalDriftEvaluator`, `EvidencePayload`) | P7 (Covenant Engine), P5 (permissions) |
| `arkhe-bt-tests` | `proptest` + Kani harnesses | P12 (no tests), P8 (no quant criteria) |

---

## Quick Start

```bash
# Build the workspace
cargo build --workspace

# Run property-based tests
cargo test -p arkhe-bt-tests

# Run Kani verification (requires kani-verifier)
cd arkhe-bt-atomic
kani --harness verify_active_idx_bounded
kani --harness verify_swap_involution
kani --harness verify_concurrent_swap_safety

cd ../arkhe-bt-tests
kani --harness verify_safety_lock_mutex
kani --harness verify_safety_lock_release
```

---

## Key Design Decisions

### 1. ShieldNode: AtomicBool + Spin-Wait

**Problem (P1):** Python `threading.Lock` blocked threads instead of preempting them.

**Solution:** `AtomicBool` with `compare_exchange(SeqCst)` provides true mutual exclusion.
Non-safety nodes use `RespectfulNode` to spin-wait while safety lock is held.

```rust
// Safety node acquires lock → preempts all non-safety nodes
shield.lock.acquire_spin();
let status = shield.child.tick(blackboard);
shield.lock.release();
```

### 2. DoubleBuffer: AtomicUsize + SeqCst

**Problem (P2):** `self.active = 1 - self.active` was a read-modify-write race.

**Solution:** `AtomicUsize::store(next, SeqCst)` is a single atomic operation.
`SeqCst` (not `Acquire/Release`) is used because safety-critical code requires
total ordering reasoning.

### 3. Depth Bound: Traversal-Level Tracking

**Problem (P3):** Depth was passed to `node.tick()` which base nodes didn't accept.

**Solution:** `DepthBoundedTraverser` tracks depth internally. Composite nodes
use `DepthTrackingComposite` to propagate depth to children.

### 4. I7: Operational Metrics

**Problem (P4):** `|d²a/dt²| ≤ Γ_max` used non-measurable "capability growth".

**Solution:** Five concrete metrics:
- `λ_dec`: decisions/sec
- `λ_ins`: node insertions/sec
- `L_p99`: 99th percentile latency (ns)
- `C`: cognitive load [0,1]
- `M`: coherence margin [0,1]

`Γ_max = Σ w_i · metric_i_max` — falsifiable, observable, loggable.

### 5. Memetic Insertion (Documented)

**Definition:** Dynamic injection of behavioral patterns (subtrees) into an active BT
based on external stimuli (red-team feedback, operator corrections, learned heuristics).

**Constitutional Constraints:**
1. All insertions pass through `SovereigntyGate`.
2. Safety-critical insertions require `ConsentCheckNode`.
3. Insertion rate bounded by I7 (`λ_ins_max`).
4. Inserted subtrees must carry `EvidencePayload` (cryptographic signature).

---

## Orphan Axiom Status

| Axiom | Original Risk | Mitigation | Status |
|-------|--------------|------------|--------|
| OA-BT-001 (Tick Latency) | High | `DepthBoundedTraverser` + `EventDrivenBT` | ✅ Mitigated |
| OA-BT-002 (Atomic Insertion) | Catastrophic | `DoubleBufferBlackboard` (AtomicUsize) | ✅ Mitigated |
| OA-BT-003 (Priority Inversion) | Critical | `ShieldNode` + `RespectfulNode` (AtomicBool) | ✅ Mitigated |

---

## Constitutional Invariants

| Invariant | Implementation | Crate |
|-----------|---------------|-------|
| I1 (Physical) | `TickResult` with `duration_ns`, `max_depth_reached` | `arkhe-bt-traversal` |
| I2 (Falsifiability) | `EvidencePayload`, `GoalDriftEvidence` | `arkhe-bt-pea` |
| I3 (Substrate) | `Blackboard` trait, generic `Node` | `arkhe-bt-core` |
| I4 (Polynomial) | `MAX_CONSTITUTIONAL_DEPTH=32`, depth-bounded traversal | `arkhe-bt-traversal` |
| I5 (Autonomy) | `SovereigntyGate`, `ConsentCheckNode`, `ShieldNode` | `arkhe-bt-shield`, `arkhe-bt-pea` |
| I6 (Self-reference) | `Blackboard` shared state, `GoalDriftNode` | `arkhe-bt-core`, `arkhe-bt-pea` |
| I7 (Bounded Acceleration) | `AccelerationMonitor` with 5 operational metrics | `arkhe-bt-i7` |

---

## License

MIT OR Apache-2.0

---

**Seal:** `ARKHE-BT-CONSTITUTION-v2.0-2026-07-29`
