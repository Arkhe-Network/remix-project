use arkhe_fountain::Arkhe_Fountain_Encoder::{FountainEncoder, OrchORState};
use arkhe_fountain::Arkhe_Fountain_Decoder::{FountainDecoder, ErasureChannel};
use clap::Parser;
use rand::thread_rng;
use serde::Serialize;
use std::time::Instant;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(short = 'k', long, default_value_t = 256)]
    k: usize,

    #[arg(short = 'l', long, default_value_t = 0.5)]
    loss_rate: f64,

    #[arg(short = 'b', long, default_value_t = 16)]
    block_size: usize,

    #[arg(short = 'n', long, default_value_t = 1000)]
    n_frames: usize,
}

#[derive(Serialize)]
struct SimulationResult {
    k: usize,
    loss_rate: f64,
    block_size: usize,
    n_frames: usize,
    success: bool,
    frames_transmitted: usize,
    frames_received: usize,
    progress: f64,
    time_ms: u128,
}

fn main() {
    let args = Args::parse();
    let data_len = args.k * args.block_size;
    let data = vec![42u8; data_len]; // Dummy data

    let mut encoder = FountainEncoder::new(&data, args.block_size, 0.03, 0.5);
    let channel = ErasureChannel::new(args.loss_rate);
    let mut decoder = FountainDecoder::new();
    let mut rng = thread_rng();

    let mut transmitted = 0;
    let mut received = 0;

    let start = Instant::now();

    for _ in 0..args.n_frames {
        let frame = encoder.next_frame();
        transmitted += 1;
        if let Some(received_frame) = channel.transmit(&frame, &mut rng) {
            received += 1;
            if decoder.receive_frame(&received_frame).unwrap_or(false) {
                break;
            }
        }
    }

    let elapsed = start.elapsed();

    let result = SimulationResult {
        k: args.k,
        loss_rate: args.loss_rate,
        block_size: args.block_size,
        n_frames: args.n_frames,
        success: decoder.is_complete(),
        frames_transmitted: transmitted,
        frames_received: received,
        progress: decoder.progress(),
        time_ms: elapsed.as_millis(),
    };

    println!("{}", serde_json::to_string(&result).unwrap());
}
