#![no_std]
#![no_main]

mod pci;
mod serial;
mod vga_buffer;

use core::panic::PanicInfo;

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    vga_buffer::print_something();
    serial_println!("Hello, serial port!");
    pci::scan_all();
    serial_println!("All done.");
    loop {}
}
