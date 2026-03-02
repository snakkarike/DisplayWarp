use windows::Win32::Graphics::Gdi::{
    DISPLAY_DEVICE_STATE_FLAGS, DISPLAY_DEVICEW, EnumDisplayDevicesW,
};
use windows::core::BOOL;
use windows::core::PCWSTR;

#[test]
fn test_enum_devices() {
    unsafe {
        for i in 0..10 {
            let mut dd = DISPLAY_DEVICEW::default();
            dd.cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;
            let ok = EnumDisplayDevicesW(PCWSTR(std::ptr::null()), i, &mut dd, 0);
            if !ok.as_bool() {
                break;
            }
            let dev_name = String::from_utf16_lossy(&dd.DeviceName)
                .trim_matches(char::from(0))
                .to_string();
            let dev_string = String::from_utf16_lossy(&dd.DeviceString)
                .trim_matches(char::from(0))
                .to_string();
            println!("Adapter {}: {} - {}", i, dev_name, dev_string);

            // Now enumerate the monitors for this adapter
            for j in 0..10 {
                let mut mon = DISPLAY_DEVICEW::default();
                mon.cb = std::mem::size_of::<DISPLAY_DEVICEW>() as u32;
                let name_u16: Vec<u16> = dev_name.encode_utf16().chain(Some(0)).collect();

                let ok2 = EnumDisplayDevicesW(PCWSTR(name_u16.as_ptr()), j, &mut mon, 0);
                if !ok2.as_bool() {
                    break;
                }
                let mon_name = String::from_utf16_lossy(&mon.DeviceName)
                    .trim_matches(char::from(0))
                    .to_string();
                let mon_string = String::from_utf16_lossy(&mon.DeviceString)
                    .trim_matches(char::from(0))
                    .to_string();
                let is_active = (mon.StateFlags
                    & windows::Win32::Graphics::Gdi::DISPLAY_DEVICE_ACTIVE)
                    != DISPLAY_DEVICE_STATE_FLAGS(0);
                println!(
                    "  -> Monitor {}: {} - {} (Active: {})",
                    j, mon_name, mon_string, is_active
                );
            }
        }
    }
}
