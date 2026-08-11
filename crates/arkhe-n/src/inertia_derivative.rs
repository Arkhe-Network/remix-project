// src/inertia_derivative.rs
//! Aplicação do Lema 3.2 à derivada da confiança (análogo a ξ')

use crate::inertia_certifier::*;
use nalgebra::DMatrix;
use nalgebra::SymmetricEigen;

pub struct DerivativeCertifier {
    pub base: InertiaCertifier,
    pub threshold_simple_deriv: f64, // 0.86864 (quártica)
    pub threshold_distinct_deriv: f64, // 0.93432 (quártica)
}

pub struct DerivativeCertificate {
    pub s_simple_deriv: f64,
    pub s_distinct_deriv: f64,
    pub simple_achieved: bool,
    pub distinct_achieved: bool,
}

impl DerivativeCertifier {
    pub fn new() -> Self {
        Self {
            base: InertiaCertifier::new(),
            threshold_simple_deriv: 0.86864,
            threshold_distinct_deriv: 0.93432,
        }
    }

    /// Certifica a derivada da matriz de coerência
    pub fn certify_derivative(&self, matrix: &DMatrix<f64>, matrix_prev: &DMatrix<f64>, dt: f64) -> DerivativeCertificate {
        // Calcula a derivada numérica da matriz de coerência
        let deriv_matrix = (matrix - matrix_prev) / dt;

        let eigen = SymmetricEigen::new(deriv_matrix.clone());
        let vals = eigen.eigenvalues;
        let n = matrix.nrows() as f64;
        let trace = vals.sum();
        let frob_norm = vals.iter().map(|v| v.powi(2)).sum::<f64>().sqrt();

        let s_simple_deriv = (4.0 * trace - 2.0 * n - frob_norm.powi(2)).max(0.0) / n;
        let s_distinct_deriv = (0.5 * (4.0 * trace - n - frob_norm.powi(2))).max(0.0) / n;

        DerivativeCertificate {
            s_simple_deriv,
            s_distinct_deriv,
            simple_achieved: s_simple_deriv >= self.threshold_simple_deriv,
            distinct_achieved: s_distinct_deriv >= self.threshold_distinct_deriv,
        }
    }
}
