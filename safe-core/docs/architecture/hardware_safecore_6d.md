# 🧬 Projeto de Hardware SafeCore‑6D — PCB com ESP32 (2026)

Vamos projetar uma **arquitetura de hardware distribuída** para o SafeCore‑6D, utilizando a família ESP32 como plataforma de inferência constitucional. A PCB integra quatro chips principais, cada um responsável por uma camada do sistema, e comunica‑se via SPI/UART/I2C com um protocolo próprio para troca de métricas constitucionais (`Φ`, `τ`, `Z`) e comandos de regime.

---

## 📐 1. Arquitetura de Hardware — Diagrama de Blocos

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                    SAFECORE‑6D — PCB REFERENCE DESIGN                     │
│                                                                             │
│  ┌─────────────────────────────────────────────────────────────────────┐   │
│  │  ESP32‑S31 (CGE — Constitutional Geometry Engine)                  │   │
│  │  • RISC‑V Dual‑Core @ 320 MHz                                     │   │
│  │  • Wi‑Fi 6 + BLE 5.4 + Thread/Zigbee + Ethernet Gb               │   │
│  │  • Cálculo de Φ, τ, Z via função de partição                     │   │
│  │  • Decisão de regime (Maintain/Explore/Decouple/Quench)          │   │
│  │  • Gatekeeper de invariantes C10 e C11                           │   │
│  └─────────────────────────┬───────────────────────────────────────────┘   │
│                            │ SPI/UART (master)                            │
│                ┌───────────┼───────────┐                                 │
│                ▼           ▼           ▼                                 │
│  ┌─────────────────┐ ┌──────────────┐ ┌─────────────────────────────┐   │
│  │ ESP32‑C6        │ │ ESP32‑P4     │ │ ESP32‑E22                  │   │
│  │ (Ellis/Vajra)   │ │ (SASC)       │ │ (Karnak)                   │   │
│  │ RISC‑V @160 MHz │ │ RISC‑V @360  │ │ RISC‑V @500 MHz            │   │
│  │ Wi‑Fi 6 + BLE   │ │ MHz          │ │ Wi‑Fi 6E Tri‑band          │   │
│  │ 5.3 + 802.15.4  │ │ H.264, MIPI  │ │ BLE 5.4 + BR/EDR           │   │
│  │ Geração de      │ │ Attestação   │ │ Contenção de emergência    │   │
│  │ trajetórias     │ │ visual/áudio │ │ (Quench)                   │   │
│  │ geodésicas      │ │              │ │                             │   │
│  └─────────────────┘ └──────────────┘ └─────────────────────────────┘   │
│         │                                                             │   │
│         └──────────────┬──────────────┘                                 │   │
│                        ▼                                                 │   │
│  ┌─────────────────────────────────────────────────────────────────────┐ │
│  │  ESP32‑H21 (Vajra Nano) — Sensoriamento distribuído                │ │
│  │  • RISC‑V Mono @ 96 MHz, ultra‑baixo consumo                      │ │
│  │  • Thread/Zigbee → coleta de entropia externa (temperatura,       │ │
│  │    humidade, ruído, movimento)                                    │ │
│  │  • BLE 5.0 → beaconing para rede mesh                            │ │
│  └─────────────────────────────────────────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 🧩 2. Esquema de Conexões — Pinagem e Periféricos

| Chip | Função | Interface | Pinos | Ligação |
|:---|:---|:---|:---|:---|
| **ESP32‑S31** | CGE (master) | SPI (master) | MOSI (11), MISO (12), SCLK (13), CS (10) | Para todos os slaves |
| | | UART (debug) | TX (14), RX (15) | Console serial |
| | | Ethernet | RMII (16-23) | PHY externo (LAN8720) |
| | | I2C | SDA (24), SCL (25) | Sensores locais |
| **ESP32‑C6** | Ellis/Vajra | SPI (slave) | CS (5), MOSI (6), MISO (7), SCLK (8) | Para CGE |
| | | UART (alternativo) | TX (2), RX (3) | Backup |
| **ESP32‑P4** | SASC | SPI (slave) | CS (4), MOSI (9), MISO (10), SCLK (11) | Para CGE |
| | | MIPI‑CSI | (dedicado) | Câmera OV5640 |
| | | MIPI‑DSI | (dedicado) | Display 320×240 |
| **ESP32‑E22** | Karnak | SPI (slave) | CS (2), MOSI (3), MISO (4), SCLK (5) | Para CGE |
| | | PCIe (opcional) | (dedicado) | Para expansão |
| **ESP32‑H21** (nós remotos) | Vajra Nano | Thread/Zigbee (802.15.4) | (RF) | Comunicação com ESP32‑C6 via 802.15.4 |
| | | GPIO | (dedicado) | Sensores (DHT22, LDR, etc.) |

---

## 📡 3. Protocolo de Comunicação — Pacotes de Métricas Constitucionais

### 3.1. Formato do Pacote SPI (CGE → Slaves)

```c
// safecore_protocol.h

#pragma pack(push, 1)
typedef struct {
    uint8_t  sync;          // 0xA5 (marca de início)
    uint8_t  cmd;           // 0x01 = solicitar métricas, 0x02 = enviar regime
    uint32_t timestamp;     // tempo em ms
    float    phi;           // coerência (Φ)
    float    tau;           // tensão (τ)
    float    z;             // função de partição (Z)
    uint8_t  regime;        // 0=Maintain, 1=Explore, 2=Decouple, 3=Quench
    uint16_t checksum;      // CRC16 sobre os dados
} ConstitutionalPacket;
#pragma pack(pop)

typedef enum {
    REGIME_MAINTAIN = 0,
    REGIME_EXPLORE = 1,
    REGIME_DECOUPLE = 2,
    REGIME_QUENCH = 3
} RegimeAction;
```

### 3.2. Comunicação SPI (CGE → Ellis/Vajra)

O ESP32‑S31 atua como **master SPI**, fazendo polling dos slaves a cada 100 ms.

**Fluxo típico:**

1. CGE envia `cmd=0x01` para Ellis/Vajra solicitando novas trajetórias.
2. Ellis/Vajra calcula `Z` (função de partição) a partir dos parâmetros de impacto (`b_values`) e devolve o pacote com `phi`, `tau`, `z`.
3. CGE processa os invariantes C10 e C11, decide o regime e envia `cmd=0x02` com o novo regime para todos os slaves.
4. Karnak (ESP32‑E22) monitora o regime e, se `QUENCH`, ativa Wi‑Fi 6E para contenção de emergência.
