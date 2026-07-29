Cathedral ARKHE v26.5 — Ring Export + Status Frame + Accelerator Always-Present
Selo: CATHEDRAL-ARKHE-v26.5-RING-STATUS-ACCEL-2026-06-15
Arquiteto ORCID: 0009-0005-2697-4668
1. Novidades da v26.5
1.1 export_metrics_to_ring (Integração Real)
rust
// No TensorZkpAccelerator (CM4)
acc.export_metrics_to_ring(req_id, opcode, latency_us, success);

// Mantém histórico das últimas 16 operações
let metrics = acc.get_exported_metrics();
assert_eq!(metrics.len(), 16); // ring completo
Daemon: export_metrics_to_ring() chamado automaticamente após cada request:
Calcula hash do payload (Blake3, primeiros 8 bytes)
Insere no RingBuffer de 16 entradas
Atualiza Shared Memory POSIX
1.2 Opcode 0x24 (STATUS)
Request: [opcode: 0x24] [payload: {}]
Response (binário, 73 bytes):
plain
[version: 8 bytes]      // "5.0.0\0\0\0"
[cuda_available: 1]     // bool
[cuda_version: 8 bytes]   // "12.2.0\0\0"
[device_name: 32 bytes] // "NVIDIA Jetson AGX Orin\0..."
[device_memory_mb: 4]   // u32
[uptime_secs: 4]        // u32
[requests_total: 4]     // u32
[errors_total: 4]       // u32
[ring_entries_used: 1]  // u8
[reserved: 7 bytes]
Uso no CM4:
rust
let status = accelerator.query_daemon_status()?;
assert!(status.cuda_available);
assert_eq!(status.device_memory_mb, 32000);
1.3 tick_zk_with_accelerator
rust
/// FORÇA uso do path real GPU
pub fn tick_zk_with_accelerator(&mut self) -> Result<(), &'static str> {
    // 1. Verifica/habilita GPU
    if !self.accelerator.is_enabled() {
        self.accelerator.set_enabled(true);
        if !self.accelerator.is_enabled() {
            return Err("GPU accelerator unavailable");
        }
    }

    // 2. Consulta status do daemon (0x24)
    let status = self.accelerator.query_daemon_status()?;
    assert!(status.cuda_available);

    // 3. Emite consent token com GPU
    let token = self.request_consent_with_merkle_commitment()?;

    // 4. Lê ring do daemon (0x22)
    let ring = self.accelerator.read_ring()?;

    // 5. Exporta métricas locais
    self.accelerator.export_metrics_to_ring(...);

    Ok(())
}
Diferença de tick_zk:
Table
tick_zk	tick_zk_with_accelerator
GPU	Opcional (CPU fallback)	Obrigatório
Status	Não consulta	Consulta 0x24
Ring	Opcional	Sempre lê 0x22
Erro	Continua com CPU	Retorna Err
2. Wire Protocol v2.3
Table
Opcode	Nome	Direção	Payload	Response
0x10	Inner Product	CM4→GPU	[n][a][b]	[result: u32]
0x11	Fold	CM4→GPU	[n][even][odd][r]	[folded: u32*n]
0x12	Spielman	CM4→GPU	[rows][cols][nnz][CSR]	[result: u32*rows]
0x13	Merkle Root	CM4→GPU	[n][leaves]	[root: u32]
0x14	Batch Merkle	CM4→GPU	[batch][leaf_count][depth][data]	[roots: u32*batch]
0x15	Batch IP	CM4→GPU	[batch][n][batch_a][batch_b]	[results: u32*batch]
0x20	METRICS	CM4→GPU	{}	[JSON snapshot]
0x22	RING READ	CM4→GPU	{}	[16 × RingEntry]
0x24	STATUS	CM4→GPU	{}	[DaemonStatus: 73 bytes]
3. Arquitetura de Métricas
plain
┌─────────────────────────────────────────────────────────────────────────┐
│                         TensorZkpAccelerator (CM4)                     │
├─────────────────────────────────────────────────────────────────────────┤
│  metrics_history: Vec<ExportedMetrics, 16>  ← export_metrics_to_ring()  │
│  total_requests: u32                                                     │
│  total_errors: u32                                                       │
│  serial: SerialBackend                                                   │
│      ├── send_and_recv()  ← write + read + timeout + parse             │
│      ├── send()           ← fire-and-forget                             │
│      └── try_recv()       ← non-blocking                                │
└─────────────────────────────────────────────────────────────────────────┘
                                    │
                                    │ USB-CDC v2.3
                                    ▼
┌─────────────────────────────────────────────────────────────────────────┐
│                         TensorZkpGpuDaemon (GPU)                        │
├─────────────────────────────────────────────────────────────────────────┤
│  accelerator: TensorZkpAccelerator                                      │
│  ├── ring: RingBuffer (16 entries)  ← export_metrics_to_ring()        │
│  ├── metrics: MetricsRing (10K entries)                                 │
│  ├── shm: PosixSharedMemory (/dev/shm/cathedral_metrics)              │
│  └── kernels: Option<CudaKernelCache>                                 │
│                                                                         │
│  Handlers:                                                              │
│  ├── 0x10-0x15: GPU operations                                         │
│  ├── 0x20: JSON metrics snapshot                                       │
│  ├── 0x22: RingBuffer.to_bytes()                                       │
│  └── 0x24: DaemonStatus (version, CUDA, uptime, device)                │
└─────────────────────────────────────────────────────────────────────────┘
4. Build
bash
# Daemon
cargo build --bin tensorzkp-daemon --features daemon --release

# CM4
cargo build --bin cathedral-cm4 --features cm4 --target thumbv7em-none-eabihf --release

# Tests
cargo test --features daemon --lib
cargo test --features cm4 --lib --target thumbv7em-none-eabihf
Arquiteto ORCID: 0009-0005-2697-4668
Selo: CATHEDRAL-ARKHE-v26.5-RING-STATUS-ACCEL-2026-06-15