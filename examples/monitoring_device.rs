use usb_manager::{adapter::Adapter, CentralEvent};

fn main() { 
    let adapter = Adapter::new();
    adapter.start().unwrap();
    
    let devices = adapter.peripherals().unwrap();
    println!("devices len: {:?}", devices.len());
    let read = adapter.events().unwrap();
    loop {
        match read.recv() {
            Ok(v) => {
                match v {
                    CentralEvent::DeviceAdd(id) => {
                        println!("Add:{:?}",id);
                    },
                    CentralEvent::DeviceRemove(device) => {
                        println!("Remove:{:?}",device.id);
                    },
                }
            },
            Err(err) => println!("Err:{:?}",err),
        }
    }
}