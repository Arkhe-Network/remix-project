#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

use alloc::vec::Vec;
use alloc::string::String;
use core::sync::atomic::{AtomicU32, Ordering};
use core::time::Duration;

// ─────────────────────────────────────────────────────────────────────────────
// CONSTANTES DO SUBSTRATO
// ─────────────────────────────────────────────────────────────────────────────

/// Máximo de interfaces monitoradas simultaneamente
pub const MAX_INTERFACES: usize = 8;

/// Máximo de rotas Arkhe canônicas
pub const MAX_ROUTES: usize = 256;

/// Janela de medição de métricas (ms)
pub const METRICS_WINDOW_MS: u64 = 1000;

/// Threshold de failover — latência acima disso triggera switch
pub const FAILOVER_LATENCY_MS: u32 = 100;

/// Threshold de perda de pacotes (% × 100, ou seja, 500 = 5%)
pub const FAILOVER_LOSS_PPM: u32 = 5000; // 5%

/// Threshold de jitter (ms × 1000, ou seja, 50000 = 50ms)
pub const FAILOVER_JITTER_US: u32 = 50000;

/// Tempo máximo de switch entre interfaces (μs)
pub const FAILOVER_DEADLINE_US: u32 = 50_000; // 50ms

/// Custo energético: Ethernet < Wi-Fi < Bluetooth (arbitrary units)
pub const COST_ETHERNET: u32 = 10;
pub const COST_WIFI: u32 = 50;
pub const COST_BLUETOOTH: u32 = 100;

/// Códigos de erro Caster
pub const CASTER_OK: u32 = 0x0000_0000;
pub const CASTER_NO_ROUTE: u32 = 0x3191_0001;
pub const CASTER_ALL_DOWN: u32 = 0x3191_0002;
pub const CASTER_FAILOVER_TIMEOUT: u32 = 0x3191_0003;
pub const CASTER_POLICY_VIOLATION: u32 = 0x3191_0004;

// ─────────────────────────────────────────────────────────────────────────────
// TIPOS FUNDAMENTAIS
// ─────────────────────────────────────────────────────────────────────────────

/// Identificador de interface de rede (heapless, no alloc)
pub type InterfaceId = [u8; 16];

/// Endereço Arkhe canônico — meio-agnóstico
/// Formato: "arkhe://node/<node_id>/service/<service_id>"
pub type ArkheAddress = [u8; 64];

/// Endereço físico de interface (IP, MAC, etc.)
pub type PhysicalAddress = [u8; 48];

/// Timestamp monotônico (μs desde boot)
pub type TimestampUs = u64;

/// Tipo de interface física
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum InterfaceType {
    Ethernet = 0x01,
    WiFi2_4GHz = 0x02,
    WiFi5GHz = 0x03,
    WiFi6GHz = 0x04,
    Bluetooth = 0x05,
    Cellular = 0x06,
    Loopback = 0xFF,
}

/// Estado de uma interface
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum InterfaceState {
    Down = 0x00,       // Interface inoperante
    Up = 0x01,         // Interface operante, não selecionada
    Primary = 0x02,    // Interface ativa principal
    Backup = 0x03,     // Interface de backup quente
    Quarantined = 0x04, // Interface isolada (anomalia detectada)
}

// ─────────────────────────────────────────────────────────────────────────────
// MÉTRICAS DE CAMPO (Field Metrics)
// ─────────────────────────────────────────────────────────────────────────────

/// Métricas coletadas em tempo real para cada interface
/// Todas as métricas são u32 para evitar float no hot path
#[derive(Clone, Copy, Debug, Default)]
pub struct FieldMetrics {
    /// Latência RTT (μs)
    pub latency_us: u32,
    /// Perda de pacotes (ppm — partes por milhão, 1000000 = 100%)
    pub loss_ppm: u32,
    /// Jitter (desvio padrão da latência, μs)
    pub jitter_us: u32,
    /// Throughput medido (kbps)
    pub throughput_kbps: u32,
    /// Custo energético (arbitrary units, menor = melhor)
    pub energy_cost: u32,
    /// Qualidade de sinal (dBm × -1, ou seja, 50 = -50dBm; 0 = não aplicável)
    pub signal_quality: u32,
    /// Timestamp da última atualização
    pub last_update_us: TimestampUs,
}

impl FieldMetrics {
    /// Calcula score composto de qualidade (maior = melhor)
    /// Fórmula: throughput / (latency × loss × jitter × energy)
    /// Normalizado para escala 0–1_000_000
    pub fn quality_score(&self) -> u32 {
        if self.latency_us == 0 || self.loss_ppm == 0 || self.jitter_us == 0 || self.energy_cost == 0 {
            return 0;
        }

        // Evita overflow: usa u64 intermediário
        let numerator = self.throughput_kbps as u64 * 1_000_000u64;
        let denominator = (self.latency_us as u64)
            .saturating_mul(self.loss_ppm.max(1) as u64)
            .saturating_mul(self.jitter_us.max(1) as u64)
            .saturating_mul(self.energy_cost as u64);

        if denominator == 0 {
            return 0;
        }

        (numerator / denominator).min(1_000_000) as u32
    }

    /// Verifica se interface está saudável (abaixo dos thresholds)
    pub fn is_healthy(&self) -> bool {
        self.latency_us <= FAILOVER_LATENCY_MS * 1000
            && self.loss_ppm <= FAILOVER_LOSS_PPM
            && self.jitter_us <= FAILOVER_JITTER_US
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// INTERFACE DE REDE
// ─────────────────────────────────────────────────────────────────────────────

/// Representação de uma interface de rede no Caster
pub struct NetworkInterface {
    pub id: InterfaceId,
    pub iface_type: InterfaceType,
    pub state: InterfaceState,
    pub physical_addr: PhysicalAddress,
    pub metrics: FieldMetrics,
    /// Contador de falhas consecutivas
    pub failure_count: u32,
    /// Timestamp da última falha
    pub last_failure_us: TimestampUs,
}

impl NetworkInterface {
    pub const fn new(id: InterfaceId, iface_type: InterfaceType) -> Self {
        Self {
            id,
            iface_type,
            state: InterfaceState::Down,
            physical_addr: [0u8; 48],
            metrics: FieldMetrics {
                latency_us: u32::MAX,
                loss_ppm: 1_000_000,
                jitter_us: u32::MAX,
                throughput_kbps: 0,
                energy_cost: match iface_type {
                    InterfaceType::Ethernet => COST_ETHERNET,
                    InterfaceType::WiFi2_4GHz | InterfaceType::WiFi5GHz | InterfaceType::WiFi6GHz => COST_WIFI,
                    InterfaceType::Bluetooth => COST_BLUETOOTH,
                    _ => 100,
                },
                signal_quality: 0,
                last_update_us: 0,
            },
            failure_count: 0,
            last_failure_us: 0,
        }
    }

    /// Atualiza métricas e estado
    pub fn update_metrics(&mut self, metrics: FieldMetrics, now_us: TimestampUs) {
        let was_healthy = self.metrics.is_healthy();
        self.metrics = metrics;
        self.metrics.last_update_us = now_us;

        let is_healthy = self.metrics.is_healthy();

        if !is_healthy {
            self.failure_count += 1;
            self.last_failure_us = now_us;
            if self.state == InterfaceState::Primary {
                self.state = InterfaceState::Up; // Perde status de primária
            }
        } else if was_healthy && self.failure_count > 0 {
            self.failure_count = self.failure_count.saturating_sub(1);
        }

        // Atualiza estado baseado em saúde
        match self.state {
            InterfaceState::Down if is_healthy => {
                self.state = InterfaceState::Up;
            }
            InterfaceState::Quarantined if self.failure_count == 0 => {
                self.state = InterfaceState::Up;
            }
            _ => {}
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// INTERFACE MONITOR — Coleta de Métricas
// ─────────────────────────────────────────────────────────────────────────────

/// Monitor de interfaces — coleta métricas de campo em tempo real
/// Sem alloc no hot path — usa array fixo
pub struct InterfaceMonitor {
    pub interfaces: [Option<NetworkInterface>; MAX_INTERFACES],
    pub count: usize,
}

impl InterfaceMonitor {
    pub const fn new() -> Self {
        const NONE: Option<NetworkInterface> = None;
        Self {
            interfaces: [NONE; MAX_INTERFACES],
            count: 0,
        }
    }

    /// Registra nova interface para monitoramento
    pub fn register(&mut self, iface: NetworkInterface) -> Result<usize, u32> {
        if self.count >= MAX_INTERFACES {
            return Err(CASTER_NO_ROUTE);
        }
        let idx = self.count;
        self.interfaces[idx] = Some(iface);
        self.count += 1;
        Ok(idx)
    }

    /// Atualiza métricas de interface por índice
    pub fn update_interface(&mut self, idx: usize, metrics: FieldMetrics, now_us: TimestampUs) -> Result<(), u32> {
        if idx >= self.count {
            return Err(CASTER_NO_ROUTE);
        }
        if let Some(ref mut iface) = self.interfaces[idx] {
            iface.update_metrics(metrics, now_us);
            Ok(())
        } else {
            Err(CASTER_NO_ROUTE)
        }
    }

    /// Retorna referência para interface por índice
    pub fn get(&self, idx: usize) -> Option<&NetworkInterface> {
        if idx >= self.count {
            return None;
        }
        self.interfaces[idx].as_ref()
    }

    /// Retorna referência mutável para interface por índice
    pub fn get_mut(&mut self, idx: usize) -> Option<&mut NetworkInterface> {
        if idx >= self.count {
            return None;
        }
        self.interfaces[idx].as_mut()
    }

    /// Itera sobre todas as interfaces saudáveis
    pub fn healthy_interfaces(&self) -> impl Iterator<Item = &NetworkInterface> {
        self.interfaces.iter()
            .filter_map(|opt| opt.as_ref())
            .filter(|iface| iface.metrics.is_healthy() && iface.state != InterfaceState::Quarantined)
    }

    /// Conta interfaces operantes
    pub fn active_count(&self) -> usize {
        self.interfaces.iter()
            .filter_map(|opt| opt.as_ref())
            .filter(|iface| iface.state != InterfaceState::Down)
            .count()
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// CASTER POLICY — Decisão de Roteamento
// ─────────────────────────────────────────────────────────────────────────────

/// Política de roteamento Arkhe — define como selecionar interface ótima
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoutingPolicy {
    /// Minimiza latência — para aplicações de tempo real
    MinLatency,
    /// Maximiza throughput — para transferência de dados
    MaxThroughput,
    /// Minimiza custo energético — para dispositivos móveis
    MinEnergy,
    /// Balanceamento de carga — distribui entre interfaces saudáveis
    Balanced,
    /// Prioridade fixa: Ethernet > Wi-Fi > Bluetooth
    PriorityFixed,
    /// Arkhe Adaptive — balanceia latência, throughput, energia, signal quality
    ArkheAdaptive,
}

/// Engine de decisão de roteamento
pub struct CasterPolicy {
    policy: RoutingPolicy,
    /// Peso para latência (ArkheAdaptive)
    weight_latency: u32,
    /// Peso para throughput
    weight_throughput: u32,
    /// Peso para energia
    weight_energy: u32,
    /// Peso para qualidade de sinal
    weight_signal: u32,
}

impl CasterPolicy {
    pub const fn new(policy: RoutingPolicy) -> Self {
        Self {
            policy,
            weight_latency: 30,
            weight_throughput: 25,
            weight_energy: 20,
            weight_signal: 25,
        }
    }

    /// Seleciona interface ótima baseada na política atual
    pub fn select_interface<'a>(
        &self,
        monitor: &'a InterfaceMonitor,
    ) -> Option<(usize, &'a NetworkInterface)> {
        let mut best_idx = None;
        let mut best_score = 0u32;

        for (idx, opt) in monitor.interfaces.iter().enumerate() {
            let iface = match opt {
                Some(i) if i.metrics.is_healthy() && i.state != InterfaceState::Quarantined => i,
                _ => continue,
            };

            let score = match self.policy {
                RoutingPolicy::MinLatency => {
                    // Inverte: menor latência = maior score
                    if iface.metrics.latency_us == 0 { 0 } else { 1_000_000_000 / iface.metrics.latency_us }
                }
                RoutingPolicy::MaxThroughput => {
                    iface.metrics.throughput_kbps
                }
                RoutingPolicy::MinEnergy => {
                    // Inverte: menor custo = maior score
                    if iface.metrics.energy_cost == 0 { 0 } else { 1_000_000 / iface.metrics.energy_cost }
                }
                RoutingPolicy::Balanced => {
                    iface.metrics.quality_score()
                }
                RoutingPolicy::PriorityFixed => {
                    // Ethernet=1000, Wi-Fi=500, Bluetooth=100
                    match iface.iface_type {
                        InterfaceType::Ethernet => 1000,
                        InterfaceType::WiFi2_4GHz | InterfaceType::WiFi5GHz | InterfaceType::WiFi6GHz => 500,
                        InterfaceType::Bluetooth => 100,
                        _ => 0,
                    }
                }
                RoutingPolicy::ArkheAdaptive => {
                    self.adaptive_score(iface)
                }
            };

            if score > best_score {
                best_score = score;
                best_idx = Some((idx, iface));
            }
        }

        best_idx
    }

    /// Score adaptativo Arkhe — pondera múltiplas métricas
    fn adaptive_score(&self, iface: &NetworkInterface) -> u32 {
        let m = &iface.metrics;

        // Normaliza cada métrica para escala 0–1000
        let latency_norm = if m.latency_us == 0 { 0 } else { (1_000_000 / m.latency_us).min(1000) };
        let throughput_norm = (m.throughput_kbps / 1000).min(1000); // Mbps → 0–1000
        let energy_norm = if m.energy_cost == 0 { 0 } else { (100_000 / m.energy_cost).min(1000) };
        let signal_norm = m.signal_quality.min(1000);

        // Ponderação
        let score = (latency_norm * self.weight_latency
            + throughput_norm * self.weight_throughput
            + energy_norm * self.weight_energy
            + signal_norm * self.weight_signal) / 100;

        score
    }

    /// Define política dinamicamente
    pub fn set_policy(&mut self, policy: RoutingPolicy) {
        self.policy = policy;
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// ARKHE RESOLVER — Endereços Canônicos
// ─────────────────────────────────────────────────────────────────────────────

/// Resolução de endereços Arkhe canônicos para interfaces físicas
/// `arkhe://node/<node_id>/service/<service_id>` → (InterfaceId, PhysicalAddress)
pub struct ArkheResolver {
    /// Tabela de rotas: ArkheAddress → (interface_idx, physical_addr)
    /// STUB: Em produção, usa DHT ou blockchain RBB para resolução distribuída
    routes: [(ArkheAddress, usize, PhysicalAddress); MAX_ROUTES],
    route_count: usize,
}

impl ArkheResolver {
    pub const fn new() -> Self {
        Self {
            routes: [([0u8; 64], 0, [0u8; 48]); MAX_ROUTES],
            route_count: 0,
        }
    }

    /// Registra rota canônica
    pub fn register_route(
        &mut self,
        arkhe_addr: ArkheAddress,
        interface_idx: usize,
        physical_addr: PhysicalAddress,
    ) -> Result<(), u32> {
        if self.route_count >= MAX_ROUTES {
            return Err(CASTER_NO_ROUTE);
        }
        self.routes[self.route_count] = (arkhe_addr, interface_idx, physical_addr);
        self.route_count += 1;
        Ok(())
    }

    /// Resolve endereço Arkhe para interface física
    pub fn resolve(&self, arkhe_addr: &ArkheAddress) -> Option<(usize, &PhysicalAddress)> {
        for i in 0..self.route_count {
            if self.routes[i].0 == *arkhe_addr {
                return Some((self.routes[i].1, &self.routes[i].2));
            }
        }
        None
    }

    /// Parse de string para ArkheAddress
    /// Formato: "arkhe://node/<node_id>/service/<service_id>"
    /// STUB: Em produção, parsing completo com validação
    pub fn parse_address(addr_str: &str) -> Option<ArkheAddress> {
        if !addr_str.starts_with("arkhe://") {
            return None;
        }

        let mut addr = [0u8; 64];
        let bytes = addr_str.as_bytes();
        let len = bytes.len().min(63);
        addr[..len].copy_from_slice(&bytes[..len]);
        Some(addr)
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// FAILOVER ENGINE — Switch Determinístico <50ms
// ─────────────────────────────────────────────────────────────────────────────

/// Engine de failover — garante switch de interface em <50ms
pub struct FailoverEngine {
    /// Interface primária atual
    pub primary_idx: Option<usize>,
    /// Interface de backup pré-selecionada
    pub backup_idx: Option<usize>,
    /// Contador de failovers executados
    pub failover_count: u64,
    /// Timestamp do último failover
    pub last_failover_us: TimestampUs,
    /// Flag: failover em andamento
    pub in_progress: bool,
}

impl FailoverEngine {
    pub const fn new() -> Self {
        Self {
            primary_idx: None,
            backup_idx: None,
            failover_count: 0,
            last_failover_us: 0,
            in_progress: false,
        }
    }

    /// Inicializa com interface primária e backup
    pub fn init(&mut self, primary: usize, backup: Option<usize>) {
        self.primary_idx = Some(primary);
        self.backup_idx = backup;
    }

    /// Verifica se failover é necessário e executa
    /// Retorna: (nova_interface_idx, foi_failover?)
    pub fn check_and_failover(
        &mut self,
        monitor: &InterfaceMonitor,
        policy: &CasterPolicy,
        now_us: TimestampUs,
    ) -> Result<(usize, bool), u32> {
        // Verifica se interface primária está saudável
        let primary_healthy = self.primary_idx
            .and_then(|idx| monitor.get(idx))
            .map(|iface| iface.metrics.is_healthy())
            .unwrap_or(false);

        if primary_healthy && !self.in_progress {
            // Tudo normal — retorna primária
            return Ok((self.primary_idx.unwrap(), false));
        }

        // Failover necessário
        if self.in_progress {
            // Já está em andamento — verifica se completou
            if now_us.saturating_sub(self.last_failover_us) > FAILOVER_DEADLINE_US as u64 {
                // Timeout — failover falhou
                return Err(CASTER_FAILOVER_TIMEOUT);
            }
            // Ainda em andamento — retorna backup se disponível
            if let Some(backup) = self.backup_idx {
                return Ok((backup, true));
            }
        }

        // Inicia novo failover
        self.in_progress = true;
        self.last_failover_us = now_us;

        // Seleciona nova interface via política
        match policy.select_interface(monitor) {
            Some((idx, _)) => {
                self.primary_idx = Some(idx);
                self.failover_count += 1;
                self.in_progress = false;
                Ok((idx, true))
            }
            None => {
                self.in_progress = false;
                Err(CASTER_ALL_DOWN)
            }
        }
    }

    /// Força failover manual (para manutenção ou teste)
    pub fn force_failover(&mut self, new_primary: usize) {
        self.primary_idx = Some(new_primary);
        self.failover_count += 1;
        self.in_progress = false;
    }

    pub fn failover_count(&self) -> u64 {
        self.failover_count
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// UNIFIED TUNNEL — WireGuard como Cripto-Trivium Software
// ─────────────────────────────────────────────────────────────────────────────

/// Túnel unificado — uma única chave SPHINCS+ para todas as interfaces
/// STUB: Em produção, integra com wireguard-go ou kernel module
pub struct UnifiedTunnel {
    /// Chave pública SPHINCS+ do nó local (3952 bytes)
    pub local_pubkey: [u8; 3952],  // SPHINCS+-128s public key
    /// Chave privada SPHINCS+ — em RAM segura, zeroizada em teardown
    pub local_privkey: [u8; 128],   // SPHINCS+-128s secret key (simplificado)
    /// Interface ativa do túnel
    pub active_interface: Option<usize>,
    /// Estado do túnel
    pub state: TunnelState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TunnelState {
    Down,
    Handshaking,
    Established,
    Rekeying,
    Error,
}

impl UnifiedTunnel {
    pub const fn new() -> Self {
        Self {
            local_pubkey: [0u8; 3952],
            local_privkey: [0u8; 128],
            active_interface: None,
            state: TunnelState::Down,
        }
    }

    /// Inicializa túnel com chaves SPHINCS+
    /// STUB: Em produção, gera ou carrega chaves do TEE
    pub fn init(&mut self, pubkey: [u8; 3952], privkey: [u8; 128]) {
        self.local_pubkey = pubkey;
        self.local_privkey = privkey;
        self.state = TunnelState::Down;
    }

    /// Estabelece túnel sobre interface selecionada
    /// STUB: Em produção, handshake WireGuard + SPHINCS+ signature
    pub fn establish(&mut self, interface_idx: usize) -> Result<(), u32> {
        self.active_interface = Some(interface_idx);
        self.state = TunnelState::Handshaking;

        // STUB: Handshake WireGuard
        // HONESTY.md: "Stub de handshake. Em produção: wireguard-go + SPHINCS+"
        self.state = TunnelState::Established;
        Ok(())
    }

    /// Migra túnel para nova interface (failover)
    /// Mantém sessão ativa — zero handshake
    pub fn migrate(&mut self, new_interface_idx: usize) -> Result<(), u32> {
        let old_idx = self.active_interface;
        self.active_interface = Some(new_interface_idx);

        // STUB: Em produção, notifica peer da migração via control channel
        // HONESTY.md: "Stub de migração. Em produção: QUIC connection migration"

        if old_idx.is_some() {
            // Sessão preservada — interface mudou
            Ok(())
        } else {
            // Sem sessão anterior — estabelece nova
            self.establish(new_interface_idx)
        }
    }

    /// Encerra túnel e zeroiza chaves
    pub fn teardown(&mut self) {
        // Zeroiza chave privada
        for byte in self.local_privkey.iter_mut() {
            *byte = 0;
        }
        self.state = TunnelState::Down;
        self.active_interface = None;
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// ABSTRAÇÕES OS/HARDWARE (HAL) - Contratos agnósticos
// ─────────────────────────────────────────────────────────────────────────────

/// Trait para coletar métricas reais do SO ou hardware.
pub trait OsMetricsProvider: Send + Sync {
    /// Coleta as métricas brutas de uma interface física (ex: eth0).
    fn collect_metrics(&self, iface_id: &InterfaceId) -> Result<FieldMetrics, u32>;
}

/// Trait para gerenciar o túnel unificado.
pub trait OsTunnelProvider: Send + Sync {
    /// Inicializa o túnel na interface especificada.
    fn setup_tunnel(&mut self, iface_idx: usize, pubkey: &[u8], privkey: &[u8]) -> Result<(), u32>;
    /// Migra o túnel (ex: troca o fd do TUN de eth0 para wlan0).
    fn migrate_tunnel(&mut self, new_iface_idx: usize) -> Result<(), u32>;
    /// Destrói o túnel e zeroiza chaves.
    fn teardown_tunnel(&mut self) -> Result<(), u32>;
}

// ─────────────────────────────────────────────────────────────────────────────
// CASTER ORQUESTRADOR — Integração completa
// ─────────────────────────────────────────────────────────────────────────────

/// Orquestrador Caster — integra todos os módulos
pub struct CasterOrchestrator<M: OsMetricsProvider, T: OsTunnelProvider> {
    pub monitor: InterfaceMonitor,
    pub policy: CasterPolicy,
    pub resolver: ArkheResolver,
    pub failover: FailoverEngine,
    pub tunnel: UnifiedTunnel,

    // NOVAS DEPENDÊNCIAS INJETADAS
    pub metrics_provider: M,
    pub tunnel_provider: T,

    /// Métricas globais
    pub messages_routed: u64,
    pub bytes_transferred: u64,
    pub last_tick_us: TimestampUs,
}

impl<M: OsMetricsProvider, T: OsTunnelProvider> CasterOrchestrator<M, T> {
    pub fn new(policy: RoutingPolicy, metrics_provider: M, tunnel_provider: T) -> Self {
        Self {
            monitor: InterfaceMonitor::new(),
            policy: CasterPolicy::new(policy),
            resolver: ArkheResolver::new(),
            failover: FailoverEngine::new(),
            tunnel: UnifiedTunnel::new(),

            metrics_provider,
            tunnel_provider,

            messages_routed: 0,
            bytes_transferred: 0,
            last_tick_us: 0,
        }
    }

    /// Tick principal — executado a cada intervalo (ex: 10ms)
    pub fn tick(&mut self, now_us: TimestampUs) -> Result<(), u32> {
        self.last_tick_us = now_us;

        // 1. Verifica saúde das interfaces via métricas reais
        for idx in 0..self.monitor.count {
            if let Some(iface) = self.monitor.interfaces[idx].as_ref() {
                // Tenta ler métricas reais do provider
                match self.metrics_provider.collect_metrics(&iface.id) {
                    Ok(mut m) => {
                        m.last_update_us = now_us;
                        if let Some(iface_mut) = self.monitor.interfaces[idx].as_mut() {
                            iface_mut.update_metrics(m, now_us);
                        }
                    }
                    Err(_) => {
                        // Não foi possível coletar métricas, considera ruim
                        let mut bad_metrics = iface.metrics;
                        bad_metrics.latency_us = u32::MAX;
                        bad_metrics.loss_ppm = 1_000_000; // 100% loss
                        if let Some(iface_mut) = self.monitor.interfaces[idx].as_mut() {
                            iface_mut.update_metrics(bad_metrics, now_us);
                        }
                    }
                }
            }
        }

        // 2. Verifica se failover é necessário
        let (active_idx, _did_failover) = self.failover.check_and_failover(
            &self.monitor,
            &self.policy,
            now_us,
        )?;

        // 3. Verifica se túnel precisa migrar
        if self.tunnel.active_interface != Some(active_idx) {
            self.tunnel.migrate(active_idx)?;
            // Chama a abstração do OS para fazer a migração no nível kernel/Wireguard
            self.tunnel_provider.migrate_tunnel(active_idx)?;
        }

        // 4. Atualiza contadores
        self.messages_routed += 1;

        Ok(())
    }

    /// Roteia mensagem para endereço Arkhe
    pub fn route_message(
        &mut self,
        dest: &ArkheAddress,
        payload: &[u8],
    ) -> Result<usize, u32> {
        // 1. Resolve endereço
        let (iface_idx, _phys_addr) = self.resolver.resolve(dest)
            .ok_or(CASTER_NO_ROUTE)?;

        // 2. Verifica se interface está ativa
        if self.failover.primary_idx != Some(iface_idx) {
            // Interface não é primária — verifica se é backup válido
            let iface = self.monitor.get(iface_idx)
                .ok_or(CASTER_NO_ROUTE)?;
            if !iface.metrics.is_healthy() {
                return Err(CASTER_ALL_DOWN);
            }
        }

        // 3. Encapsula no túnel unificado
        // STUB: Em produção, criptografa payload com chave de sessão WireGuard

        self.bytes_transferred += payload.len() as u64;
        self.messages_routed += 1;

        Ok(iface_idx)
    }

    /// Adiciona interface ao monitor
    pub fn add_interface(&mut self, iface: NetworkInterface) -> Result<usize, u32> {
        self.monitor.register(iface)
    }

    /// Define interface primária e backup
    pub fn set_failover(&mut self, primary: usize, backup: Option<usize>) {
        self.failover.init(primary, backup);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// TESTES
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    struct MockMetricsProvider;
    impl OsMetricsProvider for MockMetricsProvider {
        fn collect_metrics(&self, _iface_id: &InterfaceId) -> Result<FieldMetrics, u32> {
            Ok(FieldMetrics::default())
        }
    }

    struct MockTunnelProvider;
    impl OsTunnelProvider for MockTunnelProvider {
        fn setup_tunnel(&mut self, _iface_idx: usize, _pubkey: &[u8], _privkey: &[u8]) -> Result<(), u32> { Ok(()) }
        fn migrate_tunnel(&mut self, _new_iface_idx: usize) -> Result<(), u32> { Ok(()) }
        fn teardown_tunnel(&mut self) -> Result<(), u32> { Ok(()) }
    }


    fn make_iface(id_byte: u8, iface_type: InterfaceType) -> NetworkInterface {
        let mut id = [0u8; 16];
        id[0] = id_byte;
        NetworkInterface::new(id, iface_type)
    }

    fn healthy_metrics(latency_us: u32, throughput_kbps: u32) -> FieldMetrics {
        FieldMetrics {
            latency_us,
            loss_ppm: 100, // 0.01%
            jitter_us: 1000, // 1ms
            throughput_kbps,
            energy_cost: COST_ETHERNET,
            signal_quality: 60, // -60dBm
            last_update_us: 1_000_000,
        }
    }

    #[test]
    fn test_field_metrics_quality_score() {
        let m = healthy_metrics(1000, 100_000); // 1ms lat, 100Mbps
        let score = m.quality_score();
        assert!(score > 0);
        assert!(score <= 1_000_000);
    }

    #[test]
    fn test_interface_health() {
        let healthy = healthy_metrics(10_000, 1000); // 10ms
        assert!(healthy.is_healthy());

        let unhealthy = FieldMetrics {
            latency_us: 200_000, // 200ms > 100ms threshold
            ..healthy_metrics(200_000, 1000)
        };
        assert!(!unhealthy.is_healthy());
    }

    #[test]
    fn test_policy_min_latency() {
        let mut monitor = InterfaceMonitor::new();

        let mut eth = make_iface(0x01, InterfaceType::Ethernet);
        eth.update_metrics(healthy_metrics(1000, 100_000), 1_000_000); // 1ms
        monitor.register(eth).unwrap();

        let mut wifi = make_iface(0x02, InterfaceType::WiFi5GHz);
        wifi.update_metrics(healthy_metrics(5000, 50_000), 1_000_000); // 5ms
        monitor.register(wifi).unwrap();

        let policy = CasterPolicy::new(RoutingPolicy::MinLatency);
        let (idx, iface) = policy.select_interface(&monitor).unwrap();

        assert_eq!(iface.iface_type, InterfaceType::Ethernet); // Menor latência
    }

    #[test]
    fn test_policy_priority_fixed() {
        let mut monitor = InterfaceMonitor::new();

        let mut bt = make_iface(0x01, InterfaceType::Bluetooth);
        bt.update_metrics(healthy_metrics(1000, 1000), 1_000_000);
        monitor.register(bt).unwrap();

        let mut eth = make_iface(0x02, InterfaceType::Ethernet);
        eth.update_metrics(healthy_metrics(5000, 100_000), 1_000_000);
        monitor.register(eth).unwrap();

        let policy = CasterPolicy::new(RoutingPolicy::PriorityFixed);
        let (idx, iface) = policy.select_interface(&monitor).unwrap();

        assert_eq!(iface.iface_type, InterfaceType::Ethernet); // Prioridade fixa
    }

    #[test]
    fn test_failover_engine() {
        let mut monitor = InterfaceMonitor::new();

        let mut eth = make_iface(0x01, InterfaceType::Ethernet);
        eth.update_metrics(healthy_metrics(1000, 100_000), 1_000_000);
        let eth_idx = monitor.register(eth).unwrap();

        let mut wifi = make_iface(0x02, InterfaceType::WiFi5GHz);
        wifi.update_metrics(healthy_metrics(2000, 50_000), 1_000_000);
        let wifi_idx = monitor.register(wifi).unwrap();

        let mut failover = FailoverEngine::new();
        failover.init(eth_idx, Some(wifi_idx));

        let policy = CasterPolicy::new(RoutingPolicy::MinLatency);

        // Primária saudável — sem failover
        let (idx, did_failover) = failover.check_and_failover(&monitor, &policy, 2_000_000).unwrap();
        assert_eq!(idx, eth_idx);
        assert!(!did_failover);

        // Simula falha da primária
        monitor.interfaces[eth_idx].as_mut().unwrap().update_metrics(
            FieldMetrics {
                latency_us: 500_000, // 500ms — acima do threshold
                loss_ppm: 100_000,   // 10%
                jitter_us: 100_000,
                throughput_kbps: 0,
                energy_cost: COST_ETHERNET,
                signal_quality: 0,
                last_update_us: 3_000_000,
            },
            3_000_000,
        );

        // Failover para Wi-Fi
        let (idx2, did_failover2) = failover.check_and_failover(&monitor, &policy, 4_000_000).unwrap();
        assert_eq!(idx2, wifi_idx);
        assert!(did_failover2);
        assert_eq!(failover.failover_count(), 1);
    }

    #[test]
    fn test_arkhe_resolver() {
        let mut resolver = ArkheResolver::new();

        let addr = ArkheResolver::parse_address("arkhe://node/alpha/service/cast").unwrap();
        let phys = [0x0Au8; 48];
        resolver.register_route(addr, 0, phys).unwrap();

        let resolved = resolver.resolve(&addr);
        assert!(resolved.is_some());
        assert_eq!(resolved.unwrap().0, 0);
    }

    #[test]
    fn test_caster_orchestrator() {
        let mut caster = CasterOrchestrator::new(RoutingPolicy::ArkheAdaptive, MockMetricsProvider, MockTunnelProvider);

        let mut eth = make_iface(0x01, InterfaceType::Ethernet);
        eth.update_metrics(healthy_metrics(1000, 100_000), 1_000_000);
        let eth_idx = caster.add_interface(eth).unwrap();

        caster.set_failover(eth_idx, None);

        let result = caster.tick(2_000_000);
        assert!(result.is_ok());
    }

    #[test]
    fn test_unified_tunnel_lifecycle() {
        let mut tunnel = UnifiedTunnel::new();
        assert_eq!(tunnel.state, TunnelState::Down);

        tunnel.init([0xABu8; 3952], [0xCDu8; 128]);
        tunnel.establish(0).unwrap();
        assert_eq!(tunnel.state, TunnelState::Established);

        tunnel.migrate(1).unwrap();
        assert_eq!(tunnel.active_interface, Some(1));

        tunnel.teardown();
        assert_eq!(tunnel.state, TunnelState::Down);
        assert!(tunnel.local_privkey.iter().all(|&b| b == 0));
    }

    #[test]
    fn test_caster_error_codes() {
        assert_eq!(CASTER_OK, 0x0000_0000);
        assert_eq!(CASTER_NO_ROUTE, 0x3191_0001);
        assert_eq!(CASTER_ALL_DOWN, 0x3191_0002);
        assert_eq!(CASTER_FAILOVER_TIMEOUT, 0x3191_0003);
    }
}
