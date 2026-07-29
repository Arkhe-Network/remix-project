---------------------------- MODULE ARKHE_Composition ----------------------------
EXTENDS ARKHE_AASM, ARKHE_TraceFix, ARKHE_Loops

SecOpsSafety ==
    [][ \A i \in 1..Len(Ledger) :
         Ledger[i].type = "SecOpsCheck" =>
            LET aid == Ledger[i].artifact
                proj == CurrentProjection
            IN proj.A[aid] # None =>
                proj.A[aid].hash = HashOf[aid] ]_vars

DevOpsSafety ==
    [][ \A i \in 1..Len(Ledger) :
         Ledger[i].type = "DevOpsDeploy" =>
            LET aid == Ledger[i].artifact
            IN <>\E dep \in CurrentProjection.Deployments :
                dep.artifact = aid /\ dep.status = "verified" ]_vars

DevSecOpsSafety ==
    [][ \A i \in 1..Len(Ledger) :
         Ledger[i].type = "DevSecOpsAudit" =>
            LET aid == Ledger[i].artifact
                proj == CurrentProjection
            IN \E audit \in proj.Audits :
                audit.artifact = aid /\ audit.result = "pass" ]_vars

GlobalSafety == SecOpsSafety /\ DevOpsSafety /\ DevSecOpsSafety

CompositionSafety == [](GlobalSafety)

NoInterference ==
    [][ \A i, j \in 1..Len(Ledger) :
         i # j /\ Ledger[i].type = "DecisionMade" /\ Ledger[j].type = "DecisionMade" =>
            Ledger[i].artifact # Ledger[j].artifact ]_vars

=============================================================================
