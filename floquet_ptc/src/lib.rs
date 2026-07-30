//! Cathedral Arkhe v17.0 — Fotonic Time Crystal (L1 Physical Model)
//! Baseado no artigo de Guo, Sueiro, Andolina et al. (Nature, 29 Jul 2026)
//! EPISTEMIC STATUS: L1 (Modelo Físico Computável)

pub mod cavity;
pub mod floquet;
pub mod exceptional_point;

pub use cavity::{PlasmonicCavity, CarrierMassModulation};
pub use floquet::{FloquetState, FloquetHamiltonian};
pub use exceptional_point::{ExceptionalPointResult, PTCSignature};