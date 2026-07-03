---------------------------- MODULE ARKHE_State ----------------------------
EXTENDS ARKHE_Projection

VARIABLES
    Ledger,
    activeAgent,
    activeLoop

vars == <<Ledger, activeAgent, activeLoop>>

TypeOK ==
    /\ Ledger \in StateType
    /\ Len(Ledger) =< MaxReplay
    /\ activeAgent \in HumanAgents \cup LoopAgents \cup {"none"}
    /\ activeLoop \in LoopAgents \cup {"none"}

Init ==
    /\ Ledger = <<>>
    /\ activeAgent = "none"
    /\ activeLoop = "none"
    /\ TypeOK

=============================================================================
