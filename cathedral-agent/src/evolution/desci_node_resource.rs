use crate::evolution::resource::{Resource, ResourceMetadata, ResourceInterface, ResourceState};
use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use chrono::Utc;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum NodeStatus {
    Draft,
    Submitted,
    UnderReview,
    Published,
    Revised,
    Retracted,
    Archived,
}

impl std::fmt::Display for NodeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Draft => write!(f, "draft"),
            Self::Submitted => write!(f, "submitted"),
            Self::UnderReview => write!(f, "under_review"),
            Self::Published => write!(f, "published"),
            Self::Revised => write!(f, "revised"),
            Self::Retracted => write!(f, "retracted"),
            Self::Archived => write!(f, "archived"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ComponentType {
    Manuscript,
    Dataset,
    Code,
    Model,
    Pipeline,
    Figure,
    Supplementary,
    Custom(String),
}

impl std::fmt::Display for ComponentType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Manuscript => write!(f, "manuscript"),
            Self::Dataset => write!(f, "dataset"),
            Self::Code => write!(f, "code"),
            Self::Model => write!(f, "model"),
            Self::Pipeline => write!(f, "pipeline"),
            Self::Figure => write!(f, "figure"),
            Self::Supplementary => write!(f, "supplementary"),
            Self::Custom(s) => write!(f, "{}", s),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchComponent {
    pub component_type: ComponentType,
    pub name: String,
    pub description: Option<String>,
    pub content_hash: String,
    pub cid: Option<String>,
    pub mime_type: Option<String>,
    pub size_bytes: Option<u64>,
    pub metadata: HashMap<String, String>,
    pub created_at: u64,
    pub updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeVersion {
    pub version: String,
    pub status: NodeStatus,
    pub components: Vec<ResearchComponent>,
    pub created_at: u64,
    pub created_by: String,
    pub changelog: Option<String>,
    pub parent_version: Option<String>,
    pub content_hash: String,
    pub dpid: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContributorCredit {
    pub npub: String,
    pub orcid: Option<String>,
    pub role: ContributorRole,
    pub contribution_score: f32,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ContributorRole {
    Author,
    CoAuthor,
    DataCurator,
    CodeDeveloper,
    Reviewer,
    Editor,
    Funder,
    Supervisor,
    Custom(String),
}

impl std::fmt::Display for ContributorRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Author => write!(f, "author"),
            Self::CoAuthor => write!(f, "co_author"),
            Self::DataCurator => write!(f, "data_curator"),
            Self::CodeDeveloper => write!(f, "code_developer"),
            Self::Reviewer => write!(f, "reviewer"),
            Self::Editor => write!(f, "editor"),
            Self::Funder => write!(f, "funder"),
            Self::Supervisor => write!(f, "supervisor"),
            Self::Custom(s) => write!(f, "{}", s),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeSciNodeResource {
    pub metadata: ResourceMetadata,
    pub node_id: String,
    pub dpid: Option<String>,
    pub title: String,
    pub subtitle: Option<String>,
    pub abstract_text: Option<String>,
    pub keywords: Vec<String>,
    pub status: NodeStatus,
    pub versions: Vec<NodeVersion>,
    pub current_version: String,
    pub contributors: Vec<ContributorCredit>,
    pub external_refs: Vec<ExternalReference>,
    pub tags: Vec<String>,
    pub license: Option<String>,
    pub journal_submission: Option<JournalSubmission>,
    pub peer_reviews: Vec<PeerReviewRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalReference {
    pub ref_type: String,
    pub value: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalSubmission {
    pub journal_name: String,
    pub submitted_at: u64,
    pub status: SubmissionStatus,
    pub reviewer_comments: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SubmissionStatus {
    Submitted,
    UnderReview,
    Accepted,
    Rejected,
    Resubmitted,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerReviewRecord {
    pub reviewer_npub: String,
    pub reviewer_orcid: Option<String>,
    pub score: u8,
    pub comments: String,
    pub is_public: bool,
    pub reviewed_at: u64,
    pub version: String,
}

#[derive(Debug, Clone)]
pub enum VersionBump {
    Major,
    Minor,
    Patch,
}

impl DeSciNodeResource {
    pub fn new(node_id: &str, title: &str, author_npub: &str, author_orcid: Option<&str>) -> Self {
        let now = Utc::now().timestamp() as u64;
        let id = format!("desci:node:{}", node_id);
        let version = "1.0.0".to_string();

        let interface = ResourceInterface {
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "action": { "enum": ["add_component", "update_metadata", "submit", "publish", "evolve"] },
                    "params": { "type": "object" }
                }
            }),
            output_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "node": { "type": "object" },
                    "new_version": { "type": "string" },
                    "dpid": { "type": "string" }
                }
            }),
            side_effects: vec!["publishes_research".to_string(), "modifies_node".to_string()],
            dependencies: vec![],
        };

        let initial_version = NodeVersion {
            version: version.clone(),
            status: NodeStatus::Draft,
            components: Vec::new(),
            created_at: now,
            created_by: author_npub.to_string(),
            changelog: Some("Versão inicial".to_string()),
            parent_version: None,
            content_hash: String::new(),
            dpid: None,
        };

        let metadata = ResourceMetadata {
            id: id.clone(),
            version: version.clone(),
            state: ResourceState::Active,
            interface,
            created_at: now,
            updated_at: now,
            author: author_npub.to_string(),
            provenance: Vec::new(),
            tags: vec!["desci".to_string(), "research".to_string()],
        };

        Self {
            metadata,
            node_id: node_id.to_string(),
            dpid: None,
            title: title.to_string(),
            subtitle: None,
            abstract_text: None,
            keywords: Vec::new(),
            status: NodeStatus::Draft,
            versions: vec![initial_version],
            current_version: version,
            contributors: vec![
                ContributorCredit {
                    npub: author_npub.to_string(),
                    orcid: author_orcid.map(|s| s.to_string()),
                    role: ContributorRole::Author,
                    contribution_score: 1.0,
                    metadata: HashMap::new(),
                }
            ],
            external_refs: Vec::new(),
            tags: vec!["research".to_string()],
            license: Some("CC-BY-4.0".to_string()),
            journal_submission: None,
            peer_reviews: Vec::new(),
        }
    }

    pub fn get_reputation_score(&self) -> f32 {
        let mut score = 0.0;
        if self.status == NodeStatus::Published {
            score += 20.0;
        }
        let review_score = (self.peer_reviews.len() as f32).min(10.0) * 3.0;
        score += review_score;
        let contributor_score = (self.contributors.len() as f32).min(5.0) * 2.0;
        score += contributor_score;
        let version_score = (self.versions.len() as f32).min(5.0) * 2.0;
        score += version_score;
        if !self.external_refs.is_empty() {
            score += 5.0;
        }
        if self.dpid.is_some() {
            score += 5.0;
        }

        score.min(100.0)
    }

    pub fn is_fair_compliant(&self) -> bool {
        let has_metadata = self.title.len() > 3
            && self.abstract_text.is_some()
            && !self.keywords.is_empty();
        let has_components = !self.versions.is_empty()
            && self.versions.iter().any(|v| !v.components.is_empty());
        let has_pid = self.dpid.is_some();
        let has_license = self.license.is_some();
        let has_provenance = !self.metadata.provenance.is_empty();

        has_metadata && has_components && has_pid && has_license && has_provenance
    }
}

impl Resource for DeSciNodeResource {
    fn metadata(&self) -> &ResourceMetadata {
        &self.metadata
    }

    fn metadata_mut(&mut self) -> &mut ResourceMetadata {
        &mut self.metadata
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn to_bytes(&self) -> Result<Vec<u8>, String> {
        serde_json::to_vec(self)
            .map_err(|e| format!("Erro ao serializar DeSciNodeResource: {}", e))
    }

    fn from_bytes(bytes: &[u8]) -> Result<Self, String> {
        serde_json::from_slice(bytes)
            .map_err(|e| format!("Erro ao deserializar DeSciNodeResource: {}", e))
    }
}
