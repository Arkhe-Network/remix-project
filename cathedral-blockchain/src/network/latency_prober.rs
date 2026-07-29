#[cfg(feature = "std")]
pub struct RealisticLatencyProber;

#[cfg(feature = "std")]
impl RealisticLatencyProber {
    pub fn new() -> Self {
        Self
    }
}
