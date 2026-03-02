use windows::Win32::Devices::Display::{
    DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME, DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_PATH_INFO,
    DISPLAYCONFIG_TARGET_DEVICE_NAME, DisplayConfigGetDeviceInfo, GetDisplayConfigBufferSizes,
    QDC_ALL_PATHS, QueryDisplayConfig,
};
use windows::Win32::Foundation::WIN32_ERROR;

fn main() {
    unsafe {
        let mut path_count = 0;
        let mut mode_count = 0;
        if GetDisplayConfigBufferSizes(QDC_ALL_PATHS, &mut path_count, &mut mode_count)
            != WIN32_ERROR(0)
        {
            println!("Failed to get buffer sizes");
            return;
        }

        let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); path_count as usize];
        let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); mode_count as usize];

        if QueryDisplayConfig(
            QDC_ALL_PATHS,
            &mut path_count,
            paths.as_mut_ptr(),
            &mut mode_count,
            modes.as_mut_ptr(),
            None,
        ) != WIN32_ERROR(0)
        {
            println!("Failed to query display config");
            return;
        }

        println!("Found {} paths", path_count);

        for (i, path) in paths.iter().take(path_count as usize).enumerate() {
            let active = (path.flags & 1) != 0;
            // The first bit of targetInfo.Anonymous.Anonymous._bitfield is targetAvailable
            let target_available = (path.targetInfo.Anonymous.Anonymous._bitfield & 1) != 0;

            let mut target_name = DISPLAYCONFIG_TARGET_DEVICE_NAME::default();
            target_name.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME;
            target_name.header.size =
                std::mem::size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32;
            target_name.header.adapterId = path.targetInfo.adapterId;
            target_name.header.id = path.targetInfo.id;

            let mut friendly_name = "Unknown".to_string();
            if DisplayConfigGetDeviceInfo(&mut target_name.header) == 0 {
                friendly_name = String::from_utf16_lossy(&target_name.monitorFriendlyDeviceName)
                    .trim_matches(char::from(0))
                    .to_string();
            }

            println!(
                "Path {}: Name: '{}', Active: {}, Available: {}, TargetID: {}, statusFlags: {:08x}",
                i,
                friendly_name,
                active,
                target_available,
                path.targetInfo.id,
                path.targetInfo.statusFlags
            );
        }
    }
}
