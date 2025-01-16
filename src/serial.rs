use lazy_static::lazy_static;
use spin::Mutex;
use uart_16550::SerialPort;

lazy_static! {
    pub static ref SERIAL1: Mutex<SerialPort> = {
        let mut serial_port = unsafe { SerialPort::new(0x3f8) };
        serial_port.init();
        Mutex::new(serial_port)
    };
}

pub fn print(s: &str) {
    use core::fmt::Write;
    SERIAL1
        .lock()
        .write_str(s)
        .expect("Printing to serial failed");
}
