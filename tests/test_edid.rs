use windows::Win32::Devices::Display::{
    DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME, DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_PATH_INFO,
    DISPLAYCONFIG_TARGET_DEVICE_NAME, DISPLAYCONFIG_TARGET_DEVICE_NAME_FLAGS,
    DisplayConfigGetDeviceInfo, GetDisplayConfigBufferSizes, QDC_ONLY_ACTIVE_PATHS,
    QueryDisplayConfig,
};

#[test]
fn test_query_display_config() {
    unsafe {
        let mut path_count = 0;
        let mut mode_count = 0;
        if GetDisplayConfigBufferSizes(QDC_ONLY_ACTIVE_PATHS, &mut path_count, &mut mode_count)
            != windows::Win32::Foundation::WIN32_ERROR(0)
        {
            return;
        }

        let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); path_count as usize];
        let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); mode_count as usize];

        if QueryDisplayConfig(
            QDC_ONLY_ACTIVE_PATHS,
            &mut path_count,
            paths.as_mut_ptr(),
            &mut mode_count,
            modes.as_mut_ptr(),
            None,
        ) != windows::Win32::Foundation::WIN32_ERROR(0)
        {
            return;
        }

        for path in paths.iter().take(path_count as usize) {
            let mut target_name = DISPLAYCONFIG_TARGET_DEVICE_NAME::default();
            target_name.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME;
            target_name.header.size =
                std::mem::size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32;
            target_name.header.adapterId = path.targetInfo.adapterId;
            target_name.header.id = path.targetInfo.id;

            if DisplayConfigGetDeviceInfo(&mut target_name.header) == 0 {
                let friendly_name =
                    String::from_utf16_lossy(&target_name.monitorFriendlyDeviceName)
                        .trim_matches(char::from(0))
                        .to_string();
                let device_path = String::from_utf16_lossy(&target_name.monitorDevicePath)
                    .trim_matches(char::from(0))
                    .to_string();

                println!("Target ID: {}", path.targetInfo.id);
                println!("  Friendly Name: {}", friendly_name);
                println!("  Device Path: {}", device_path);
            }
        }
    }
}
