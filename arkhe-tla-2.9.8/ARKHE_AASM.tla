---------------------------- MODULE ARKHE_AASM ----------------------------
EXTENDS ARKHE_Projection, ARKHE_Replay, ARKHE_State

AASM_CredentialLifecycle ==
    \A a \in HumanAgents :
        LET proj == CurrentProjection
        IN proj.Creds[a] # None => proj.Creds[a].expiry > 0

AASM_ConsentEnforcement ==
    \A agent \in HumanAgents :
        LET proj == CurrentProjection
        IN \A action \in proj.Perms[agent] :
            \E consent \in DOMAIN proj.Consents :
                proj.Consents[consent].agent = agent
                /\ proj.Consents[consent].granted = TRUE

AASM_AuditCompleteness ==
    \A eid \in EventIDs :
        LET proj == CurrentProjection
        IN proj.E[eid] # None /\ proj.E[eid].type = "DecisionMade" =>
            \E audit \in proj.Audits : audit.event = eid

AASM_DataMinimization ==
    \A a \in ArtifactIDs :
        LET proj == CurrentProjection
        IN proj.A[a] # None => Len(proj.A[a].payload) <= 100

AASM_Resilience ==
    LET proj == CurrentProjection
    IN \E deployment \in proj.Deployments : deployment.status = "verified"

AASM_Invariants ==
    /\ AASM_CredentialLifecycle
    /\ AASM_ConsentEnforcement
    /\ AASM_AuditCompleteness
    /\ AASM_DataMinimization
    /\ AASM_Resilience

=============================================================================
