#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod caster;

#[cfg(all(feature = "std", target_os = "linux"))]
pub mod linux_metrics;

#[cfg(all(feature = "std", target_os = "linux"))]
pub mod linux_wg_tunnel;
