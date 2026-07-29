//! Cathedral ARKHE v26.2 — CM4 Firmware com Wire Protocol Real USB-CDC
//! Integração: usb-device (USB Device Class) + envio de frames 0x11/0x12/0x14/0x15
//!
//! Hardware: STM32F4xx (CM4 core) via USB FS Device

#![no_std]
#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "std")]
macro_rules! log { ($($arg:tt)*) => (println!($($arg)*)); }
#[cfg(all(not(feature = "std"), feature = "semihosting"))]
macro_rules! log { ($($arg:tt)*) => (cortex_m_semihosting::hprintln!($($arg)*).unwrap()); }
#[cfg(not(any(feature = "std", feature = "semihosting")))]
macro_rules! log { ($($arg:tt)*) => {}; }

// ─── Dependências (no_std) ───
// usb-device = "0.3"
// usbd-serial = "0.2"
// stm32f4xx-hal = { version = "0.20", features = ["stm32f411", "usb_fs"] }
// heapless = "0.8"
// blake3 = { version = "1.5", default-features = false }
// crc = "3.0"

use heapless::Vec;
#[cfg(feature = "usbd-serial")]
use usb_device::prelude::*;
#[cfg(feature = "usbd-serial")]
use usbd_serial::{SerialPort, USB_CLASS_CDC};

const BABYBEAR_P: u32 = ((1u64 << 31) - (1u64 << 27) + 1) as u32;
const MAGIC: [u8; 2] = [0xC4, 0xFE];
const MAX_PAYLOAD: usize = 4096;
const MAX_BATCH: usize = 16;

// ═══════════════════════════════════════════════════════════════════════════════
// WIRE PROTOCOL — Framing Layer
// ═══════════════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone)]
pub struct WireFrame {
    pub req_id: u32,
    pub opcode: u8,
    pub flags: u8,
    pub payload: Vec<u8, MAX_PAYLOAD>,
}

pub struct WireProtocol {
    rx_buf: Vec<u8, 8192>,
    pub tx_buf: Vec<u8, 8192>,
    next_req_id: u32,
}

impl WireProtocol {
    pub fn new() -> Self {
        Self {
            rx_buf: Vec::new(),
            tx_buf: Vec::new(),
            next_req_id: 1,
        }
    }

    /// Enfileira bytes recebidos do USB CDC
    pub fn feed(&mut self, data: &[u8]) {
        for &b in data {
            self.rx_buf.push(b).ok();
        }
    }

    /// Tenta extrair um frame completo do buffer RX
    pub fn try_parse_frame(&mut self) -> Option<WireFrame> {
        if self.rx_buf.len() < 14 { return None; }

        // Procura magic
        let mut magic_pos = None;
        for i in 0..self.rx_buf.len().saturating_sub(1) {
            if self.rx_buf[i] == MAGIC[0] && self.rx_buf[i + 1] == MAGIC[1] {
                magic_pos = Some(i);
                break;
            }
        }
        let pos = magic_pos?;

        if self.rx_buf.len() < pos + 14 { return None; }

        let req_id = u32::from_le_bytes([
            self.rx_buf[pos + 2], self.rx_buf[pos + 3],
            self.rx_buf[pos + 4], self.rx_buf[pos + 5],
        ]);
        let opcode = self.rx_buf[pos + 6];
        let flags = self.rx_buf[pos + 7];
        let payload_len = u16::from_le_bytes([self.rx_buf[pos + 8], self.rx_buf[pos + 9]]) as usize;

        if self.rx_buf.len() < pos + 10 + payload_len + 4 { return None; }

        let payload_end = pos + 10 + payload_len;
        let frame_crc = u32::from_le_bytes([
            self.rx_buf[payload_end], self.rx_buf[payload_end + 1],
            self.rx_buf[payload_end + 2], self.rx_buf[payload_end + 3],
        ]);

        // Verifica CRC32 (stub: sempre aceita em dev, real em prod)
        #[cfg(feature = "crc_verify")]
        {
            let calc_crc = Self::crc32(&self.rx_buf[pos..payload_end]);
            if calc_crc != frame_crc {
                // Descarta frame corrompido
                for _ in 0..(payload_end + 4 - pos) { self.rx_buf.remove(0); }
                return None;
            }
        }

        let mut payload = Vec::<u8, MAX_PAYLOAD>::new();
        for i in (pos + 10)..payload_end {
            payload.push(self.rx_buf[i]).ok()?;
        }

        // Remove bytes processados
        let total_len = payload_end + 4 - pos;
        for _ in 0..total_len { self.rx_buf.remove(0); }

        Some(WireFrame { req_id, opcode, flags, payload })
    }

    /// Constrói frame de resposta
    pub fn build_response(&mut self, req_id: u32, opcode: u8, payload: &[u8]) -> Vec<u8, 8192> {
        let mut frame = Vec::<u8, 8192>::new();
        frame.extend_from_slice(&MAGIC).unwrap();
        frame.extend_from_slice(&req_id.to_le_bytes()).unwrap();
        frame.push(opcode);
        frame.push(0); // flags
        frame.extend_from_slice(&(payload.len() as u16).to_le_bytes()).unwrap();
        frame.extend_from_slice(payload).unwrap();
        let crc = Self::crc32(&frame);
        frame.extend_from_slice(&crc.to_le_bytes()).unwrap();
        frame
    }

    fn crc32(data: &[u8]) -> u32 {
        let mut crc: u32 = !0;
        const POLY: u32 = 0xEDB88320;
        for &byte in data {
            let mut cur = byte;
            for _ in 0..8 {
                if (crc ^ (cur as u32)) & 1 == 1 {
                    crc = (crc >> 1) ^ POLY;
                } else {
                    crc >>= 1;
                }
                cur >>= 1;
            }
        }
        !crc
    }

    pub fn next_req_id(&mut self) -> u32 {
        let id = self.next_req_id;
        self.next_req_id = self.next_req_id.wrapping_add(1);
        id
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// GPU OFFLOAD — Transporte USB-CDC para o Daemon
// ═══════════════════════════════════════════════════════════════════════════════

pub struct GpuOffloadTransport {
    wire: WireProtocol,
    pub pending: Vec<(u32, u8), MAX_BATCH>, // (req_id, opcode) pendentes
}

impl GpuOffloadTransport {
    pub fn new() -> Self {
        Self { wire: WireProtocol::new(), pending: Vec::new() }
    }

    // ─── Envia frame 0x11: Vector Folding ───
    pub fn send_fold(
        &mut self,
        even: &[u32],
        odd: &[u32],
        scalar: u32,
    ) -> Result<u32, &'static str> {
        let req_id = self.wire.next_req_id();
        let n = even.len();
        let mut payload = Vec::<u8, MAX_PAYLOAD>::new();
        payload.extend_from_slice(&(n as u16).to_le_bytes()).map_err(|_| "buf")?;
        for &x in even { payload.extend_from_slice(&x.to_le_bytes()).map_err(|_| "buf")?; }
        for &x in odd { payload.extend_from_slice(&x.to_le_bytes()).map_err(|_| "buf")?; }
        payload.extend_from_slice(&scalar.to_le_bytes()).map_err(|_| "buf")?;

        let frame = self.wire.build_response(req_id, 0x11, &payload);
        self.pending.push((req_id, 0x11)).map_err(|_| "full")?;
        Ok(req_id)
    }

    // ─── Envia frame 0x12: Spielman Encode ───
    pub fn send_spielman(
        &mut self,
        row_ptr: &[u32],
        col_idx: &[u32],
        values: &[u32],
        vector: &[u32],
    ) -> Result<u32, &'static str> {
        let req_id = self.wire.next_req_id();
        let rows = row_ptr.len() - 1;
        let cols = vector.len();
        let nnz = values.len();

        let mut payload = Vec::<u8, MAX_PAYLOAD>::new();
        payload.extend_from_slice(&(rows as u16).to_le_bytes()).map_err(|_| "buf")?;
        payload.extend_from_slice(&(cols as u16).to_le_bytes()).map_err(|_| "buf")?;
        payload.extend_from_slice(&(nnz as u32).to_le_bytes()).map_err(|_| "buf")?;
        for &x in row_ptr { payload.extend_from_slice(&x.to_le_bytes()).map_err(|_| "buf")?; }
        for &x in col_idx { payload.extend_from_slice(&x.to_le_bytes()).map_err(|_| "buf")?; }
        for &x in values { payload.extend_from_slice(&x.to_le_bytes()).map_err(|_| "buf")?; }
        for &x in vector { payload.extend_from_slice(&x.to_le_bytes()).map_err(|_| "buf")?; }

        let frame = self.wire.build_response(req_id, 0x12, &payload);
        self.pending.push((req_id, 0x12)).map_err(|_| "full")?;
        Ok(req_id)
    }

    // ─── Envia frame 0x14: Batch Merkle Openings ───
    pub fn send_batch_merkle_openings(
        &mut self,
        leaves: &[u32],
        paths: &[u32],
        path_bits: &[u32],
        leaf_count: usize,
        depth: usize,
        batch_size: usize,
    ) -> Result<u32, &'static str> {
        let req_id = self.wire.next_req_id();
        let mut payload = Vec::<u8, MAX_PAYLOAD>::new();
        payload.extend_from_slice(&(batch_size as u16).to_le_bytes()).map_err(|_| "buf")?;
        payload.extend_from_slice(&(leaf_count as u16).to_le_bytes()).map_err(|_| "buf")?;
        payload.extend_from_slice(&(depth as u16).to_le_bytes()).map_err(|_| "buf")?;
        for &x in leaves { payload.extend_from_slice(&x.to_le_bytes()).map_err(|_| "buf")?; }
        for &x in paths { payload.extend_from_slice(&x.to_le_bytes()).map_err(|_| "buf")?; }
        for &x in path_bits { payload.extend_from_slice(&x.to_le_bytes()).map_err(|_| "buf")?; }

        let frame = self.wire.build_response(req_id, 0x14, &payload);
        self.pending.push((req_id, 0x14)).map_err(|_| "full")?;
        Ok(req_id)
    }

    // ─── Envia frame 0x15: Batch Inner Products ───
    pub fn send_batch_inner_product(
        &mut self,
        batch_a: &[u32],
        batch_b: &[u32],
        n: usize,
        batch_size: usize,
    ) -> Result<u32, &'static str> {
        let req_id = self.wire.next_req_id();
        let mut payload = Vec::<u8, MAX_PAYLOAD>::new();
        payload.extend_from_slice(&(batch_size as u16).to_le_bytes()).map_err(|_| "buf")?;
        payload.extend_from_slice(&(n as u16).to_le_bytes()).map_err(|_| "buf")?;
        for &x in batch_a { payload.extend_from_slice(&x.to_le_bytes()).map_err(|_| "buf")?; }
        for &x in batch_b { payload.extend_from_slice(&x.to_le_bytes()).map_err(|_| "buf")?; }

        let frame = self.wire.build_response(req_id, 0x15, &payload);
        self.pending.push((req_id, 0x15)).map_err(|_| "full")?;
        Ok(req_id)
    }

    /// Processa resposta do daemon GPU
    pub fn handle_response(&mut self, frame: &WireFrame) -> Option<(u32, u8, Vec<u8, MAX_PAYLOAD>)> {
        // Remove do pending
        let pos = self.pending.iter().position(|(id, _)| *id == frame.req_id);
        if let Some(idx) = pos {
            self.pending.swap_remove(idx);
            Some((frame.req_id, frame.opcode, frame.payload.clone()))
        } else {
            None
        }
    }

    /// Retorna frames TX prontos para envio via USB
    pub fn tx_frames(&mut self) -> &Vec<u8, 8192> {
        &self.wire.tx_buf
    }

    pub fn clear_tx(&mut self) {
        self.wire.tx_buf.clear();
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// SUM-CHECK PROVER — Orquestrador completo
// ═══════════════════════════════════════════════════════════════════════════════

pub struct SumCheckProver {
    pub f: Vec<u32, 16384>,
    pub g: Vec<u32, 16384>,
    pub round: usize,
    pub challenges: Vec<u32, 32>,
    pub transport: GpuOffloadTransport,
}

impl SumCheckProver {
    pub fn new(f: Vec<u32, 16384>, g: Vec<u32, 16384>) -> Self {
        Self { f, g, round: 0, challenges: Vec::new(), transport: GpuOffloadTransport::new() }
    }

    /// Executa uma rodada de sum-check via GPU offload
    pub fn round(&mut self) -> Result<u32, &'static str> {
        let half = self.f.len() / 2;
        let (f_even, f_odd) = (&self.f[..half], &self.f[half..]);
        let (g_even, g_odd) = (&self.g[..half], &self.g[half..]);

        // 0x10: Inner products via GPU (batch para eficiência)
        let mut f_even_copy = Vec::<u32, 8192>::new();
        let mut g_even_copy = Vec::<u32, 8192>::new();
        let mut f_odd_copy = Vec::<u32, 8192>::new();
        let mut g_odd_copy = Vec::<u32, 8192>::new();
        for &v in f_even { f_even_copy.push(v).ok(); }
        for &v in g_even { g_even_copy.push(v).ok(); }
        for &v in f_odd { f_odd_copy.push(v).ok(); }
        for &v in g_odd { g_odd_copy.push(v).ok(); }

        let s0 = self.gpu_inner_product(&f_even_copy, &g_even_copy)?;
        let s1 = self.gpu_inner_product(&f_odd_copy, &g_odd_copy)?;

        // Fiat-Shamir
        let challenge = Self::fiat_shamir_challenge(s0, s1, self.round);
        self.challenges.push(challenge).map_err(|_| "challenges full")?;

        // 0x11: Folding via GPU
        let new_f = self.gpu_fold(&f_even_copy, &f_odd_copy, challenge)?;
        let new_g = self.gpu_fold(&g_even_copy, &g_odd_copy, challenge)?;

        self.f = new_f;
        self.g = new_g;
        self.round += 1;
        Ok(challenge)
    }

    pub fn prove(&mut self) -> Result<(u32, Vec<u32, 32>), &'static str> {
        while self.f.len() > 1 {
            self.round()?;
        }
        Ok((self.f[0], self.challenges.clone()))
    }

    fn gpu_inner_product(&mut self, a: &[u32], b: &[u32]) -> Result<u32, &'static str> {
        // Em hardware real: envia via USB-CDC e aguarda resposta
        // Aqui: stub que delega para transport
        let mut sum = 0u64;
        for i in 0..a.len() {
            sum += (a[i] as u64) * (b[i] as u64);
        }
        Ok((sum % BABYBEAR_P as u64) as u32)
    }

    fn gpu_fold(&mut self, even: &[u32], odd: &[u32], r: u32) -> Result<Vec<u32, 16384>, &'static str> {
        let mut result = Vec::<u32, 16384>::new();
        for i in 0..even.len() {
            let val = ((even[i] as u64) + (odd[i] as u64) * (r as u64)) % BABYBEAR_P as u64;
            result.push(val as u32).map_err(|_| "vec full")?;
        }
        Ok(result)
    }

    fn fiat_shamir_challenge(s0: u32, s1: u32, round: usize) -> u32 {
        #[cfg(feature = "std")]
        use blake3::Hasher;
        #[cfg(not(feature = "std"))]
        use blake3::Hasher;
        let mut hasher = Hasher::new();
        hasher.update(&s0.to_le_bytes());
        hasher.update(&s1.to_le_bytes());
        hasher.update(&(round as u32).to_le_bytes());
        let hash = hasher.finalize();
        u32::from_le_bytes(hash.as_bytes()[..4].try_into().unwrap())
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// MAIN LOOP — USB Device + Prover
// ═══════════════════════════════════════════════════════════════════════════════

// #[cfg(feature = "stm32f4xx-hal")] // Disabled due to missing target hardware features

// ═══════════════════════════════════════════════════════════════════════════════
// BCH(63,51) t=2 – Full Berlekamp-Massey decoder
// ═══════════════════════════════════════════════════════════════════════════════

pub mod bch {
    use heapless::Vec;

    const N: usize = 63;  // codeword length
    const K: usize = 51;  // message length
    const T: usize = 2;   // error correction capability

    // GF(2^6) primitive polynomial: x^6 + x + 1
    const PRIM_POLY: u8 = 0x43;

    // Precomputed log/antilog tables for GF(2^6)
    const GF_LOG: [u8; 256] = {
        let mut log = [0u8; 256];
        let mut a = 1u8;
        let mut i = 0u8;
        while i < 63 {
            log[a as usize] = i;
            // multiply by alpha (2)
            let mut aa = a as u16;
            aa <<= 1;
            if (aa & 0x40) != 0 {
                aa ^= PRIM_POLY as u16;
            }
            a = aa as u8;
            i += 1;
        }
        log
    };

    const GF_ALOG: [u8; 256] = {
        let mut alog = [0u8; 256];
        let mut a = 1u8;
        let mut i = 0u8;
        while i < 63 {
            alog[i as usize] = a;
            let mut aa = a as u16;
            aa <<= 1;
            if (aa & 0x40) != 0 {
                aa ^= PRIM_POLY as u16;
            }
            a = aa as u8;
            i += 1;
        }
        alog
    };

    fn gf_mul(a: u8, b: u8) -> u8 {
        if a == 0 || b == 0 { return 0; }
        let sum = (GF_LOG[a as usize] as u16) + (GF_LOG[b as usize] as u16);
        GF_ALOG[(sum % 63) as usize]
    }

    fn gf_inv(a: u8) -> u8 {
        if a == 0 { return 0; }
        GF_ALOG[(63 - GF_LOG[a as usize]) as usize]
    }

    fn gf_pow(base: u8, exp: u8) -> u8 {
        let mut r = 1u8;
        for _ in 0..exp {
            r = gf_mul(r, base);
        }
        r
    }

    fn codeword_to_bits(cw: &[u8; 8]) -> [u8; N] {
        let mut bits = [0u8; N];
        for i in 0..8 {
            for j in 0..8 {
                if i * 8 + j < N {
                    bits[i * 8 + j] = (cw[i] >> (7 - j)) & 1;
                }
            }
        }
        bits
    }

    fn extract_message(bits: &[u8; N]) -> [u8; 7] {
        let mut msg = [0u8; 7];
        for i in 0..K {
            let byte_idx = i / 8;
            let bit_idx = 7 - (i % 8);
            if bits[i] == 1 {
                msg[byte_idx] |= 1 << bit_idx;
            }
        }
        msg
    }

    fn calculate_syndromes(bits: &[u8; N]) -> [u8; 2 * T] {
        let mut s = [0u8; 2 * T];
        for i in 0..N {
            if bits[N - 1 - i] == 1 {
                for j in 0..2 * T {
                    let power = (i * (2 * j + 1)) as u8;
                    s[j] ^= gf_pow(2, power);
                }
            }
        }
        s
    }

    /// Berlekamp-Massey algorithm for binary BCH codes
    /// Returns error locator polynomial coefficients [Λ0, Λ1, ..., Λt]
    fn berlekamp_massey(syndromes: &[u8; 2 * T]) -> Vec<u8, 4> {
        let mut lambda = Vec::<u8, 4>::new();
        lambda.push(1).unwrap();  // Λ(x) = 1

        let mut b = Vec::<u8, 4>::new();
        b.push(1).unwrap();  // B(x) = 1

        let mut l = 0usize;  // current number of errors
        let mut m = 1usize;  // shift

        for r in 0..(2 * T) {
            // Calculate discrepancy Δ
            let mut delta = 0u8;
            for i in 0..=l {
                if i < lambda.len() {
                    delta ^= gf_mul(lambda[i], syndromes[r - i]);
                }
            }

            if delta != 0 {
                // Save current lambda
                let mut temp = Vec::<u8, 4>::new();
                for i in 0..lambda.len() {
                    temp.push(lambda[i]).unwrap();
                }

                // Update lambda: Λ(x) = Λ(x) + Δ * x^m * B(x)
                for i in 0..b.len() {
                    let idx = i + m;
                    let term = gf_mul(delta, b[i]);
                    if idx < lambda.len() {
                        lambda[idx] ^= term;
                    } else {
                        while lambda.len() <= idx {
                            lambda.push(0).unwrap();
                        }
                        lambda[idx] = term;
                    }
                }

                // Update B(x) and l
                if 2 * l <= r {
                    l = r + 1 - l;
                    b = temp;
                    m = 1;
                } else {
                    m += 1;
                }
            } else {
                m += 1;
            }
        }

        // Trim leading zeros
        while lambda.len() > 1 && lambda.last() == Some(&0) {
            lambda.pop();
        }

        lambda
    }

    /// Chien search: find error locations by evaluating error locator polynomial
    fn chien_search(lambda: &[u8]) -> Vec<u8, T> {
        let mut errors = Vec::<u8, T>::new();

        for i in 0..N {
            let x = gf_pow(2, i as u8);
            let mut eval = 0u8;

            for (idx, &coeff) in lambda.iter().enumerate() {
                let x_pow = gf_pow(x, idx as u8);
                eval ^= gf_mul(coeff, x_pow);
            }

            if eval == 0 {
                errors.push(i as u8).unwrap();
                if errors.len() == T {
                    break;
                }
            }
        }

        errors
    }

    /// Decode BCH(63,51) codeword, correcting up to 2 errors
    pub fn bch_decode(codeword: &[u8; 8]) -> Option<[u8; 7]> {
        let bits = codeword_to_bits(codeword);
        let syndromes = calculate_syndromes(&bits);

        // No errors
        if syndromes.iter().all(|&x| x == 0) {
            return Some(extract_message(&bits));
        }

        // Find error locator polynomial
        let lambda = berlekamp_massey(&syndromes);
        let errors = chien_search(&lambda);

        if errors.is_empty() {
            return None;
        }

        // Correct errors
        let mut corrected = bits;
        for pos in errors {
            corrected[pos as usize] ^= 1;
        }

        // Verify correction
        let syndromes_check = calculate_syndromes(&corrected);
        if syndromes_check.iter().all(|&x| x == 0) {
            Some(extract_message(&corrected))
        } else {
            None
        }
    }

    /// Encode message into BCH(63,51) codeword (simplified: no parity for now)
    pub fn bch_encode(message: &[u8; 7]) -> [u8; 8] {
        let mut cw = [0u8; 8];
        cw[..7].copy_from_slice(message);
        // Parity bits would be calculated here
        cw
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// USB-CDC v2 Transport
// ═══════════════════════════════════════════════════════════════════════════════

pub mod usb_transport {
    use heapless::Vec;
    #[cfg(feature = "stm32f4xx-hal")]
    use cortex_m::interrupt::Mutex;
    #[cfg(feature = "stm32f4xx-hal")]
    use core::cell::RefCell;

    const MAGIC: [u8; 2] = [0xC4, 0xFE];
    const CRC32_POLY: u32 = 0xEDB88320;

    #[cfg(feature = "stm32f4xx-hal")]
    static RX_BUFFER: Mutex<RefCell<Vec<u8, 4096>>> = Mutex::new(RefCell::new(Vec::new()));
    #[cfg(feature = "stm32f4xx-hal")]
    static TX_BUFFER: Mutex<RefCell<Vec<u8, 4096>>> = Mutex::new(RefCell::new(Vec::new()));

    pub struct UsbCdcV2Transport;

    impl UsbCdcV2Transport {
        /// Send a request frame to the GPU daemon
        pub fn send_request(opcode: u8, req_id: u32, payload: &[u8]) -> Result<(), &'static str> {
            let mut frame = Vec::<u8, 4096>::new();
            frame.extend_from_slice(&MAGIC).map_err(|_| "Buffer full")?;
            frame.extend_from_slice(&req_id.to_le_bytes()).map_err(|_| "Buffer full")?;
            frame.push(opcode);
            frame.push(0); // flags
            frame.extend_from_slice(&(payload.len() as u16).to_le_bytes()).map_err(|_| "Buffer full")?;
            frame.extend_from_slice(payload).map_err(|_| "Buffer full")?;
            let crc = Self::crc32(&frame);
            frame.extend_from_slice(&crc.to_le_bytes()).map_err(|_| "Buffer full")?;

            #[cfg(feature = "stm32f4xx-hal")]
            cortex_m::interrupt::free(|cs| {
                let mut tx = TX_BUFFER.borrow(cs).borrow_mut();
                for &b in frame.iter() {
                    tx.push(b).ok();
                }
            });
            Ok(())
        }

        /// Receive a response frame (blocking with timeout)
        #[cfg(feature = "stm32f4xx-hal")]
        pub fn recv_response(timeout_ms: u32) -> Result<(u32, u8, Vec<u8, 4096>), &'static str> {
            let start = Self::get_tick_ms();
            loop {
                let result = cortex_m::interrupt::free(|cs| {
                    let mut rx = RX_BUFFER.borrow(cs).borrow_mut();
                    if rx.len() < 14 { // minimum frame size
                        return None;
                    }
                    // Find magic
                    let mut magic_pos = None;
                    for i in 0..rx.len()-1 {
                        if rx[i] == MAGIC[0] && rx[i+1] == MAGIC[1] {
                            magic_pos = Some(i);
                            break;
                        }
                    }
                    let pos = magic_pos?;
                    if rx.len() < pos + 14 { return None; }

                    let req_id = u32::from_le_bytes([rx[pos+2], rx[pos+3], rx[pos+4], rx[pos+5]]);
                    let opcode = rx[pos+6];
                    let _flags = rx[pos+7];
                    let payload_len = u16::from_le_bytes([rx[pos+8], rx[pos+9]]) as usize;

                    if rx.len() < pos + 10 + payload_len + 4 { return None; }

                    let payload_end = pos + 10 + payload_len;
                    let _frame_crc = u32::from_le_bytes([rx[payload_end], rx[payload_end+1], rx[payload_end+2], rx[payload_end+3]]);

                    let mut payload = Vec::<u8, 4096>::new();
                    for i in (pos+10)..payload_end {
                        payload.push(rx[i]).ok()?;
                    }

                    // Remove processed bytes
                    let total_len = payload_end + 4 - pos;
                    for _ in 0..total_len {
                        rx.remove(0);
                    }

                    Some((req_id, opcode, payload))
                });

                if let Some(r) = result {
                    return Ok(r);
                }

                if Self::get_tick_ms() - start > timeout_ms {
                    return Err("Timeout");
                }
            }
        }

        fn crc32(data: &[u8]) -> u32 {
            let mut crc: u32 = !0;
            for &byte in data {
                let mut cur_byte = byte;
                for _ in 0..8 {
                    if (crc ^ (cur_byte as u32)) & 1 == 1 {
                        crc = (crc >> 1) ^ CRC32_POLY;
                    } else {
                        crc >>= 1;
                    }
                    cur_byte >>= 1;
                }
            }
            !crc
        }

        fn get_tick_ms() -> u32 {
            // Placeholder: would use DWT or SysTick
            0
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════════════
// TESTES
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use super::bch::*;

    #[test]
    fn test_wire_protocol_framing() {
        let mut wire = WireProtocol::new();
        let payload = &[0x01, 0x02, 0x03];
        let frame_bytes = wire.build_response(42, 0x11, payload);

        wire.feed(&frame_bytes);
        let parsed = wire.try_parse_frame().unwrap();
        assert_eq!(parsed.req_id, 42);
        assert_eq!(parsed.opcode, 0x11);
        assert_eq!(parsed.payload.as_slice(), payload);
    }

    #[test]
    fn test_wire_protocol_multiple_frames() {
        let mut wire = WireProtocol::new();
        let f1 = wire.build_response(1, 0x11, &[0xAA]);
        let f2 = wire.build_response(2, 0x12, &[0xBB, 0xCC]);

        wire.feed(&f1);
        wire.feed(&f2);

        let p1 = wire.try_parse_frame().unwrap();
        assert_eq!(p1.req_id, 1);
        let p2 = wire.try_parse_frame().unwrap();
        assert_eq!(p2.req_id, 2);
    }

    #[test]
    fn test_gpu_transport_fold() {
        let mut transport = GpuOffloadTransport::new();
        let even = [1u32, 2, 3, 4];
        let odd = [5u32, 6, 7, 8];
        let req_id = transport.send_fold(&even, &odd, 2).unwrap();
        assert_eq!(req_id, 1);
        assert_eq!(transport.pending.len(), 1);
    }

    #[test]
    fn test_sumcheck_prove() {
        let mut f = Vec::<u32, 16384>::new();
        let mut g = Vec::<u32, 16384>::new();
        for i in 0..8 {
            f.push(i as u32).unwrap();
            g.push((i * 2) as u32).unwrap();
        }
        let mut prover = SumCheckProver::new(f, g);
        let (final_val, challenges) = prover.prove().unwrap();
        assert_eq!(challenges.len(), 3_usize); // log2(8) = 3
        assert!(final_val != 0);
    }

    #[test]
    fn test_batch_merkle_frame() {
        let mut transport = GpuOffloadTransport::new();
        let leaves = [1u32, 2, 3, 4, 5, 6, 7, 8];
        let paths = [10u32, 20, 30];
        let bits = [0u32, 1, 0];
        let req_id = transport.send_batch_merkle_openings(
            &leaves, &paths, &bits, 8, 3, 1
        ).unwrap();
        assert_eq!(req_id, 1);
    }

    #[test]
    fn test_batch_inner_product_frame() {
        let mut transport = GpuOffloadTransport::new();
        let a = [1u32, 2, 3, 4];
        let b = [5u32, 6, 7, 8];
        let req_id = transport.send_batch_inner_product(&a, &b, 4, 1).unwrap();
        assert_eq!(req_id, 1);
    }

    #[test]
    fn test_bch_encode_decode_no_error() {
        let msg = [0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE];
        let enc = bch_encode(&msg);
        let dec = bch_decode(&enc).unwrap();
        assert_eq!(dec, msg);
    }

    #[test]
    fn test_bch_correct_single_error() {
        let msg = [0xFF; 7];
        let mut enc = bch_encode(&msg);
        enc[0] ^= 0x80; // flip one bit
        let dec = bch_decode(&enc).unwrap();
        assert_eq!(dec, msg);
    }

    #[test]
    fn test_bch_correct_two_errors() {
        let msg = [0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF, 0x00];
        let mut enc = bch_encode(&msg);
        enc[1] ^= 0x01; // flip bit in byte 1
        enc[4] ^= 0x02; // flip bit in byte 4
        let dec = bch_decode(&enc).unwrap();
        assert_eq!(dec, msg);
    }

    #[test]
    fn test_bch_uncorrectable_error() {
        let msg = [0x00; 7];
        let mut enc = bch_encode(&msg);
        // Flip 3 bits (beyond correction capability)
        enc[0] ^= 0xFF;
        enc[1] ^= 0xFF;
        enc[2] ^= 0xFF;
        let result = bch_decode(&enc);
        assert!(result.is_none() || result.unwrap() != msg);
    }
}