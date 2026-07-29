---------------------------- MODULE ARKHE_Main ----------------------------
EXTENDS ARKHE_Proofs

Invariants ==
    /\ I1_TypeOK
    /\ I4_ValidRefs
    /\ I6_Immutability
    /\ I7_AppendOnly
    /\ AASM_Invariants
    /\ NoInterference

Properties ==
    /\ Progress
    /\ CompositionSafety
    /\ AgentLiveness
    /\ AllLoopsLiveness

THEOREM Spec => []TypeOK

=============================================================================
