// src/integrations/opencut_script.rs
//! DSL para gerar projetos OpenCut programaticamente

use serde::{Serialize, Deserialize};
use std::collections::HashMap;
use chrono::Utc;

// ─── Tipos ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenCutProject {
    pub version: String,
    pub name: String,
    pub description: Option<String>,
    pub resolution: Resolution,
    pub fps: u32,
    pub duration_seconds: f64,
    pub tracks: Vec<Track>,
    pub timeline: Vec<Clip>,
    pub transitions: Vec<TransitionDef>,
    pub audio: Vec<AudioTrack>,
    pub metadata: HashMap<String, String>,
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Resolution {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    pub id: String,
    pub name: String,
    pub track_type: TrackType,
    pub clips: Vec<Clip>,
    pub visible: bool,
    pub locked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrackType {
    Video,
    Audio,
    Text,
    Overlay,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Clip {
    pub id: String,
    pub source_path: String,
    pub start_time: f64,
    pub end_time: f64,
    pub duration: f64,
    pub position: Option<Position>,
    pub scale: Option<f32>,
    pub opacity: Option<f32>,
    pub effects: Vec<Effect>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Effect {
    pub effect_type: EffectType,
    pub params: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EffectType {
    Blur,
    Brightness,
    Contrast,
    Saturation,
    FadeIn,
    FadeOut,
    ChromaKey,
    Speed,
    Reverse,
    Crop,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionDef {
    pub from_clip_id: String,
    pub to_clip_id: String,
    pub transition_type: TransitionType,
    pub duration: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TransitionType {
    CrossFade,
    Slide,
    FadeToBlack,
    Zoom,
    Wipe,
    Iris,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioTrack {
    pub id: String,
    pub source_path: String,
    pub volume: f32,
    pub start_time: f64,
    pub duration: f64,
    pub fade_in: Option<f64>,
    pub fade_out: Option<f64>,
}

// ─── Builder DSL ──────────────────────────────────────────────────

pub struct OpenCutProjectBuilder {
    project: OpenCutProject,
    current_track_id: Option<String>,
}

impl OpenCutProjectBuilder {
    pub fn new(name: &str) -> Self {
        let project = OpenCutProject {
            version: "1.0.0".to_string(),
            name: name.to_string(),
            description: None,
            resolution: Resolution { width: 1920, height: 1080 },
            fps: 30,
            duration_seconds: 0.0,
            tracks: Vec::new(),
            timeline: Vec::new(),
            transitions: Vec::new(),
            audio: Vec::new(),
            metadata: HashMap::new(),
            created_at: Utc::now().timestamp() as u64,
        };
        Self {
            project,
            current_track_id: None,
        }
    }

    pub fn resolution(mut self, width: u32, height: u32) -> Self {
        self.project.resolution = Resolution { width, height };
        self
    }

    pub fn fps(mut self, fps: u32) -> Self {
        self.project.fps = fps;
        self
    }

    pub fn description(mut self, desc: &str) -> Self {
        self.project.description = Some(desc.to_string());
        self
    }

    pub fn metadata(mut self, key: &str, value: &str) -> Self {
        self.project.metadata.insert(key.to_string(), value.to_string());
        self
    }

    pub fn add_track(mut self, name: &str, track_type: TrackType) -> Self {
        let track = Track {
            id: format!("track-{}", self.project.tracks.len() + 1),
            name: name.to_string(),
            track_type,
            clips: Vec::new(),
            visible: true,
            locked: false,
        };
        self.current_track_id = Some(track.id.clone());
        self.project.tracks.push(track);
        self
    }

    pub fn add_clip(
        mut self,
        source_path: &str,
        start_time: f64,
        duration: f64,
    ) -> Self {
        let track_id = self.current_track_id
            .clone()
            .unwrap_or_else(|| "track-1".to_string());

        let clip = Clip {
            id: format!("clip-{}", self.project.timeline.len() + 1),
            source_path: source_path.to_string(),
            start_time,
            end_time: start_time + duration,
            duration,
            position: None,
            scale: None,
            opacity: None,
            effects: Vec::new(),
        };

        self.project.timeline.push(clip.clone());

        // Adiciona ao track correspondente
        if let Some(track) = self.project.tracks.iter_mut().find(|t| t.id == track_id) {
            track.clips.push(clip);
        }

        self.project.duration_seconds = self.project.duration_seconds.max(start_time + duration);
        self
    }

    pub fn add_clip_with_position(
        mut self,
        source_path: &str,
        start_time: f64,
        duration: f64,
        x: f32,
        y: f32,
        scale: f32,
        opacity: f32,
    ) -> Self {
        let track_id = self.current_track_id
            .clone()
            .unwrap_or_else(|| "track-1".to_string());

        let clip = Clip {
            id: format!("clip-{}", self.project.timeline.len() + 1),
            source_path: source_path.to_string(),
            start_time,
            end_time: start_time + duration,
            duration,
            position: Some(Position { x, y }),
            scale: Some(scale),
            opacity: Some(opacity),
            effects: Vec::new(),
        };

        self.project.timeline.push(clip.clone());
        if let Some(track) = self.project.tracks.iter_mut().find(|t| t.id == track_id) {
            track.clips.push(clip);
        }

        self.project.duration_seconds = self.project.duration_seconds.max(start_time + duration);
        self
    }

    pub fn add_transition(
        mut self,
        from_clip_id: &str,
        to_clip_id: &str,
        transition_type: TransitionType,
        duration: f64,
    ) -> Self {
        self.project.transitions.push(TransitionDef {
            from_clip_id: from_clip_id.to_string(),
            to_clip_id: to_clip_id.to_string(),
            transition_type,
            duration,
        });
        self
    }

    pub fn add_effect(
        mut self,
        clip_id: &str,
        effect_type: EffectType,
        params: HashMap<String, serde_json::Value>,
    ) -> Self {
        if let Some(clip) = self.project.timeline.iter_mut().find(|c| c.id == clip_id) {
            clip.effects.push(Effect {
                effect_type,
                params,
            });
        }
        self
    }

    pub fn add_audio(
        mut self,
        source_path: &str,
        volume: f32,
        start_time: f64,
        duration: f64,
    ) -> Self {
        self.project.audio.push(AudioTrack {
            id: format!("audio-{}", self.project.audio.len() + 1),
            source_path: source_path.to_string(),
            volume,
            start_time,
            duration,
            fade_in: None,
            fade_out: None,
        });
        self
    }

    pub fn add_audio_with_fade(
        mut self,
        source_path: &str,
        volume: f32,
        start_time: f64,
        duration: f64,
        fade_in: f64,
        fade_out: f64,
    ) -> Self {
        self.project.audio.push(AudioTrack {
            id: format!("audio-{}", self.project.audio.len() + 1),
            source_path: source_path.to_string(),
            volume,
            start_time,
            duration,
            fade_in: Some(fade_in),
            fade_out: Some(fade_out),
        });
        self
    }

    pub fn build(self) -> OpenCutProject {
        self.project
    }

    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(&self.project)
            .map_err(|e| format!("Erro ao serializar projeto: {}", e))
    }

    pub fn to_opencut_file(&self) -> Result<String, String> {
        self.to_json()
    }
}
