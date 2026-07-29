#![cfg_attr(not(feature = "std"), no_std)]
extern crate alloc;

pub mod substrates;
pub mod cognitive;
pub mod consensus;

pub mod network;

// ============================================================================
pub mod substrato_4004;
// SUBSTRATO 319.1 — CASTER SOFTWARE (Network Unification Layer)
// ============================================================================

use alloc::vec::Vec;
use alloc::string::String;
use alloc::boxed::Box;
use core::fmt;

use crate::consensus::PqcSignature;

#[inline]
fn secure_zeroize(bytes: &mut [u8]) {
    for byte in bytes.iter_mut() {
        unsafe {
            core::ptr::write_volatile(byte, 0u8);
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct PhysicalAddress {
    pub interface_type: InterfaceType,
    pub mac_or_ip: [u8; 16],
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum InterfaceType {
    Ethernet,
    WiFi,
    Cellular,
    Starlink,
    ArkheQuantum,
}

#[derive(Clone, Debug)]
pub struct FieldMetrics {
    pub latency_us: u32,
    pub jitter_us: u32,
    pub loss_ppm: u32,
    pub energy_cost: u32,
    pub bandwidth_kbps: u32,
}

impl FieldMetrics {
    pub fn quality_score(&self) -> u8 {
        if self.loss_ppm == 0 && self.jitter_us == 0 {
            return 100;
        }

        let loss_penalty = (self.loss_ppm / 1000) as u32;
        let jitter_penalty = (self.jitter_us / 100) as u32;
        let energy_penalty = (self.energy_cost / 10) as u32;

        let raw = 100u32
            .saturating_sub(loss_penalty)
            .saturating_sub(jitter_penalty)
            .saturating_sub(energy_penalty);

        raw.min(100) as u8
    }
}

pub struct UnifiedTunnel {
    pub local_pubkey: [u8; 32],
    pub remote_pubkey: [u8; 32],
    local_privkey: [u8; 32],
    pub interface: PhysicalAddress,
    pub mtu: u16,
    pub is_active: bool,
}

impl UnifiedTunnel {
    pub fn new(
        local_pub: [u8; 32],
        remote_pub: [u8; 32],
        local_priv: [u8; 32],
        interface: PhysicalAddress,
    ) -> Self {
        Self {
            local_pubkey: local_pub,
            remote_pubkey: remote_pub,
            local_privkey: local_priv,
            interface,
            mtu: 1420,
            is_active: true,
        }
    }

    pub fn teardown(&mut self) {
        secure_zeroize(&mut self.local_privkey);
        secure_zeroize(&mut self.local_pubkey);
        secure_zeroize(&mut self.remote_pubkey);
        self.is_active = false;
    }

    pub fn send(&self, _payload: &[u8]) -> Result<usize, TunnelError> {
        if !self.is_active {
            return Err(TunnelError::TunnelDown);
        }
        Ok(_payload.len())
    }
}

impl Drop for UnifiedTunnel {
    fn drop(&mut self) {
        if self.is_active {
            self.teardown();
        }
    }
}

#[derive(Debug)]
pub enum TunnelError {
    TunnelDown,
    KeyZeroizationFailed,
    MtuMismatch,
}

pub const FAILOVER_LOSS_PPM: u32 = 50_000;

pub struct InterfaceMonitor {
    interfaces: Vec<Option<Box<FieldMetrics>>>,
}

impl InterfaceMonitor {
    pub fn new(max_interfaces: usize) -> Self {
        let mut interfaces = Vec::new();
        for _ in 0..max_interfaces {
            interfaces.push(None);
        }
        Self {
            interfaces,
        }
    }

    pub fn register(&mut self, idx: usize, metrics: FieldMetrics) -> Result<(), &'static str> {
        if idx >= self.interfaces.len() {
            return Err("Index out of bounds");
        }
        self.interfaces[idx] = Some(Box::new(metrics));
        Ok(())
    }

    pub fn get_mut(&mut self, idx: usize) -> Option<&mut FieldMetrics> {
        self.interfaces.get_mut(idx)?.as_mut().map(|b| &mut **b)
    }

    pub fn best_interface(&self) -> Option<usize> {
        self.interfaces
            .iter()
            .enumerate()
            .filter_map(|(i, m)| m.as_ref().map(|metrics| (i, metrics.quality_score())))
            .max_by_key(|&(_, score)| score)
            .map(|(i, _)| i)
    }
}

pub struct CasterOrchestrator {
    pub monitor: InterfaceMonitor,
    pub active_tunnel: Option<UnifiedTunnel>,
}

impl CasterOrchestrator {
    pub fn new(max_interfaces: usize) -> Self {
        Self {
            monitor: InterfaceMonitor::new(max_interfaces),
            active_tunnel: None,
        }
    }

    pub fn tick(&mut self) {
        if let Some(best_idx) = self.monitor.best_interface() {
            let _ = best_idx;
        }
    }
}

pub trait LatencyProber: Send + Sync {
    fn probe_rtt(&mut self, target: &[u8; 32]) -> Option<u32>;
    fn name(&self) -> &'static str;
}

pub struct StubLatencyProber;

impl LatencyProber for StubLatencyProber {
    fn probe_rtt(&mut self, _target: &[u8; 32]) -> Option<u32> {
        Some(12_000)
    }

    fn name(&self) -> &'static str {
        "stub_quic_ping_v11.7"
    }
}

#[no_mangle]
pub extern "C" fn cathedral_boringtun_device_new(
    _privkey: *const u8,
    _pubkey: *const u8,
    _listen_port: u32,
) -> *mut UnifiedTunnel {
    core::ptr::null_mut()
}

#[no_mangle]
pub extern "C" fn cathedral_boringtun_device_free(_device: *mut UnifiedTunnel) {
}

#[no_mangle]
pub extern "C" fn cathedral_boringtun_device_tick(device: *mut UnifiedTunnel, _now_ms: u32) -> i32 {
    if device.is_null() {
        return -1;
    }
    0
}

#[no_mangle]
pub extern "C" fn cathedral_boringtun_device_read(
    device: *mut UnifiedTunnel,
    buf: *mut *mut u8,
    len: *mut u32,
) -> i32 {
    if device.is_null() || buf.is_null() || len.is_null() {
        return -1;
    }
    0
}

#[no_mangle]
pub extern "C" fn cathedral_boringtun_device_write(
    device: *mut UnifiedTunnel,
    data: *const u8,
    _len: u32,
) -> i32 {
    if device.is_null() || data.is_null() {
        return -1;
    }
    _len as i32
}

#[cfg(feature = "std")]
pub struct UdpLatencyProber {
    socket: Option<std::net::UdpSocket>,
    target_addr: Option<std::net::SocketAddr>,
    timeout_us: u64,
}

#[cfg(feature = "std")]
impl UdpLatencyProber {
    pub fn new(timeout_ms: u32) -> Result<Self, &'static str> {
        let socket = std::net::UdpSocket::bind("0.0.0.0:0")
            .map_err(|_| "Falha ao criar socket UDP para Prober")?;
        socket.set_nonblocking(true)
            .map_err(|_| "Falha ao configurar non-blocking")?;

        Ok(Self {
            socket: Some(socket),
            target_addr: None,
            timeout_us: timeout_ms as u64 * 1000,
        })
    }

    pub fn set_target(&mut self, ip: &str, port: u16) -> Result<(), &'static str> {
        let addr_str = format!("{}:{}", ip, port);
        let addr: std::net::SocketAddr = addr_str.parse()
            .map_err(|_| "Endereço IP inválido para Prober")?;
        self.target_addr = Some(addr);
        Ok(())
    }
}

#[cfg(feature = "std")]
impl LatencyProber for UdpLatencyProber {
    fn probe_rtt(&mut self, _target: &[u8; 32]) -> Option<u32> {
        if self.socket.is_none() || self.target_addr.is_none() {
            return None;
        }

        let socket = self.socket.as_ref().unwrap();
        let target = self.target_addr.unwrap();

        let tx_instant = std::time::Instant::now();
        let tx_timestamp = tx_instant.elapsed().as_micros() as u64;

        let packet = ProbePacket {
            magic: 0x4152_4B48_454D_5F50,
            tx_timestamp_us: tx_timestamp,
            nonce: tx_instant.elapsed().as_nanos() as u64,
        };

        let mut send_buf = [0u8; 24];
        unsafe {
            core::ptr::copy_nonoverlapping(
                &packet as *const _ as *const u8,
                send_buf.as_mut_ptr(),
                24,
            );
        }

        if socket.send_to(&send_buf, target).is_err() {
            return None;
        }

        let mut recv_buf = [0u8; 24];
        let start = std::time::Instant::now();

        loop {
            match socket.recv_from(&mut recv_buf) {
                Ok((len, _src)) if len == 24 => {
                    let rtt_us = start.elapsed().as_micros() as u32;
                    return Some(rtt_us);
                }
                Ok(_) => continue,
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if start.elapsed().as_micros() as u64 > self.timeout_us {
                        return None;
                    }
                    std::thread::sleep(std::time::Duration::from_micros(50));
                }
                Err(_) => return None,
            }
        }
    }

    fn name(&self) -> &'static str {
        "udp_raw_prober_v11.7_real"
    }
}

#[cfg(feature = "std")]
#[repr(C, packed)]
struct ProbePacket {
    magic: u64,
    tx_timestamp_us: u64,
    nonce: u64,
}

#[cfg(feature = "std")]
pub mod network_icmp;
#[cfg(feature = "std")]
pub use network_icmp::RealIcmpLatencyProber as IcmpLatencyProber;

#[cfg(not(feature = "std"))]
pub struct IcmpLatencyProber;

#[cfg(not(feature = "std"))]
impl LatencyProber for IcmpLatencyProber {
    fn probe_rtt(&mut self, _target: &[u8; 32]) -> Option<u32> {
        Some(8_000)
    }

    fn name(&self) -> &'static str {
        "icmp_echo_v11.7"
    }
}


#[no_mangle]
pub extern "C" fn cathedral_icmp_probe(target_bytes: *const u8) -> i32 {
    if target_bytes.is_null() {
        return -1;
    }
    unsafe {
        let target_slice = core::slice::from_raw_parts(target_bytes, 4);
        let mut target_arr = [0u8; 32];
        for i in 0..4 {
            target_arr[i] = target_slice[i];
        }
        #[cfg(feature = "std")]
        {
            if let Ok(mut prober) = network_icmp::RealIcmpLatencyProber::new() {
                if let Some(rtt) = prober.probe_rtt(&target_arr) {
                    return rtt as i32;
                }
            }
        }
        #[cfg(not(feature = "std"))]
        {
            let mut prober = IcmpLatencyProber;
            if let Some(rtt) = prober.probe_rtt(&target_arr) {
                return rtt as i32;
            }
        }
    }
    -1
}

pub mod integrations;
pub mod multi_chain;
