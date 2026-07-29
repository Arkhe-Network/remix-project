#![cfg(all(feature = "std", target_os = "linux"))]

use crate::caster::*;

pub struct BoringtunNativeTunnel {
    // peer: boringtun::peer::Peer, // Stub for actual boringtun usage
}

impl BoringtunNativeTunnel {
    pub fn new() -> Self {
        Self {
            // peer: ...
        }
    }
}

impl OsTunnelProvider for BoringtunNativeTunnel {
    fn setup_tunnel(&mut self, _iface_idx: usize, _pubkey: &[u8], _privkey: &[u8]) -> Result<(), u32> {
        // Tudo roda no mesmo espaço de endereço, sem FFI, sem Go, sem syscalls de fork().
        Ok(())
    }

    fn migrate_tunnel(&mut self, _new_iface_idx: usize) -> Result<(), u32> {
        Ok(())
    }

    fn teardown_tunnel(&mut self) -> Result<(), u32> {
        Ok(())
    }
}
