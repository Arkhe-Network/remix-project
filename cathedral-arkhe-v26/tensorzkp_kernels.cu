// tensorzkp_kernels.cu – BabyBear inner product, folding, Spielman encode e Merkle Tree
#include <stdint.h>

// BabyBear prime
const uint32_t P = (1ULL << 31) - (1ULL << 27) + 1;

// Redução modular rápida para BabyBear
__device__ __forceinline__ uint32_t mod_add(uint32_t a, uint32_t b) {
    uint32_t r = a + b;
    if (r >= P) r -= P;
    return r;
}

__device__ __forceinline__ uint32_t mod_mul(uint32_t a, uint32_t b) {
    uint64_t r = (uint64_t)a * b;
    r = (r & 0x7FFFFFFF) + (r >> 31);
    if (r >= P) r -= P;
    return (uint32_t)r;
}

// ==== KERNEL INNER PRODUCT (opcode 0x10) ====
extern "C" __global__ void inner_product_kernel(
    const uint32_t* __restrict__ vec_a,
    const uint32_t* __restrict__ vec_b,
    uint32_t* __restrict__ result,
    int n
) {
    __shared__ uint32_t smem[256];
    uint32_t sum = 0;
    for (int i = threadIdx.x + blockIdx.x * blockDim.x; i < n; i += blockDim.x * gridDim.x) {
        sum = mod_add(sum, mod_mul(vec_a[i], vec_b[i]));
    }
    smem[threadIdx.x] = sum;
    __syncthreads();
    for (int s = blockDim.x / 2; s > 0; s >>= 1) {
        if (threadIdx.x < s) {
            smem[threadIdx.x] = mod_add(smem[threadIdx.x], smem[threadIdx.x + s]);
        }
        __syncthreads();
    }
    if (threadIdx.x == 0) {
        result[blockIdx.x] = smem[0];
    }
}

// ==== KERNEL FOLDING (opcode 0x11) ====
extern "C" __global__ void fold_vector_kernel(
    const uint32_t* __restrict__ input,
    uint32_t* __restrict__ output,
    uint32_t scalar,
    int n
) {
    int idx = threadIdx.x + blockIdx.x * blockDim.x;
    if (idx < n) {
        output[idx] = mod_mul(input[idx], scalar);
    }
}

// ==== KERNEL SPIELMAN ENCODE (opcode 0x12) ====
extern "C" __global__ void spielman_encode_kernel(
    const uint32_t* __restrict__ row_ptr,
    const uint32_t* __restrict__ col_idx,
    const uint32_t* __restrict__ values,
    const uint32_t* __restrict__ vector,
    uint32_t* __restrict__ output,
    int rows,
    int cols
) {
    int row = blockIdx.x * blockDim.x + threadIdx.x;
    if (row >= rows) return;
    int start = row_ptr[row];
    int end = row_ptr[row + 1];
    uint32_t sum = 0;
    for (int j = start; j < end; ++j) {
        uint32_t col = col_idx[j];
        if (col < cols) {
            sum = mod_add(sum, mod_mul(values[j], vector[col]));
        }
    }
    output[row] = sum;
}

// ==== KERNEL MERKLE TREE (opcode 0x13) ====
__device__ uint32_t hash_pair(uint32_t a, uint32_t b) {
    uint32_t h = 5381;
    h = ((h << 5) + h) ^ a;
    h = ((h << 5) + h) ^ b;
    // Finalização com redução modular
    h = (h & 0x7FFFFFFF) + (h >> 31);
    if (h >= P) h -= P;
    return h;
}

extern "C" __global__ void merkle_level_kernel(
    const uint32_t* __restrict__ input,
    uint32_t* __restrict__ output,
    int len
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= len / 2) return;
    uint32_t left = input[2 * idx];
    uint32_t right = input[2 * idx + 1];
    output[idx] = hash_pair(left, right);
}
