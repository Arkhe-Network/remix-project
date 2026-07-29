use socket2::{Socket, Domain, Type, Protocol};
use std::net::{SocketAddr, IpAddr, Ipv4Addr};
use std::time::{Instant, Duration};

pub struct RealIcmpLatencyProber {
    socket: Option<Socket>,
    seq: u16,
}

impl RealIcmpLatencyProber {
    pub fn new() -> Result<Self, &'static str> {
        let socket = Socket::new(Domain::IPV4, Type::RAW, Some(Protocol::ICMPV4))
            .map_err(|_| "Falha ao criar raw socket ICMP. Requer privilégios root/CAP_NET_RAW.")?;

        socket.set_nonblocking(true)
            .map_err(|_| "Falha ao configurar socket non-blocking")?;

        Ok(Self {
            socket: Some(socket),
            seq: 0,
        })
    }
}

// Checksum for ICMP
fn checksum(data: &[u8]) -> u16 {
    let mut sum = 0u32;
    let mut i = 0;
    while i < data.len() {
        let word = if i + 1 < data.len() {
            (data[i] as u32) << 8 | (data[i + 1] as u32)
        } else {
            (data[i] as u32) << 8
        };
        sum = sum.wrapping_add(word);
        i += 2;
    }
    while (sum >> 16) > 0 {
        sum = (sum & 0xffff) + (sum >> 16);
    }
    !(sum as u16)
}

impl crate::LatencyProber for RealIcmpLatencyProber {
    fn probe_rtt(&mut self, _target: &[u8; 32]) -> Option<u32> {
        if self.socket.is_none() {
            return None;
        }

        let socket = self.socket.as_ref().unwrap();
        // just using a dummy ip here, since the target is a byte array and we're missing translation logic
        // For testing we will ping 8.8.8.8
        let target_ip = Ipv4Addr::new(_target[0], _target[1], _target[2], _target[3]);
        let dest = SocketAddr::new(IpAddr::V4(target_ip), 0);

        self.seq = self.seq.wrapping_add(1);

        let mut packet = [0u8; 64];
        packet[0] = 8; // ICMP Echo Request type
        packet[1] = 0; // Code
        packet[4] = 0x12; // ID
        packet[5] = 0x34;
        packet[6] = (self.seq >> 8) as u8;
        packet[7] = (self.seq & 0xff) as u8;

        let cksum = checksum(&packet);
        packet[2] = (cksum >> 8) as u8;
        packet[3] = (cksum & 0xff) as u8;

        let start = Instant::now();
        if socket.send_to(&packet, &dest.into()).is_err() {
            return None;
        }

        let mut buf = [std::mem::MaybeUninit::new(0); 128];
        loop {
            match socket.recv_from(&mut buf) {
                Ok(_) => {
                    let rtt_us = start.elapsed().as_micros() as u32;
                    return Some(rtt_us);
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if start.elapsed() > Duration::from_millis(50) {
                        return None;
                    }
                    std::thread::sleep(Duration::from_micros(100));
                }
                Err(_) => return None,
            }
        }
    }

    fn name(&self) -> &'static str {
        "icmp_echo_v11.7"
    }
}
