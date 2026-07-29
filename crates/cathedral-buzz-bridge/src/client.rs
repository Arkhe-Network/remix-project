use anyhow::Result;
use nostr::Keys;
use url::Url;

pub struct BuzzBridgeClient {
    pub url: Url,
    pub keys: Keys,
}

impl BuzzBridgeClient {
    pub fn new(url: &str, keys: Keys) -> Result<Self> {
        let url = Url::parse(url)?;
        Ok(Self { url, keys })
    }
}
