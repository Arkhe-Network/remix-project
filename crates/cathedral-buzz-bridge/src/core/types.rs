use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchORState {
    pub timestamp: u64,
    pub coherence_time: f64,
    pub frequency: f64,
    pub energy: f64,
    pub hexagon_state: [u16; 12],
    pub regime: u8,
}

impl OrchORState {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::with_capacity(128);
        buf.extend_from_slice(&self.timestamp.to_le_bytes());
        buf.extend_from_slice(&self.coherence_time.to_le_bytes());
        buf.extend_from_slice(&self.frequency.to_le_bytes());
        buf.extend_from_slice(&self.energy.to_le_bytes());
        for &v in &self.hexagon_state {
            buf.extend_from_slice(&v.to_le_bytes());
        }
        buf.push(self.regime);
        buf
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self, anyhow::Error> {
        if bytes.len() < 57 {
            return Err(anyhow::anyhow!("Invalid state length"));
        }
        let mut offset = 0;
        let timestamp = u64::from_le_bytes(bytes[offset..offset+8].try_into()?);
        offset += 8;
        let coherence_time = f64::from_le_bytes(bytes[offset..offset+8].try_into()?);
        offset += 8;
        let frequency = f64::from_le_bytes(bytes[offset..offset+8].try_into()?);
        offset += 8;
        let energy = f64::from_le_bytes(bytes[offset..offset+8].try_into()?);
        offset += 8;
        let mut hexagon_state = [0u16; 12];
        for i in 0..12 {
            hexagon_state[i] = u16::from_le_bytes(bytes[offset..offset+2].try_into()?);
            offset += 2;
        }
        let regime = bytes[offset];
        Ok(OrchORState {
            timestamp, coherence_time, frequency, energy, hexagon_state, regime,
        })
    }
}
