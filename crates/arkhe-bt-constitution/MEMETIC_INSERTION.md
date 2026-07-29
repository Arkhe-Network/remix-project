# Memetic Insertion: Constitutional Specification

## 1. Definition

**Memetic Insertion** is the dynamic injection of behavioral patterns (subtrees)
into an active Behavior Tree based on external stimuli. The term "memetic"
refers to the replicable, transmissible nature of these patterns — they behave
like cultural memes, propagating across agent instances and generations.

## 2. Stimuli Sources

| Source | Description | Trust Level |
|--------|-------------|-------------|
| Red-Team Feedback | Adversarial test results prompting behavioral patches | High (signed) |
| Operator Override | Human operator injects corrective subtree | Highest (multisig) |
| Learned Heuristics | Agent-derived patterns from experience | Low (sandboxed) |
| Constitutional Update | Formal amendment to the invariant set | Highest (governance vote) |

## 3. Constitutional Constraints

### 3.1 Sovereignty Gate
All insertions MUST pass through `SovereigntyGate`. Assets (synthetic consciousness)
MUST NOT initiate insertions. Only operators with valid cryptographic credentials
may trigger insertion.

### 3.2 Consent Registry
Insertions that modify safety-critical paths (paths containing `ShieldNode`)
MUST have explicit consent recorded in the `ConsentRegistry`.

### 3.3 Rate Limiting (I7)
The insertion rate `λ_ins` MUST NOT exceed `λ_ins_max` (default: 10 nodes/sec).
Violations trigger `AccelerationGuardNode` deceleration.

### 3.4 Evidence Requirement
Every inserted subtree MUST carry an `EvidencePayload` with:
- Cryptographic signature of the insertion source
- Timestamp and provenance chain
- Hash of the pre-insertion tree state (for rollback)

### 3.5 Shield Node Protection
Insertions MUST NOT displace or reorder `ShieldNode` instances.
The safety lock mechanism prevents any insertion from interrupting
a safety-critical sequence.

## 4. Falsification Criteria

A memetic insertion is **constitutionally valid** iff:
1. `SovereigntyGate::tick()` returns `Status::Success`
2. `ConsentCheckNode::tick()` returns `Status::Success`
3. `AccelerationMonitor::insertion_rate() ≤ λ_ins_max`
4. `EvidencePayload::verify_signature()` returns `Ok(())`
5. No `ShieldNode` is displaced (verified by tree diff)

## 5. Rollback Protocol

If an inserted subtree causes `GoalDriftNode` to detect drift:
1. Halt execution of the inserted subtree
2. Restore pre-insertion tree state (from EvidencePayload hash)
3. Log rollback event with full evidence
4. Notify operator

## 6. Terminology Note

The term "memetic insertion" is retained because it precisely describes
the propagation of behavioral patterns across agents. It is NOT a metaphor
— it is a technical term for a specific class of dynamic tree modifications
that are replicable and transmissible.
