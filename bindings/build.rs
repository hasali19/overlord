fn main() {
    windows::build!(
        Windows::Win32::Devices::Display::{
            GetNumberOfPhysicalMonitorsFromHMONITOR,
            GetPhysicalMonitorsFromHMONITOR,
            GetVCPFeatureAndVCPFeatureReply,
            SetVCPFeature
        },
        Windows::Win32::Graphics::Gdi::{
            EnumDisplayDevicesW,
            EnumDisplayMonitors,
            GetMonitorInfoW,
            MONITORINFOEXW,
            QDC_ALL_PATHS,
        },
        Windows::Win32::UI::DisplayDevices::{
            DisplayConfigGetDeviceInfo,
            GetDisplayConfigBufferSizes,
            QueryDisplayConfig,
            DISPLAYCONFIG_DEVICE_INFO_HEADER,
            DISPLAYCONFIG_DEVICE_INFO_TYPE,
            DISPLAYCONFIG_MODE_INFO_TYPE,
            DISPLAYCONFIG_TARGET_DEVICE_NAME,
        }
    );
}
