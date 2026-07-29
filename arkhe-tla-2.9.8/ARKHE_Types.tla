---------------------------- MODULE ARKHE_Types ----------------------------
EXTENDS Integers, FiniteSets, Sequences

CONSTANT
    None,
    ArtifactIDs,
    EventIDs,
    DecisionIDs,
    ActionIDs,
    HumanAgents,
    LoopAgents,
    Payloads,
    Hashes,
    MaxReplay,
    HashOf

ASSUME
    None \notin ArtifactIDs \cup EventIDs \cup DecisionIDs \cup ActionIDs \cup HumanAgents \cup LoopAgents \cup Payloads \cup Hashes
    /\ HashOf \in [ArtifactIDs -> Hashes]
    /\ Payloads # {}
    /\ HumanAgents # {}
    /\ MaxReplay \in Int /\ MaxReplay >= 1

ConfidenceLevel == {0, 1, 2}

EventType == {
    "ArtifactAdded", "ArtifactRemoved", "DecisionMade", "BeliefUpdated",
    "ConsentGranted", "DeploymentVerified", "CredentialIssued",
    "SecOpsCheck", "DevOpsDeploy", "DevSecOpsAudit",
    "OntologyInferred", "IntentClassified",
    "ContextRetrieved", "ContextUpdated",
    "PromptGenerated", "LLMResponse",
    "SemanticGrounded", "AmbiguityResolved",
    "CausalGraphUpdated", "InterventionPlanned",
    "SelfAssessment", "StrategyAdjusted",
    "UncertaintyQuantified", "KnowledgeGap",
    "BiasDetected", "MitigationSuggested",
    "ModelUpdate", "GradientComputed",
    "SecurityAlert", "ThreatMitigated",
    "DeployInitiated", "RollbackExecuted",
    "AuditPassed", "ComplianceChecked",
    "CVEPrioritized", "PatchDeployed",
    "MemoryConsolidated", "MemoryPruned",
    "MessageSent", "CollaborationEstablished",
    "TaskScheduled", "ResourceAllocated",
    "DecisionAccepted", "DecisionRejected",
    "BeliefStrengthened", "BeliefWeakened",
    "CausalLinkAdded", "CausalLinkRemoved",
    "NovelIdeaGenerated", "IdeaEvaluated",
    "EmotionDetected", "EmpathicResponse",
    "EthicalCheckPassed", "EthicalViolation",
    "TaskDelegated", "TaskCompleted"
}

DecisionType == {"Accept", "Reject", "Defer"}

Artifact == [id: ArtifactIDs, payload: Payloads, hash: Hashes]

Event == [
    id: EventIDs,
    type: EventType,
    artifact: ArtifactIDs,
    payload: MaybePayload,
    timestamp: Int,
    agent: HumanAgents \cup LoopAgents \cup {None},
    action: ActionIDs \cup {None}   (* NOVO campo *)
]

Decision == [id: DecisionIDs, event: EventIDs, type: DecisionType, confidence: ConfidenceLevel]

Credential == [agent: HumanAgents, expiry: Int, issuer: HumanAgents]
Consent    == [agent: HumanAgents, action: ActionIDs, granted: BOOLEAN, timestamp: Int]
Deployment == [id: Int, artifact: ArtifactIDs, status: {"pending","verified","failed"}, timestamp: Int]
Audit      == [id: Int, event: EventIDs, artifact: ArtifactIDs, action: ActionIDs,
               agent: HumanAgents, result: {"pass","fail"}, timestamp: Int]

MaybeArtifact == Artifact \cup {None}
MaybeEvent    == Event    \cup {None}
MaybeDecision == Decision \cup {None}
MaybeCredential == Credential \cup {None}
MaybeConsent    == Consent    \cup {None}
MaybeDeployment == Deployment \cup {None}
MaybeAudit      == Audit      \cup {None}
MaybePayload    == Payloads   \cup {None}

StateType == Seq(Event)

=============================================================================
