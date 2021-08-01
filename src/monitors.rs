use std::error::Error;
use std::mem;
use std::ptr;

use bindings::Windows::Win32::Devices::Display::GetNumberOfPhysicalMonitorsFromHMONITOR;
use bindings::Windows::Win32::Devices::Display::GetPhysicalMonitorsFromHMONITOR;
use bindings::Windows::Win32::Devices::Display::GetVCPFeatureAndVCPFeatureReply;
use bindings::Windows::Win32::Devices::Display::SetVCPFeature;
use bindings::Windows::Win32::Devices::Display::PHYSICAL_MONITOR;
use bindings::Windows::Win32::Foundation::BOOL;
use bindings::Windows::Win32::Foundation::HANDLE;
use bindings::Windows::Win32::Foundation::LPARAM;
use bindings::Windows::Win32::Foundation::PWSTR;
use bindings::Windows::Win32::Foundation::RECT;
use bindings::Windows::Win32::Graphics::Gdi::EnumDisplayDevicesW;
use bindings::Windows::Win32::Graphics::Gdi::EnumDisplayMonitors;
use bindings::Windows::Win32::Graphics::Gdi::GetMonitorInfoW;
use bindings::Windows::Win32::Graphics::Gdi::DISPLAY_DEVICEW;
use bindings::Windows::Win32::Graphics::Gdi::HDC;
use bindings::Windows::Win32::Graphics::Gdi::HMONITOR;
use bindings::Windows::Win32::Graphics::Gdi::MONITORINFOEXW;
use bindings::Windows::Win32::Graphics::Gdi::QDC_ALL_PATHS;
use bindings::Windows::Win32::UI::DisplayDevices::DisplayConfigGetDeviceInfo;
use bindings::Windows::Win32::UI::DisplayDevices::GetDisplayConfigBufferSizes;
use bindings::Windows::Win32::UI::DisplayDevices::QueryDisplayConfig;
use bindings::Windows::Win32::UI::DisplayDevices::DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME;
use bindings::Windows::Win32::UI::DisplayDevices::DISPLAYCONFIG_DEVICE_INFO_HEADER;
use bindings::Windows::Win32::UI::DisplayDevices::DISPLAYCONFIG_MODE_INFO_TYPE_TARGET;
use bindings::Windows::Win32::UI::DisplayDevices::DISPLAYCONFIG_TARGET_DEVICE_NAME;

const VCP_POWER_MODE: u8 = 0xd6;
const VCP_POWER_MODE_NONE: u32 = 0x00;
const VCP_POWER_MODE_ON: u32 = 0x01;
const VCP_POWER_MODE_OFF: u32 = 0x05;

#[derive(Copy, Clone, Debug)]
pub enum PowerMode {
    On,
    Off,
}

impl PowerMode {
    fn from_vcp_code(value: u32) -> PowerMode {
        match value {
            VCP_POWER_MODE_ON => PowerMode::On,
            VCP_POWER_MODE_NONE | VCP_POWER_MODE_OFF => PowerMode::Off,
            _ => panic!("unsupported power mode"),
        }
    }

    fn vcp_code(&self) -> u32 {
        match self {
            PowerMode::On => VCP_POWER_MODE_ON,
            PowerMode::Off => VCP_POWER_MODE_OFF,
        }
    }
}

#[derive(Debug)]
pub struct Monitor {
    id: i32,
    name: String,
    handle: HANDLE,
}

impl Monitor {
    pub fn id(&self) -> i32 {
        self.id
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn power_mode(&self) -> PowerMode {
        let mut value = 0;
        unsafe {
            GetVCPFeatureAndVCPFeatureReply(
                self.handle,
                VCP_POWER_MODE,
                ptr::null_mut(),
                &mut value,
                ptr::null_mut(),
            );
        }
        PowerMode::from_vcp_code(value)
    }

    pub fn set_power_mode(&self, mode: PowerMode) -> Result<(), Box<dyn Error>> {
        let res = unsafe { SetVCPFeature(self.handle, VCP_POWER_MODE, mode.vcp_code()) };
        if res == 1 {
            Ok(())
        } else {
            Err("failed to set power mode".into())
        }
    }
}

pub fn get_monitors() -> Vec<Monitor> {
    let display_devices = get_display_devices();
    let display_monitors = get_display_monitors();

    let mut monitors = Vec::new();

    for (id, device) in (1..).zip(display_devices.into_iter()) {
        let monitor = display_monitors
            .iter()
            .find(|monitor| device.device_name.starts_with(&monitor.device_name))
            .unwrap();

        monitors.push(Monitor {
            id,
            name: device.friendly_name,
            handle: monitor.handle,
        });
    }

    monitors
}

#[derive(Debug)]
struct DisplayDevice {
    friendly_name: String,
    device_name: String,
}

fn get_display_devices() -> Vec<DisplayDevice> {
    let device_map = get_device_map();

    let mut num_paths = 0;
    let mut num_modes = 0;

    unsafe {
        GetDisplayConfigBufferSizes(QDC_ALL_PATHS, &mut num_paths, &mut num_modes);
    }

    let mut paths = Vec::with_capacity(num_paths as usize);
    let mut modes = Vec::with_capacity(num_modes as usize);

    unsafe {
        QueryDisplayConfig(
            QDC_ALL_PATHS,
            &mut num_paths,
            paths.as_mut_ptr(),
            &mut num_modes,
            modes.as_mut_ptr(),
            ptr::null_mut(),
        );

        paths.set_len(num_paths as usize);
        modes.set_len(num_modes as usize);
    }

    let mut devices = vec![];

    for mode in modes {
        if mode.infoType == DISPLAYCONFIG_MODE_INFO_TYPE_TARGET {
            let mut name: DISPLAYCONFIG_TARGET_DEVICE_NAME = unsafe { mem::zeroed() };

            name.header = DISPLAYCONFIG_DEVICE_INFO_HEADER {
                r#type: DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
                size: mem::size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32,
                adapterId: mode.adapterId,
                id: mode.id,
            };

            unsafe {
                DisplayConfigGetDeviceInfo(
                    (&mut name as *mut DISPLAYCONFIG_TARGET_DEVICE_NAME).cast(),
                );
            }

            let friendly_name = utf16_nt_to_string(&name.monitorFriendlyDeviceName);
            let device_path = name.monitorDevicePath;
            let (_, device_name) = device_map
                .iter()
                .find(|(id, _)| id[..] == device_path[..])
                .unwrap();

            devices.push(DisplayDevice {
                friendly_name,
                device_name: utf16_nt_to_string(device_name),
            });
        }
    }

    devices
}

type DeviceMap = Vec<([u16; 128], [u16; 32])>;

fn get_device_map() -> DeviceMap {
    let mut map = DeviceMap::new();

    let mut device = unsafe {
        let mut device: DISPLAY_DEVICEW = mem::zeroed();
        device.cb = mem::size_of::<DISPLAY_DEVICEW>() as u32;
        device
    };

    let mut i = 0;
    unsafe {
        while EnumDisplayDevicesW(PWSTR::NULL, i, &mut device, 0).as_bool() {
            let mut name = device.DeviceName.to_owned();

            if EnumDisplayDevicesW(PWSTR(name.as_mut_ptr()), 0, &mut device, 1).as_bool() {
                map.push((device.DeviceID.to_owned(), device.DeviceName.to_owned()));
            }

            i += 1
        }
    }

    map
}

#[derive(Debug)]
struct DisplayMonitor {
    device_name: String,
    handle: HANDLE,
}

fn get_display_monitors() -> Vec<DisplayMonitor> {
    let mut monitors = Vec::<DisplayMonitor>::new();

    unsafe {
        EnumDisplayMonitors(
            HDC::NULL,
            ptr::null_mut(),
            Some(enum_monitor_proc),
            LPARAM(&mut monitors as *mut _ as _),
        );
    }

    monitors
}

unsafe extern "system" fn enum_monitor_proc(
    hmonitor: HMONITOR,
    _hdc: HDC,
    _rect: *mut RECT,
    LPARAM(data): LPARAM,
) -> BOOL {
    let monitors = data as *mut Vec<DisplayMonitor>;

    let info = match get_monitor_info(hmonitor) {
        Some(info) => info,
        None => {
            eprint!("failed to get monitor info for hmonitor {:?}", hmonitor);
            return BOOL(1);
        }
    };

    let physical_monitors = match get_physical_monitors(hmonitor) {
        Some(v) => v,
        None => {
            eprintln!("no physical monitors found for hmonitor {:?}", hmonitor);
            return BOOL(1);
        }
    };

    for monitor in physical_monitors {
        (*monitors).push(DisplayMonitor {
            device_name: utf16_nt_to_string(&info.szDevice),
            handle: monitor.hPhysicalMonitor,
        })
    }

    BOOL(1)
}

fn get_monitor_info(hmonitor: HMONITOR) -> Option<MONITORINFOEXW> {
    let mut info = MONITORINFOEXW::default();
    info.__AnonymousBase_winuser_L13558_C43.cbSize = mem::size_of::<MONITORINFOEXW>() as u32;

    unsafe {
        if !GetMonitorInfoW(hmonitor, &mut info as *mut _ as _).as_bool() {
            return None;
        }
    }

    Some(info)
}

fn get_physical_monitors(hmonitor: HMONITOR) -> Option<Vec<PHYSICAL_MONITOR>> {
    let mut count = 0;
    unsafe {
        if GetNumberOfPhysicalMonitorsFromHMONITOR(hmonitor, &mut count) == 0 {
            return None;
        }
    }

    let mut monitors = Vec::with_capacity(count as usize);
    unsafe {
        if GetPhysicalMonitorsFromHMONITOR(hmonitor, count, monitors.as_mut_ptr()) == 0 {
            return None;
        }
        monitors.set_len(count as usize);
    }

    Some(monitors)
}

/// Converts a null terminated buffer of utf16 characters to a `String`.
fn utf16_nt_to_string(buf: &[u16]) -> String {
    let len = buf.iter().take_while(|&&i| i != 0).count();
    String::from_utf16_lossy(&buf[0..len])
}
