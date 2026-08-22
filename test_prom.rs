use prometheus::HistogramTimer;
fn check(timer: HistogramTimer) {
    let d = timer.observe_duration();
    let _: f64 = d; // See if this compiles
}
