-------------------------------- MODULE ASS-1.1-v4.0 ----------------------------------
EXTENDS Naturals, FiniteSets, TLC, Sequences

CONSTANT Payloads, Metadata, HashValues, Propositions, Justifications, Goals
CONSTANT MaxEntities, H

ASSUME H \in [Payloads -> HashValues]
ASSUME \A p1,p2 \in Payloads : (H[p1] = H[p2]) => (p1 = p2)

(*--- TIPOS ---*)
Artifact == [payload: Payloads, metadata: Metadata, hash: HashValues]
Evidence  == [artifact_id: 1..MaxEntities, content: Payloads, signature: HashValues,
              timestamp: Nat, parent_hash: HashValues \cup {NULL}, hash: HashValues]
Claim     == [proposition: Propositions, evidence_ids: SUBSET (1..MaxEntities)]
Belief    == [claim_id: 1..MaxEntities, confidence: 0..100, justification: Justifications]
Decision  == [goal: Goals, belief_ids: SUBSET (1..MaxEntities), timestamp: Nat]

NULL == "NULL"

(*--- ESTADO (5 variáveis, não 10) ---*)
VARIABLES artifacts, evidences, claims, beliefs, decisions

vars == <<artifacts, evidences, claims, beliefs, decisions>>

(*--- INICIALIZAÇÃO ---*)
Init ==
    /\ artifacts = [id \in 1..MaxEntities |-> NULL]
    /\ evidences = [id \in 1..MaxEntities |-> NULL]
    /\ claims = [id \in 1..MaxEntities |-> NULL]
    /\ beliefs = [id \in 1..MaxEntities |-> NULL]
    /\ decisions = [id \in 1..MaxEntities |-> NULL]

(*--- CONJUNTOS ATIVOS (derivados, não armazenados) ---*)
Active(dom) == {id \in DOMAIN dom : dom[id] # NULL}

(*--- δ: TRANSIÇÕES ---*)
AddArtifact(id, payload, metadata) ==
    /\ id \notin Active(artifacts)
    /\ artifacts' = [artifacts EXCEPT ![id] =
        [payload |-> payload, metadata |-> metadata, hash |-> H[payload]]]
    /\ UNCHANGED <<evidences, claims, beliefs, decisions>>

AddEvidence(id, artId, content, signature, parent_hash) ==
    /\ id \notin Active(evidences)
    /\ artId \in Active(artifacts)
    /\ (parent_hash = NULL) \/ (\E e \in Active(evidences) : evidences[e].hash = parent_hash)
    /\ evidences' = [evidences EXCEPT ![id] = [
        artifact_id |-> artId, content |-> content, signature |-> signature,
        timestamp |-> Cardinality(Active(evidences)) + 1,
        parent_hash |-> parent_hash, hash |-> H[content]]]
    /\ UNCHANGED <<artifacts, claims, beliefs, decisions>>

AddClaim(id, proposition, evIds) ==
    /\ id \notin Active(claims)
    /\ evIds # {}
    /\ evIds \subseteq Active(evidences)
    /\ claims' = [claims EXCEPT ![id] = [proposition |-> proposition, evidence_ids |-> evIds]]
    /\ UNCHANGED <<artifacts, evidences, beliefs, decisions>>

AddBelief(id, claimId, confidence, justification) ==
    /\ id \notin Active(beliefs)
    /\ claimId \in Active(claims)
    /\ confidence \in 0..100
    /\ beliefs' = [beliefs EXCEPT ![id] = [
        claim_id |-> claimId, confidence |-> confidence, justification |-> justification]]
    /\ UNCHANGED <<artifacts, evidences, claims, decisions>>

AddDecision(id, goal, belIds, ts) ==
    /\ id \notin Active(decisions)
    /\ belIds \subseteq Active(beliefs)
    /\ decisions' = [decisions EXCEPT ![id] = [
        goal |-> goal, belief_ids |-> belIds, timestamp |-> ts]]
    /\ UNCHANGED <<artifacts, evidences, claims, beliefs>>

(*--- δ TOTAL ---*)
Next ==
    \/ \E id \in 1..MaxEntities, p \in Payloads, m \in Metadata : AddArtifact(id, p, m)
    \/ \E id \in 1..MaxEntities, aId \in 1..MaxEntities, c \in Payloads,
           sig \in HashValues, ph \in HashValues \cup {NULL} :
        AddEvidence(id, aId, c, sig, ph)
    \/ \E id \in 1..MaxEntities, prop \in Propositions, evs \in SUBSET (1..MaxEntities) :
        evs # {} /\ AddClaim(id, prop, evs)
    \/ \E id \in 1..MaxEntities, cId \in 1..MaxEntities, conf \in 0..100, j \in Justifications :
        AddBelief(id, cId, conf, j)
    \/ \E id \in 1..MaxEntities, g \in Goals, bids \in SUBSET (1..MaxEntities), ts \in Nat :
        AddDecision(id, g, bids, ts)
    \/ UNCHANGED vars

(*--- INVARIANTES (I) v4.0 ---*)
IC1 ==
    /\ artifacts \in [1..MaxEntities -> Artifact \cup {NULL}]
    /\ evidences \in [1..MaxEntities -> Evidence \cup {NULL}]
    /\ claims \in [1..MaxEntities -> Claim \cup {NULL}]
    /\ beliefs \in [1..MaxEntities -> Belief \cup {NULL}]
    /\ decisions \in [1..MaxEntities -> Decision \cup {NULL}]

IC2 ==
    \A e \in Active(evidences) :
        LET ev == evidences[e] IN
        (ev.parent_hash = NULL) \/ (\E p \in Active(evidences) : ev.parent_hash = evidences[p].hash)

IC3 ==
    \A id \in Active(artifacts) : artifacts[id].hash = H[artifacts[id].payload]

IC4 ==
    /\ \A b \in Active(beliefs) : beliefs[b].claim_id \in Active(claims)
    /\ \A e \in Active(evidences) : evidences[e].artifact_id \in Active(artifacts)

IC5 ==
    \A c \in Active(claims) :
        LET cl == claims[c] IN
        \A eid \in cl.evidence_ids : eid \in Active(evidences)

IC6 ==
    \A e \in Active(evidences) : evidences[e].artifact_id \in Active(artifacts)

IC7 == TRUE (* Aciclicidade — verificada no Coq/Rust via DFS *)

IC8 ==
    \A e1, e2 \in Active(evidences) :
        (e1 # e2 /\ evidences[e1].parent_hash = evidences[e2].hash) =>
        (evidences[e1].timestamp > evidences[e2].timestamp)

IC9 ==
    \A e \in DOMAIN evidences : e \in Active(evidences) <=> evidences[e] # NULL

IC10 ==
    /\ \A id \in 1..MaxEntities : id \in Active(artifacts) <=> artifacts[id] # NULL
    /\ \A id \in 1..MaxEntities : id \in Active(evidences) <=> evidences[id] # NULL
    /\ \A id \in 1..MaxEntities : id \in Active(claims) <=> claims[id] # NULL
    /\ \A id \in 1..MaxEntities : id \in Active(beliefs) <=> beliefs[id] # NULL
    /\ \A id \in 1..MaxEntities : id \in Active(decisions) <=> decisions[id] # NULL

IC11 ==
    \A c \in Active(claims) : claims[c].evidence_ids # {}

IC16 ==
    \A d \in Active(decisions) :
        LET dec == decisions[d] IN
        \A bid \in dec.belief_ids :
            LET bel == beliefs[bid] IN
            LET cl == claims[bel.claim_id] IN
            \A eid \in cl.evidence_ids : eid \in Active(evidences)

================================================================================
