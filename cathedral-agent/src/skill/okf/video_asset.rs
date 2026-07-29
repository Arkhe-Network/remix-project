// src/skill/okf/video_asset.rs
pub struct VideoAsset {
    pub name: String,
    pub description: String,
    pub content: Vec<u8>,
    pub metadata: VideoMetadata,
}

pub struct VideoMetadata {
    pub duration: f64,
    pub resolution: String,
    pub format: String,
    pub fps: u32,
    pub created_at: u64,
}
