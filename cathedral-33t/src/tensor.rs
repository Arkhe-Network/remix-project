//! Tensor backend concreto baseado em ndarray
//!
//! Suporta operações básicas para deep learning:
//! - Criação (zeros, ones, randn, from_vec)
//! - Operações aritméticas (add, sub, mul, div, matmul)
//! - Manipulação (slice, reshape, transpose, flatten)
//! - Ativações (sigmoid, relu, gelu, swiglu_clamp)
//! - Normalização (rms_norm, layer_norm)
//! - Redução (sum, mean, max, argmax)

use ndarray::{
    ArrayD, Axis, Ix1, Ix2, IxDyn,
    s,
};
use ndarray_rand::rand_distr::StandardNormal;
use ndarray_rand::RandomExt;
use rand::thread_rng;
use std::ops::{Add, Mul, Sub};

/// Tipo de dado para elementos do tensor
pub type TensorDtype = f32;

/// Tensor multi-dimensional baseado em ndarray
#[derive(Debug, Clone, PartialEq)]
pub struct Tensor {
    data: ArrayD<TensorDtype>,
}

/// Shape de um tensor
pub type Shape = Vec<usize>;

impl Tensor {
    // ==================== CRIAÇÃO ====================

    /// Cria tensor preenchido com zeros
    pub fn zeros(shape: &[usize]) -> Self {
        Self {
            data: ArrayD::zeros(IxDyn(shape)),
        }
    }

    /// Cria tensor preenchido com uns
    pub fn ones(shape: &[usize]) -> Self {
        Self {
            data: ArrayD::ones(IxDyn(shape)),
        }
    }

    /// Cria tensor com distribuição normal N(0, 1)
    pub fn randn(shape: &[usize]) -> Self {
        let mut rng = thread_rng();
        Self {
            data: ArrayD::random_using(IxDyn(shape), StandardNormal, &mut rng),
        }
    }

    /// Cria tensor a partir de um vetor 1D
    pub fn from_vec(vec: Vec<TensorDtype>, shape: &[usize]) -> Self {
        let expected_len: usize = shape.iter().product();
        assert_eq!(
            vec.len(),
            expected_len,
            "Tamanho do vetor ({}) não corresponde ao shape ({:?})",
            vec.len(),
            shape
        );
        Self {
            data: ArrayD::from_shape_vec(IxDyn(shape), vec)
                .expect("Shape inválido para o vetor fornecido"),
        }
    }

    /// Cria tensor escalar
    pub fn scalar(value: TensorDtype) -> Self {
        Self {
            data: ArrayD::from_elem(IxDyn(&[]), value),
        }
    }

    // ==================== PROPRIEDADES ====================

    /// Retorna o shape do tensor
    pub fn shape(&self) -> Vec<usize> {
        self.data.shape().to_vec()
    }

    /// Retorna o número de dimensões
    pub fn ndim(&self) -> usize {
        self.data.ndim()
    }

    /// Retorna o número total de elementos
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Verifica se o tensor está vazio
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    // ==================== ACESSO ====================

    /// Obtém elemento em índices específicos
    pub fn get(&self, indices: &[usize]) -> TensorDtype {
        self.data[indices].clone()
    }

    /// Define elemento em índices específicos
    pub fn set(&mut self, indices: &[usize], value: TensorDtype) {
        self.data[indices] = value;
    }

    /// Slice ao longo do primeiro eixo (batch)
    pub fn slice(&self, idx: usize) -> Self {
        let sliced = self.data.slice_axis(Axis(0), ndarray::Slice::new(idx as isize, Some((idx + 1) as isize), 1));
        Self {
            data: sliced.to_owned().into_dyn(),
        }
    }

    /// Slice genérico ao longo de um eixo
    pub fn slice_axis(&self, axis: usize, idx: usize) -> Self {
        let sliced = self.data.slice_axis(Axis(axis), ndarray::Slice::new(idx as isize, Some((idx + 1) as isize), 1));
        Self {
            data: sliced.to_owned().into_dyn(),
        }
    }

    /// Reshape do tensor
    pub fn reshape(&self, shape: &[usize]) -> Self {
        let expected_len: usize = shape.iter().product();
        assert_eq!(
            self.len(),
            expected_len,
            "Número de elementos ({}) não corresponde ao novo shape ({:?})",
            self.len(),
            shape
        );
        Self {
            data: self.data.clone().into_shape(IxDyn(shape))
                .expect("Reshape falhou"),
        }
    }

    /// Flatten para 1D
    pub fn flatten(&self) -> Self {
        Self {
            data: self.data.clone().into_shape(IxDyn(&[self.len()]))
                .expect("Flatten falhou"),
        }
    }

    /// Transpose (apenas 2D)
    pub fn t(&self) -> Self {
        assert_eq!(self.ndim(), 2, "Transpose só suporta 2D");
        let view2d = self.data.view().into_dimensionality::<Ix2>().unwrap();
        Self {
            data: view2d.t().to_owned().into_dyn(),
        }
    }

    /// Converte para vetor 1D
    pub fn to_vec(&self) -> Vec<TensorDtype> {
        self.data.iter().copied().collect()
    }

    /// View como Array2 (2D)
    pub fn as_array2(&self) -> ndarray::ArrayView2<TensorDtype> {
        self.data.view().into_dimensionality::<Ix2>()
            .expect("Tensor não é 2D")
    }

    /// View como Array1 (1D)
    pub fn as_array1(&self) -> ndarray::ArrayView1<TensorDtype> {
        self.data.view().into_dimensionality::<Ix1>()
            .expect("Tensor não é 1D")
    }

    // ==================== OPERAÇÕES ARITMÉTICAS ====================

    /// Multiplicação de matrizes (2D)
    pub fn matmul(&self, other: &Self) -> Self {
        assert_eq!(self.ndim(), 2, "matmul requer tensor 2D");
        assert_eq!(other.ndim(), 2, "matmul requer tensor 2D");
        let a = self.as_array2();
        let b = other.as_array2();
        let result = a.dot(&b);
        Self {
            data: result.into_dyn(),
        }
    }

    /// Multiplicação de matriz por vetor
    pub fn matmul_vec(&self, vec: &Self) -> Self {
        let a = self.as_array2();
        let b = vec.as_array1();
        let result = a.dot(&b);
        Self {
            data: result.into_dyn(),
        }
    }

    /// Produto elemento-a-elemento (Hadamard)
    pub fn mul_elem(&self, other: &Self) -> Self {
        Self {
            data: &self.data * &other.data,
        }
    }

    /// Divisão elemento-a-elemento
    pub fn div_elem(&self, other: &Self) -> Self {
        Self {
            data: &self.data / &other.data,
        }
    }

    /// Soma elemento-a-elemento
    pub fn add_elem(&self, other: &Self) -> Self {
        Self {
            data: &self.data + &other.data,
        }
    }

    /// Subtração elemento-a-elemento
    pub fn sub_elem(&self, other: &Self) -> Self {
        Self {
            data: &self.data - &other.data,
        }
    }

    /// Multiplicação por escalar
    pub fn scale(&self, scalar: TensorDtype) -> Self {
        Self {
            data: &self.data * scalar,
        }
    }

    /// Divisão por escalar
    pub fn div_scalar(&self, scalar: TensorDtype) -> Self {
        Self {
            data: &self.data / scalar,
        }
    }

    /// Soma por escalar
    pub fn add_scalar(&self, scalar: TensorDtype) -> Self {
        Self {
            data: &self.data + scalar,
        }
    }

    /// Clamping
    pub fn clamp(&self, min: TensorDtype, max: TensorDtype) -> Self {
        Self {
            data: self.data.mapv(|v| v.clamp(min, max)),
        }
    }

    /// Map elemento-a-elemento
    pub fn mapv(&self, f: impl Fn(TensorDtype) -> TensorDtype) -> Self {
        Self {
            data: self.data.mapv(f),
        }
    }

    // ==================== REDUÇÃO ====================

    /// Soma ao longo de um eixo
    pub fn sum_axis(&self, axis: usize) -> Self {
        Self {
            data: self.data.sum_axis(Axis(axis)),
        }
    }

    /// Média ao longo de um eixo
    pub fn mean_axis(&self, axis: usize) -> Self {
        let sum = self.sum_axis(axis);
        let count = self.shape()[axis] as TensorDtype;
        sum.scale(1.0 / count)
    }

    /// Soma de todos os elementos
    pub fn sum_all(&self) -> TensorDtype {
        self.data.sum()
    }

    /// Média de todos os elementos
    pub fn mean_all(&self) -> TensorDtype {
        self.data.mean().unwrap_or(0.0)
    }

    /// Máximo ao longo de um eixo
    pub fn max_axis(&self, axis: usize) -> Self {
        Self {
            data: self.data.map_axis(Axis(axis), |view| {
                view.iter().copied().fold(TensorDtype::NEG_INFINITY, TensorDtype::max)
            }),
        }
    }

    /// Argmax ao longo de um eixo
    pub fn argmax_axis(&self, axis: usize) -> Vec<usize> {
        self.data.axis_iter(Axis(axis))
            .map(|view| {
                view.iter()
                    .enumerate()
                    .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap())
                    .map(|(idx, _)| idx)
                    .unwrap_or(0)
            })
            .collect()
    }

    /// Top-k índices e valores ao longo do último eixo
    pub fn topk(&self, k: usize, axis: usize) -> Vec<(usize, TensorDtype)> {
        let mut result = Vec::new();
        for slice in self.data.axis_iter(Axis(axis)) {
            let mut indexed: Vec<(usize, TensorDtype)> = slice.iter()
                .enumerate()
                .map(|(i, &v)| (i, v))
                .collect();
            indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
            result.extend(indexed.into_iter().take(k));
        }
        result
    }

    // ==================== NORMALIZAÇÃO ====================

    /// RMS Normalization
    pub fn rms_norm(&self, eps: TensorDtype) -> Self {
        let mean_sq = self.data.mapv(|v| v * v).mean().unwrap_or(1.0);
        let norm = (mean_sq + eps).sqrt();
        self.scale(1.0 / norm)
    }

    /// Layer Normalization
    pub fn layer_norm(&self, eps: TensorDtype) -> Self {
        let mean = self.mean_all();
        let var = self.data.mapv(|v| (v - mean).powi(2)).mean().unwrap_or(1.0);
        let std = (var + eps).sqrt();
        self.mapv(|v| (v - mean) / std)
    }

    // ==================== ATIVAÇÕES ====================

    /// Sigmoid
    pub fn sigmoid(&self) -> Self {
        self.mapv(|v| 1.0 / (1.0 + (-v).exp()))
    }

    /// ReLU
    pub fn relu(&self) -> Self {
        self.mapv(|v| v.max(0.0))
    }

    /// GELU
    pub fn gelu(&self) -> Self {
        self.mapv(|v| {
            let cdf = 0.5 * (1.0 + (v * 0.7978845608 * (1.0 + 0.044715 * v * v)).tanh());
            v * cdf
        })
    }

    /// SwiGLU com clamping (supressão de outliers)
    pub fn swiglu_clamp(&self, gate: &Self, up: &Self, clamp_limit: TensorDtype) -> Self {
        let g = gate.clamp(-clamp_limit, clamp_limit);
        let u = up.clamp(-clamp_limit, clamp_limit);
        let sig_g = g.sigmoid();
        g.mul_elem(&sig_g).mul_elem(&u)
    }

    /// Softmax ao longo do último eixo
    pub fn softmax(&self, axis: usize) -> Self {
        let max_val = self.max_axis(axis);
        let shifted = self.sub_elem(&max_val);
        let exp = shifted.mapv(|v| v.exp());
        let sum_exp = exp.sum_axis(axis);
        exp.div_elem(&sum_exp)
    }

    // ==================== CONCATENAÇÃO ====================

    /// Concatena ao longo de um eixo
    pub fn concat(tensors: &[&Self], axis: usize) -> Self {
        let arrays: Vec<_> = tensors.iter().map(|t| t.data.view()).collect();
        Self {
            data: ndarray::concatenate(Axis(axis), &arrays)
                .expect("Concatenação falhou")
                .into_owned()
                .into_dyn(),
        }
    }

    // ==================== BROADCASTING ====================

    /// Broadcasting add
    pub fn broadcast_add(&self, other: &Self) -> Self {
        Self {
            data: &self.data + &other.data,
        }
    }

    /// Broadcasting mul
    pub fn broadcast_mul(&self, other: &Self) -> Self {
        Self {
            data: &self.data * &other.data,
        }
    }
}

// ==================== OPERADORES ====================

impl Add for &Tensor {
    type Output = Tensor;
    fn add(self, other: &Tensor) -> Tensor {
        self.add_elem(other)
    }
}

impl Sub for &Tensor {
    type Output = Tensor;
    fn sub(self, other: &Tensor) -> Tensor {
        self.sub_elem(other)
    }
}

impl Mul for &Tensor {
    type Output = Tensor;
    fn mul(self, other: &Tensor) -> Tensor {
        self.mul_elem(other)
    }
}

impl Add<TensorDtype> for &Tensor {
    type Output = Tensor;
    fn add(self, scalar: TensorDtype) -> Tensor {
        self.add_scalar(scalar)
    }
}

impl Mul<TensorDtype> for &Tensor {
    type Output = Tensor;
    fn mul(self, scalar: TensorDtype) -> Tensor {
        self.scale(scalar)
    }
}

// ==================== TESTES ====================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_creation() {
        let t = Tensor::zeros(&[2, 3]);
        assert_eq!(t.shape(), vec![2, 3]);
        assert_eq!(t.sum_all(), 0.0);
    }

    #[test]
    fn test_matmul() {
        let a = Tensor::from_vec(vec![1.0, 2.0, 3.0, 4.0], &[2, 2]);
        let b = Tensor::from_vec(vec![5.0, 6.0, 7.0, 8.0], &[2, 2]);
        let c = a.matmul(&b);
        assert_eq!(c.shape(), vec![2, 2]);
        assert!((c.get(&[0, 0]) - 19.0).abs() < 1e-6);
    }

    #[test]
    fn test_sigmoid() {
        let t = Tensor::zeros(&[1, 1]);
        let s = t.sigmoid();
        assert!((s.get(&[0, 0]) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_clamp() {
        let t = Tensor::from_vec(vec![-20.0, -5.0, 0.0, 5.0, 20.0], &[5]);
        let c = t.clamp(-10.0, 10.0);
        assert_eq!(c.get(&[0]), -10.0);
        assert_eq!(c.get(&[1]), -5.0);
        assert_eq!(c.get(&[4]), 10.0);
    }

    #[test]
    fn test_rms_norm() {
        let t = Tensor::from_vec(vec![1.0, 2.0, 3.0], &[3]);
        let n = t.rms_norm(1e-6);
        let expected = (14.0f32 / 3.0).sqrt();
        assert!((n.get(&[0]) - 1.0 / expected).abs() < 1e-5);
    }

    #[test]
    fn test_reshape() {
        let t = Tensor::from_vec(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], &[2, 3]);
        let r = t.reshape(&[3, 2]);
        assert_eq!(r.shape(), vec![3, 2]);
        assert_eq!(r.get(&[0, 0]), 1.0);
    }

    #[test]
    fn test_slice() {
        let t = Tensor::from_vec(vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0], &[2, 3]);
        let s = t.slice(0);
        assert_eq!(s.shape(), vec![1, 3]);
        assert_eq!(s.get(&[0, 0]), 1.0);
        assert_eq!(s.get(&[0, 2]), 3.0);
    }

    #[test]
    fn test_softmax() {
        let t = Tensor::from_vec(vec![1.0, 2.0, 3.0], &[1, 3]);
        let s = t.softmax(1);
        let sum = s.sum_all();
        assert!((sum - 1.0).abs() < 1e-5);
    }
}