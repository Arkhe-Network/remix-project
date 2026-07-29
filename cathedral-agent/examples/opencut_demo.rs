use cathedral_agent::integrations::opencut_script::{OpenCutProjectBuilder, TrackType, TransitionType};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let project = OpenCutProjectBuilder::new("Demo Video")
        .resolution(1920, 1080)
        .fps(30)
        .description("Vídeo de demonstração")
        .add_track("Main Video", TrackType::Video)
        .add_clip("video1.mp4", 0.0, 10.0)
        .add_clip("video2.mp4", 10.0, 15.0)
        .add_transition("clip-1", "clip-2", TransitionType::CrossFade, 1.0)
        .add_audio("music.mp3", 0.8, 0.0, 25.0)
        .build();

    println!("✅ OpenCut Demo Project Created: {}", project.name);
    Ok(())
}
