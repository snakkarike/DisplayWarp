use windows::Win32::Devices::Display::{
    DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME, DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME,
    DISPLAYCONFIG_MODE_INFO, DISPLAYCONFIG_PATH_INFO, DISPLAYCONFIG_SOURCE_DEVICE_NAME,
    DISPLAYCONFIG_TARGET_DEVICE_NAME, DisplayConfigGetDeviceInfo, GetDisplayConfigBufferSizes,
    QDC_ALL_PATHS, QDC_DATABASE_CURRENT, QueryDisplayConfig,
};
use windows::Win32::Foundation::WIN32_ERROR;

#[test]
fn test_topo() {
    unsafe {
        let mut path_count = 0;
        let mut mode_count = 0;
        if GetDisplayConfigBufferSizes(QDC_DATABASE_CURRENT, &mut path_count, &mut mode_count)
            != WIN32_ERROR(0)
        {
            println!("GetDisplayConfigBufferSizes failed");
            return;
        }

        let mut paths = vec![DISPLAYCONFIG_PATH_INFO::default(); path_count as usize];
        let mut modes = vec![DISPLAYCONFIG_MODE_INFO::default(); mode_count as usize];

        if QueryDisplayConfig(
            QDC_DATABASE_CURRENT,
            &mut path_count,
            paths.as_mut_ptr(),
            &mut mode_count,
            modes.as_mut_ptr(),
            None,
        ) != WIN32_ERROR(0)
        {
            println!("QueryDisplayConfig failed");
            return;
        }

        for path in paths.iter().take(path_count as usize) {
            // Get Source Name
            let mut source_name = DISPLAYCONFIG_SOURCE_DEVICE_NAME::default();
            source_name.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_SOURCE_NAME;
            source_name.header.size =
                std::mem::size_of::<DISPLAYCONFIG_SOURCE_DEVICE_NAME>() as u32;
            source_name.header.adapterId = path.sourceInfo.adapterId;
            source_name.header.id = path.sourceInfo.id;

            let mut device_name = String::new();
            if DisplayConfigGetDeviceInfo(&mut source_name.header) == 0 {
                device_name = String::from_utf16_lossy(&source_name.viewGdiDeviceName)
                    .trim_matches(char::from(0))
                    .to_string();
            }

            // Get Target Name
            let mut target_name = DISPLAYCONFIG_TARGET_DEVICE_NAME::default();
            target_name.header.r#type = DISPLAYCONFIG_DEVICE_INFO_GET_TARGET_NAME;
            target_name.header.size =
                std::mem::size_of::<DISPLAYCONFIG_TARGET_DEVICE_NAME>() as u32;
            target_name.header.adapterId = path.targetInfo.adapterId;
            target_name.header.id = path.targetInfo.id;

            let mut friendly_name = String::new();
            if DisplayConfigGetDeviceInfo(&mut target_name.header) == 0 {
                friendly_name = String::from_utf16_lossy(&target_name.monitorFriendlyDeviceName)
                    .trim_matches(char::from(0))
                    .to_string();
            }

            let target_available = (path.targetInfo.Anonymous.Anonymous._bitfield & 1) != 0;
            println!(
                "Path: active={}, available={}",
                (path.flags & 1) != 0,
                target_available
            );
            println!("  Source: id={}", path.sourceInfo.id);
            println!(
                "  Target: id={} name='{}' statusFlags={:08x}",
                path.targetInfo.id, friendly_name, path.targetInfo.statusFlags
            );
        }
    }
}
