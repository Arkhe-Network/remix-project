#![cfg_attr(not(test), no_std)]
#![cfg_attr(not(test), no_main)]

mod memory;
mod scheduler;
mod syscalls;
mod ipc;
mod isolation;
mod temporal;
mod axiarchy;

#[cfg(not(test))]
use core::panic::PanicInfo;

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[cfg_attr(not(test), no_mangle)]
pub extern "C" fn kmain() -> ! {
    memory::init();
    scheduler::init();
    ipc::init();
    isolation::init();

    loop {
        scheduler::tick();
    }
}

#[cfg(not(target_os = "none"))]
fn main() {}
