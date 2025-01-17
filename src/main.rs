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
    let mut pci_bus_scanner = pci::PCIBusScanner::new();
    pci_bus_scanner.scan_all();
    let xhci_controller = pci_bus_scanner.get_xhci_controller_address().unwrap();
    serial_println!("xHCI controller: {:?}", &xhci_controller);
    serial_println!("All done.");
    loop {}
}
