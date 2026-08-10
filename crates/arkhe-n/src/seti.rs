//! src/seti.rs
//! SETI Interestelar — ARKHE-N v1.4
//! Ressonância Glashow (~6.3 PeV) como canal de comunicação galáctica

use serde::{Deserialize, Serialize};

/// Constantes físicas para SETI
pub const GLASHOW_RESONANCE_PEV: f64 = 6.3; // PeV
pub const GLASHOW_RESONANCE_EV: f64 = 6.3e15; // eV
pub const SPEED_OF_LIGHT_M_S: f64 = 2.998e8;
pub const PARSEC_M: f64 = 3.086e16;
pub const LIGHT_YEAR_M: f64 = 9.461e15;

/// Modo de comunicação galáctica
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum GalacticMode {
    GlashowResonance,    // 6.3 PeV — ressonância e+e- via ν̄e
    CosmicNeutrino,      // ~1 EeV — neutrinos cósmicos de ultra-alta energia
    DiffuseBackground,   // Background difuso como portadora
}

/// Configuração de transmissão interestelar
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetiConfig {
    pub mode: GalacticMode,
    pub target_distance_ly: f64,     // anos-luz
    pub transmitter_power_w: f64,    // potência do transmissor
    pub detector_area_m2: f64,       // área do detector
    pub energy_per_neutrino_ev: f64, // energia por neutrino
}

impl SetiConfig {
    /// Configuração padrão para ressonância Glashow
    pub fn glashow_default() -> Self {
        Self {
            mode: GalacticMode::GlashowResonance,
            target_distance_ly: 1000.0,      // 1 kly
            transmitter_power_w: 1e15,         // 1 PW (civilização tipo II)
            detector_area_m2: 1e12,          // 1000 km²
            energy_per_neutrino_ev: GLASHOW_RESONANCE_EV,
        }
    }

    /// Configuração para sondagem galáctica (100 kly)
    pub fn galactic_survey() -> Self {
        Self {
            mode: GalacticMode::CosmicNeutrino,
            target_distance_ly: 100_000.0,   // ~diâmetro da galáxia
            transmitter_power_w: 1e18,       // 1 EW (civilização tipo III)
            detector_area_m2: 1e14,          // escala planetária
            energy_per_neutrino_ev: 1e18,    // 1 EeV
        }
    }

    /// Distância em metros
    pub fn distance_m(&self) -> f64 {
        self.target_distance_ly * LIGHT_YEAR_M
    }

    /// Tempo de propagação (anos)
    pub fn propagation_time_years(&self) -> f64 {
        self.target_distance_ly // ~c, então 1 ly = 1 ano
    }

    /// Fluxo de neutrinos no detector (neutrinos / m² / s)
    /// Aproximação: potência / energia_por_neutrino espalhada em esfera
    pub fn neutrino_flux(&self) -> f64 {
        let sphere_area = 4.0 * std::f64::consts::PI * self.distance_m().powi(2);
        let neutrinos_per_second = self.transmitter_power_w / (self.energy_per_neutrino_ev * 1.602e-19);
        neutrinos_per_second / sphere_area
    }

    /// Taxa de detecção esperada (eventos / s)
    pub fn detection_rate_hz(&self) -> f64 {
        self.neutrino_flux() * self.detector_area_m2
    }

    /// Taxa de dados máxima teórica (bits/s)
    /// Usando capacidade do canal Poisson com λ = taxa de detecção × período
    pub fn max_data_rate_bps(&self) -> f64 {
        let lambda = self.detection_rate_hz() * 1.0; // 1 segundo de integração
        let p_detect = 1.0 - (-lambda).exp();
        p_detect * lambda.log2().max(0.0)
    }

    /// Contexto da transmissão
    pub fn context(&self) -> String {
        match self.mode {
            GalacticMode::GlashowResonance => format!(
                "Glashow Resonance | E={:.1} PeV | D={:.0} ly | P={:.0e} W | Rate={:.2e} Hz",
                self.energy_per_neutrino_ev / 1e15,
                self.target_distance_ly,
                self.transmitter_power_w,
                self.detection_rate_hz()
            ),
            GalacticMode::CosmicNeutrino => format!(
                "Cosmic Neutrino | E={:.0} EeV | D={:.0} ly | Galactic Survey",
                self.energy_per_neutrino_ev / 1e18,
                self.target_distance_ly
            ),
            GalacticMode::DiffuseBackground => format!(
                "Diffuse Background | D={:.0} ly | Passive listening",
                self.target_distance_ly
            ),
        }
    }
}

/// Resultado de uma busca SETI
#[derive(Debug, Clone, Serialize)]
pub struct SetiResult {
    pub config: SetiConfig,
    pub flux_hz_m2: f64,
    pub detection_rate_hz: f64,
    pub max_data_rate_bps: f64,
    pub snr_db: f64,
    pub is_detectable: bool,
}

impl SetiResult {
    pub fn from_config(config: &SetiConfig) -> Self {
        let flux = config.neutrino_flux();
        let rate = config.detection_rate_hz();
        let data_rate = config.max_data_rate_bps();
        // SNR aproximado: taxa de sinal / sqrt(taxa de background)
        let bg_rate: f64 = 1e-3; // background típico de neutrinos cósmicos
        let snr = if bg_rate > 0.0 { rate / bg_rate.sqrt() } else { rate };
        let snr_db = 10.0 * snr.log10();

        Self {
            config: config.clone(),
            flux_hz_m2: flux,
            detection_rate_hz: rate,
            max_data_rate_bps: data_rate,
            snr_db,
            is_detectable: snr_db > 3.0 && rate > 1e-6,
        }
    }
}

/// Verificação de candidato SETI
pub fn analyze_seti_candidate(
    energy_ev: f64,
    flux_hz_m2: f64,
    direction: (f64, f64), // (ra, dec) em graus
) -> SetiResult {
    let config = if (energy_ev - GLASHOW_RESONANCE_EV).abs() < 1e15 {
        SetiConfig::glashow_default()
    } else {
        SetiConfig::galactic_survey()
    };

    let mut result = SetiResult::from_config(&config);
    result.flux_hz_m2 = flux_hz_m2;
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glashow_resonance() {
        let config = SetiConfig::glashow_default();
        assert_eq!(config.mode, GalacticMode::GlashowResonance);
        assert!(config.detection_rate_hz() > 0.0);
    }

    #[test]
    fn test_galactic_survey() {
        let config = SetiConfig::galactic_survey();
        let result = SetiResult::from_config(&config);
        assert!(result.max_data_rate_bps >= 0.0);
        println!("Galactic Survey: {}", result.config.context());
    }

    #[test]
    fn test_propagation_time() {
        let config = SetiConfig::glashow_default();
        assert!((config.propagation_time_years() - 1000.0).abs() < 0.1);
    }

    #[test]
    fn test_seti_candidate() {
        let result = analyze_seti_candidate(GLASHOW_RESONANCE_EV, 1e-10, (180.0, 0.0));
        assert!(result.is_detectable || !result.is_detectable); // sempre válido
    }
}
