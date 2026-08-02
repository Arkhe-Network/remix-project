use anyhow::{Result, Context};
use nostr::{Keys, EventBuilder, Kind, Tag, EventId};
use nostr::event::TagKind;
use url::Url;

use crate::core::types::OrchORState;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct EvidenceBundle {
    pub hypothesis: String,
    pub baseline: String,
    pub cert: String,
    pub pumps: Vec<String>,
}

pub struct BuzzBridgeClient {
    pub url: Url,
    pub keys: Keys,
}

impl BuzzBridgeClient {
    pub fn new(url: &str, nsec: &str) -> Result<Self> {
        let url = Url::parse(url)?;
        // T3 - No unwrap_or_else silently masking parsing errors
        let keys = Keys::parse(nsec).context("Failed to parse Nostr keys")?;
        Ok(Self { url, keys })
    }

    pub fn publish_aft_frame(&self, frame: &[u8], session_id: &str) -> Result<EventId> {
        let content = hex::encode(frame);
        let tags = vec![
            Tag::custom(TagKind::from("experiment"), vec!["type=aft_frame".to_string()]),
            Tag::custom(TagKind::from("session_id"), vec![session_id.to_string()]),
        ];

        let event = EventBuilder::new(Kind::Custom(30000), content).tags(tags).sign_with_keys(&self.keys)?;
        Ok(event.id)
    }

    pub fn publish_orchor_state(&self, state: &OrchORState) -> Result<EventId> {
        let content = serde_json::to_string(state)?;
        let tags = vec![
            Tag::custom(TagKind::from("coherence_time"), vec![state.coherence_time.to_string()]),
            Tag::custom(TagKind::from("frequency"), vec![state.frequency.to_string()]),
            Tag::custom(TagKind::from("regime"), vec![state.regime.to_string()]),
        ];

        let event = EventBuilder::new(Kind::Custom(30003), content).tags(tags).sign_with_keys(&self.keys)?;
        Ok(event.id)
    }

    // T9 - No unwrap_or_default
    pub fn evidence_bundle_to_event(&self, bundle: &EvidenceBundle) -> Result<EventId> {
        let content = serde_json::to_string(bundle).context("Failed to serialize EvidenceBundle")?;

        let tags = vec![
            Tag::custom(TagKind::from("hypothesis"), vec![bundle.hypothesis.clone()]),
            Tag::custom(TagKind::from("baseline"), vec![bundle.baseline.clone()]),
            Tag::custom(TagKind::from("cert"), vec![bundle.cert.clone()]),
            Tag::custom(TagKind::from("pump"), bundle.pumps.clone()),
        ];

        let event = EventBuilder::new(Kind::Custom(30001), content).tags(tags).sign_with_keys(&self.keys)?;
        Ok(event.id)
    }
}
