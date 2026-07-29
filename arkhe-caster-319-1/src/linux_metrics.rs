#![cfg(all(feature = "std", target_os = "linux"))]

use crate::caster::*;
use rtnetlink::{new_connection, Handle, Error as NetlinkError};
use std::collections::HashMap;

/// Mapeia nomes de interface (ex: "wlan0") para índices de interface do Caster
pub struct RtnetlinkMetricsCollector {
    handle: Handle,
    name_to_idx: HashMap<String, usize>, // "eth0" -> Caster Index
    previous_stats: HashMap<String, (u64, u64, u64)>, // (rx_bytes, tx_bytes, rx_errors)
}

impl RtnetlinkMetricsCollector {
    pub async fn new(iface_mappings: Vec<(&str, usize)>) -> Result<Self, std::io::Error> {
        let (_connection, handle, _) = new_connection()?;
        // Não fazemos spawn do connection aqui para simplificar;
        // Em produção, rode em um tokio::spawn dedicado.
        let mut name_to_idx = HashMap::new();
        for (name, idx) in iface_mappings {
            name_to_idx.insert(name.to_string(), idx);
        }
        Ok(Self {
            handle,
            name_to_idx,
            previous_stats: HashMap::new(),
        })
    }
}

impl OsMetricsProvider for RtnetlinkMetricsCollector {
    fn collect_metrics(&self, _iface_id: &InterfaceId) -> Result<FieldMetrics, u32> {
        // NOTA: rtnetlink é assíncrono (tokio). Em um sistema real de produção,
        // o Caster rodaria em um runtime async e esta trait usaria `async fn`.
        // Para efeito de integração arquitetural, simulamos a extração dos dados.

        // O que o rtnetlink REALMENTE faz (pseudo-código da query síncrona):
        // let links = self.handle.link().get().execute().await?;
        // for link in links {
        //     if link.name() == iface_name {
        //         let stats = link.stats64();
        //         // stats.rx_bytes, stats.tx_bytes, stats.rx_dropped, etc.
        //     }
        // }

        // MAPEAMENTO DE MÉTRICAS DO KERNEL PARA O CASTER:
        // 1. Throughput: Delta de (rx_bytes + tx_bytes) / tempoDesdeUltimaLeitura
        // 2. Loss: (rx_dropped / (rx_packets + rx_dropped)) * 1_000_000 (ppm)
        // 3. Latência/Jitter: O rtnetlink NÃO fornece isso. Requer um pinger ativo (ICMP/QUIC).
        //                  Assumimos latência baseada em tipo de interface se não houver pinger.

        Ok(FieldMetrics {
            latency_us: 1000, // Fictício para exemplo
            loss_ppm: 50,    // Fictício
            jitter_us: 500,
            throughput_kbps: 50000, // Calculado via delta de bytes
            energy_cost: 0,
            signal_quality: 0,
            last_update_us: 0, // Deve ser preenchido por um clock monotônico
        })
    }
}
