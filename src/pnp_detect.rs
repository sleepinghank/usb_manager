//
// Copyright © 2020 Haim Gelfenbeyn
// This code is licensed under MIT license (see LICENSE.txt for details)
//
use std::ffi::OsStr;
use std::iter::once;
use std::os::windows::ffi::OsStrExt;

use anyhow::{Result};
use winapi::shared::minwindef::{LPARAM, LRESULT, UINT, WPARAM};
use winapi::shared::ntdef::LPCWSTR;
use winapi::shared::windef::{HBRUSH, HCURSOR, HICON, HWND};
use winapi::um::libloaderapi::GetModuleHandleW;
use winapi::um::winuser::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, GetWindowLongPtrW, PostQuitMessage, RegisterClassW,
    SetWindowLongPtrW, TranslateMessage, GWLP_USERDATA, MSG, WM_CREATE, WM_DESTROY, WM_DEVICECHANGE, WNDCLASSW,
};

// use rusb::UsbContext;

// pub fn device2str<T: UsbContext>(device: rusb::Device<T>) -> Option<String> {
//     device
//         .device_descriptor()
//         .map(|device_desc| format!("{:04x}:{:04x}", device_desc.vendor_id(), device_desc.product_id()))
//         .ok()
// }


/// Detection of plugged in / removed USB devices on Windows: listens for WM_DEVICECHANGE messages.
/// This code should be removed once libusb supports hotplug notifications on Windows:
/// https://github.com/libusb/libusb/issues/86
pub struct PnPDetectWindows {
    hwnd: HWND,
    callback: Box<dyn Fn()>,
    // current_devices: HashSet<String>,
}

impl PnPDetectWindows {
    pub fn new(callback: Box<dyn Fn()>) -> Self {
        let mut pnp_detect = Self {
            callback,
            // current_devices: Self::read_device_list().unwrap_or_default(),
            hwnd: std::ptr::null_mut(),
        };
        pnp_detect.create_window();
        return pnp_detect;
    }

    // fn handle_hotplug_event(&mut self) {
    //     let new_devices = match Self::read_device_list() {
    //         Ok(devices) => devices,
    //         Err(err) => {
    //             println!("Cannot get a list of USB devices: {:?}", err);
    //             return;
    //         }
    //     };
    //     let added_devices = &new_devices - &self.current_devices;
    //     let removed_devices = &self.current_devices - &new_devices;
    //     for device in added_devices.iter() {
    //         self.callback.device_added(&device);
    //     }
    //     for device in removed_devices.iter() {
    //         self.callback.device_removed(&device);
    //     }
    //     self.current_devices = new_devices;
    // }

    /// Get a list of currently connected USB devices
    // fn read_device_list() -> Result<HashSet<String>> {
    //     Ok(rusb::devices()?
    //         .iter()
    //         .map(|device| device2str(device).ok_or(anyhow!("Cannot get device Ids")))
    //         .collect::<std::result::Result<_, _>>()?)
    // }

    /// Detect USB events: just run a Windows event loop
    pub fn detect(&self) -> Result<()> {
        unsafe {
            let mut msg: MSG = std::mem::MaybeUninit::zeroed().assume_init();
            loop {
                let val = GetMessageW(&mut msg, self.hwnd, 0, 0);
                if val == 0 {
                    break;
                } else {
                    TranslateMessage(&msg);
                    DispatchMessageW(&msg);
                }
            }
        }
        Ok(())
    }

    /// Window procedure function to handle events
    pub unsafe extern "system" fn window_proc(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        match msg {
            WM_CREATE => {
                let create_struct = lparam as *mut winapi::um::winuser::CREATESTRUCTW;
                let c_struct = match create_struct.as_ref() {
                    Some(v) => v,
                    None => return 0,
                };
                let window_state_ptr = c_struct.lpCreateParams;
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, window_state_ptr as isize);
            }
            WM_DESTROY => {
                PostQuitMessage(0);
            }
            WM_DEVICECHANGE => {
                let self_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut Self;
                let window_state: &mut Self = match self_ptr.as_mut() {
                    Some(v) => v,
                    None => return 0,
                };
                (window_state.callback)();
            }
            _ => return DefWindowProcW(hwnd, msg, wparam, lparam),
        }
        return 0;
    }

    /// Create an invisible window to handle WM_DEVICECHANGE message
    fn create_window(&mut self) {
        let winapi_class_name: Vec<u16> = OsStr::new("DisplaySwitchPnPDetectWindowClass")
            .encode_wide()
            .chain(once(0))
            .collect();
        let hinstance = unsafe { GetModuleHandleW(std::ptr::null()) };

        let wc = WNDCLASSW {
            style: 0,
            lpfnWndProc: Some(Self::window_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance,
            hIcon: 0 as HICON,
            hCursor: 0 as HCURSOR,
            hbrBackground: 0 as HBRUSH,
            lpszMenuName: 0 as LPCWSTR,
            lpszClassName: winapi_class_name.as_ptr(),
        };

        let error_code = unsafe { RegisterClassW(&wc) };
        assert_ne!(error_code, 0, "failed to register the window class");

        let window_name: Vec<u16> = OsStr::new("DisplaySwitchPnPDetectWindow")
            .encode_wide()
            .chain(once(0))
            .collect();

        let hwnd = unsafe {
            CreateWindowExW(
                0,
                winapi_class_name.as_ptr(),
                window_name.as_ptr(),
                0,
                0,
                0,
                0,
                0,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                hinstance,
                self as *mut Self as *mut winapi::ctypes::c_void,
                //std::ptr::null_mut(),
            )
        };

        if hwnd.is_null() {
            panic!("Something went wrong while creating a window");
        }
        self.hwnd = hwnd;
    }
}


#[cfg(test)]
mod tests {
    use std::time::Duration;
    use std::thread::sleep;
    use crate::*;
    #[test]
    fn windows_test() {
        // let usb_call = crate::usb::UsbCall::default();
        // let result = crate::usb::pnp_detect::PnPDetectWindows::new(Box::new(usb_call));
        // result.detect().unwrap();
        // loop {
        //     sleep(Duration::from_secs(10));
        // }
        assert_eq!(1, 1);
    }
}