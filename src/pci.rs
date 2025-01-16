use x86_64::instructions::port::Port;

const CONFIG_ADDRESS: u16 = 0x0cf8;
const CONFIG_DATA: u16 = 0x0cfc;

fn write_config_address(address: u32) {
    let mut port = Port::new(CONFIG_ADDRESS);
    unsafe { port.write(address) }
}

fn read_config_data() -> u32 {
    let mut port = Port::new(CONFIG_DATA);
    unsafe { port.read() }
}

struct ClassCode {
    class_code: u8,
    subclass: u8,
    prog_if: u8,
}

impl ClassCode {
    fn device_type_str(&self) -> &'static str {
        match self.class_code {
            0x01 => match self.subclass {
                0x01 => "IDE Controller",
                _ => "Mass Storage Controller",
            },
            0x02 => match self.subclass {
                0x00 => "Ethernet Controller",
                _ => "Network Controller",
            },
            0x03 => match self.subclass {
                0x00 => "VGA Compatible Controller",
                _ => "Display Controller",
            },
            0x06 => match self.subclass {
                0x00 => "Host Bridge",
                0x01 => "ISA Bridge",
                _ => "Bridge",
            },
            0x0c => match self.subclass {
                0x03 => "USB Controller",
                _ => "Serial Bus Controller",
            },
            _ => "Unknown",
        }
    }
    pub fn is_pci_to_pci_bridge(&self) -> bool {
        self.class_code == 0x06 && self.subclass == 0x04
    }
}

impl core::fmt::Debug for ClassCode {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{} ({:02x}.{:02x}.{:02x})",
            self.device_type_str(),
            self.class_code,
            self.subclass,
            self.prog_if
        )
    }
}

struct VendorId {
    vendor: u16,
}

impl VendorId {
    const INVALID_ID: u16 = 0xffff;
    pub fn new(vendor: u16) -> Self {
        Self { vendor }
    }
    pub fn get(&self) -> u16 {
        self.vendor
    }
    pub fn is_invalid(&self) -> bool {
        self.vendor == Self::INVALID_ID
    }
}

struct PCIConfigurationReader {
    bus_num: u8,
    device_num: u8,
    function_num: u8,
}

impl PCIConfigurationReader {
    pub fn new(bus_num: u8, device_num: u8, function_num: u8) -> Self {
        PCIConfigurationReader {
            bus_num,
            device_num,
            function_num,
        }
    }
    fn make_address(&self, offset: u8) -> u32 {
        1 << 31
            | (self.bus_num as u32) << 16
            | (self.device_num as u32) << 11
            | (self.function_num as u32) << 8
            | offset as u32
    }
    fn read_data_at(&self, offset: u8) -> u32 {
        assert_eq!(offset % 4, 0); // must be 32bit aligned
        let address = self.make_address(offset);
        write_config_address(address);
        let data = read_config_data();
        data
    }
    pub fn read_class_code(&self) -> ClassCode {
        let data = self.read_data_at(0x8);
        let class_code = (data >> 24) & 0xff;
        let subclass = (data >> 16) & 0xff;
        let prog_if = (data >> 8) & 0xff;
        ClassCode {
            class_code: class_code as u8,
            subclass: subclass as u8,
            prog_if: prog_if as u8,
        }
    }
    pub fn read_header_type(&self) -> u8 {
        let data = self.read_data_at(0xc);
        (data >> 16) as u8
    }
    pub fn is_single_function(&self) -> bool {
        let header = self.read_header_type();
        (header & 0x80) == 0
    }
    pub fn read_vendor_id(&self) -> VendorId {
        let data = self.read_data_at(0x0);
        let vendor = (data & 0xffff) as u16;
        VendorId::new(vendor)
    }
    pub fn read_secondary_bus_num(&self) -> u8 {
        let class_code = self.read_class_code();
        assert!(class_code.is_pci_to_pci_bridge());
        let data = self.read_data_at(0x18);
        (data >> 0x08) as u8
    }
}

fn log_pci_function(bus_num: u8, device_num: u8, function_num: u8) {
    let config_reader = PCIConfigurationReader::new(bus_num, device_num, function_num);
    let vendor_id = config_reader.read_vendor_id();
    if vendor_id.is_invalid() {
        // crate::serial_println!("{}.{}.{}: Invalid", bus_num, device_num, function_num);
        return;
    }
    let class_code = config_reader.read_class_code();
    let header_type = config_reader.read_header_type();
    crate::serial_println!(
        "{}.{}.{}: vendor {:04x}, {:?}, header type {:02x}",
        bus_num,
        device_num,
        function_num,
        vendor_id.get(),
        class_code,
        header_type,
    );
}

// see: https://wiki.osdev.org/PCI#Recursive_Scan
pub fn scan_all() {
    let host_bridge_config = PCIConfigurationReader::new(0, 0, 0);
    if host_bridge_config.is_single_function() {
        scan_bus(0);
        return;
    }
    // multiple host controllers
    for function_num in 0..8 {
        // responsible for bus: bus_num = function_num
        let config = PCIConfigurationReader::new(0, 0, function_num);
        if config.read_vendor_id().is_invalid() {
            break;
        }
        let bus_num = function_num;
        scan_bus(bus_num);
    }
}

fn scan_bus(bus_num: u8) {
    for device_num in 0..32 {
        scan_device(bus_num, device_num);
    }
}

fn scan_device(bus_num: u8, device_num: u8) {
    // Every device must implement function 0
    let config = PCIConfigurationReader::new(bus_num, device_num, 0);
    if config.read_vendor_id().is_invalid() {
        // Skip non-existing device
        return;
    }
    scan_function(bus_num, device_num, 0);

    // Check multi function
    if !config.is_single_function() {
        for function_num in 1..8 {
            if config.read_vendor_id().is_invalid() {
                continue;
            }
            scan_function(bus_num, device_num, function_num);
        }
    }
}

fn scan_function(bus_num: u8, device_num: u8, function_num: u8) {
    log_pci_function(bus_num, device_num, function_num);

    let config = PCIConfigurationReader::new(bus_num, device_num, function_num);
    if config.read_class_code().is_pci_to_pci_bridge() {
        let secondary_bus = config.read_secondary_bus_num();
        scan_bus(secondary_bus);
    }
}
