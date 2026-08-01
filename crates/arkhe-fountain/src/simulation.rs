use std::process::Command;
use serde::Deserialize;

#[derive(Deserialize)]
pub struct SimulationResult {
    pub k: usize,
    pub loss_rate: f64,
    pub block_size: usize,
    pub n_frames: usize,
    pub success: bool,
    pub frames_transmitted: usize,
    pub frames_received: usize,
    pub progress: f64,
    pub time_ms: u128,
}

pub fn run_simulation(k: usize, loss_rate: f64, block_size: usize, n_frames: usize) -> SimulationResult {
    let output = Command::new("cargo")
        .args(&[
            "run",
            "--release",
            "--bin", "arkhe-fountain",
            "--",
            "-k", &k.to_string(),
            "-l", &loss_rate.to_string(),
            "-b", &block_size.to_string(),
            "-n", &n_frames.to_string()
        ])
        .output()
        .expect("Failed to run simulation");

    let output_str = String::from_utf8(output.stdout).expect("Invalid UTF-8");
    serde_json::from_str(&output_str).expect("Failed to parse output")
}
