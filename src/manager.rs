
use dashmap::{mapref::one::RefMut, DashMap};
use crate::CentralEvent;

use super::{hid_device::HidDevice};
use uuid::Uuid;
use anyhow::{Result};
use crossbeam_channel::{unbounded,Receiver,Sender};


#[derive(Debug, Clone)]
pub struct Manager{
    devices: DashMap<Uuid, HidDevice>,
    receiver: Receiver<CentralEvent>,
    sender: Sender<CentralEvent>,
}

impl Manager {
    pub fn new() -> Self {
        let (sender,receiver) = unbounded();
        Self{
            devices:DashMap::new(),
            receiver, 
            sender,
        }
    }

    pub fn emit(&self, event: CentralEvent) {
        if let Err(err) = self.sender.send(event) {
            println!("send event error: {}",err);
        }
    }

    pub fn event_channel(&self) -> Receiver<CentralEvent>{
        self.receiver.clone()
    }

    pub fn add_devices(&self,key:Uuid,device:HidDevice) ->Result<()>{
        // if self.devices.contains_key(&key) {
        //     bail!("Adding a device that's already in the map.");
        // }
        self.devices.insert(key,device);
        Ok(())
    }

    pub fn contains_device(&self,key:Uuid) -> bool {
        self.devices.contains_key(&key)
    }

    pub fn remove_device(&self,key:Uuid) -> Option<(Uuid, HidDevice)>{
        self.devices.remove(&key)
    }

    pub fn devices(&self) -> Vec<HidDevice> {
        self.devices
            .iter()
            .map(|val| val.value().clone())
            .collect()
    }

    pub fn device_keys(&self) -> Vec<Uuid> {
        self.devices
            .iter()
            .map(|val| val.key().clone())
            .collect()
    }

    pub fn _device_mut (
        &self,
        key:&Uuid,
    ) -> Option<RefMut<Uuid, HidDevice>>{
        self.devices.get_mut(key)
    }

    pub fn device(&self, key:&Uuid) -> Option<HidDevice>{
        self.devices.get(key).map(|val| val.value().clone())
    }
}
