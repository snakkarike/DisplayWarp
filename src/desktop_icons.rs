use serde::{Deserialize, Serialize};
use windows::core::{Interface, Result};
use windows::Win32::Foundation::POINT;
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL, IServiceProvider};
use windows::Win32::System::SystemInformation::GetLocalTime;
use windows::Win32::System::Variant::VARIANT;
use windows::Win32::UI::Shell::{
    IEnumIDList, IFolderView, IShellBrowser, IShellFolder, IShellView, IShellWindows,
    SHGDN_NORMAL, SID_STopLevelBrowser, SVGIO_ALLVIEW, SWC_DESKTOP, SWFO_NEEDDISPATCH,
    ShellWindows, StrRetToBSTR,
};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DesktopIconPos {
    pub name: String,
    pub x: i32,
    pub y: i32,
    #[serde(default)]
    pub category_id: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct DesktopIconLayout {
    pub name: String,
    #[serde(default)]
    pub created_at: String,
    pub icons: Vec<DesktopIconPos>,
}

pub fn current_timestamp_string() -> String {
    let st = unsafe { GetLocalTime() };
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        st.wYear, st.wMonth, st.wDay, st.wHour, st.wMinute, st.wSecond
    )
}

pub unsafe fn get_desktop_folder_view() -> Result<(IFolderView, IShellFolder)> {
    unsafe {
        let shell_windows: IShellWindows = CoCreateInstance(&ShellWindows, None, CLSCTX_ALL)?;

        let vt_loc = VARIANT::from(0i32); // CSIDL_DESKTOP = 0
        let vt_empty = VARIANT::default();
        let mut hwnd: i32 = 0;

        let disp = shell_windows.FindWindowSW(
            &vt_loc,
            &vt_empty,
            SWC_DESKTOP,
            &mut hwnd,
            SWFO_NEEDDISPATCH,
        )?;

        let provider: IServiceProvider = disp.cast()?;
        let browser: IShellBrowser = provider.QueryService(&SID_STopLevelBrowser)?;
        let view: IShellView = browser.QueryActiveShellView()?;
        let folder_view: IFolderView = view.cast()?;
        let folder: IShellFolder = folder_view.GetFolder()?;

        Ok((folder_view, folder))
    }
}

pub fn capture_desktop_icons() -> Result<Vec<DesktopIconPos>> {
    let mut icons = Vec::new();
    unsafe {
        let _ = windows::Win32::System::Com::CoInitializeEx(
            None,
            windows::Win32::System::Com::COINIT_APARTMENTTHREADED,
        );
        let (folder_view, folder) = get_desktop_folder_view()?;
        let enm: IEnumIDList = folder_view.Items(SVGIO_ALLVIEW)?;

        loop {
            let mut fetched = 0;
            let mut pidls = [std::ptr::null_mut()];
            let hr = enm.Next(&mut pidls, Some(&mut fetched));
            let pidl = pidls[0];
            if hr != windows::core::HRESULT(0) || fetched == 0 || pidl.is_null() {
                break;
            }
            let mut strret: windows::Win32::UI::Shell::Common::STRRET = std::mem::zeroed();
            if folder.GetDisplayNameOf(pidl, SHGDN_NORMAL, &mut strret).is_ok() {
                let mut bstr = windows::core::BSTR::new();
                if StrRetToBSTR(&mut strret, Some(pidl), &mut bstr).is_ok() {
                    let name = bstr.to_string();
                    if let Ok(pt) = folder_view.GetItemPosition(pidl) {
                        icons.push(DesktopIconPos {
                            name,
                            x: pt.x,
                            y: pt.y,
                            category_id: None,
                        });
                    }
                }
            }
            windows::Win32::System::Com::CoTaskMemFree(Some(pidl as *mut std::ffi::c_void));
        }
    }
    Ok(icons)
}

pub fn restore_desktop_icons(icons: &[DesktopIconPos]) -> Result<usize> {
    let mut restored_count = 0;
    unsafe {
        let _ = windows::Win32::System::Com::CoInitializeEx(
            None,
            windows::Win32::System::Com::COINIT_APARTMENTTHREADED,
        );
        let (folder_view, folder) = get_desktop_folder_view()?;
        let enm: IEnumIDList = folder_view.Items(SVGIO_ALLVIEW)?;

        loop {
            let mut fetched = 0;
            let mut pidls = [std::ptr::null_mut()];
            let hr = enm.Next(&mut pidls, Some(&mut fetched));
            let pidl = pidls[0];
            if hr != windows::core::HRESULT(0) || fetched == 0 || pidl.is_null() {
                break;
            }
            let mut strret: windows::Win32::UI::Shell::Common::STRRET = std::mem::zeroed();
            if folder.GetDisplayNameOf(pidl, SHGDN_NORMAL, &mut strret).is_ok() {
                let mut bstr = windows::core::BSTR::new();
                if StrRetToBSTR(&mut strret, Some(pidl), &mut bstr).is_ok() {
                    let name = bstr.to_string();
                    if let Some(target) = icons.iter().find(|i| i.name.eq_ignore_ascii_case(&name)) {
                        let pt = POINT { x: target.x, y: target.y };
                        let apidl = [pidl as *const windows::Win32::UI::Shell::Common::ITEMIDLIST];
                        // SVSI_POSITIONITEM = 0x00000001
                        let res = folder_view.SelectAndPositionItems(1, apidl.as_ptr(), Some(&pt), 0x00000001);
                        if res.is_ok() {
                            restored_count += 1;
                        }
                    }
                }
            }
            windows::Win32::System::Com::CoTaskMemFree(Some(pidl as *mut std::ffi::c_void));
        }
    }
    Ok(restored_count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capture_and_restore() {
        let icons = capture_desktop_icons().expect("capture icons");
        assert!(!icons.is_empty(), "Should capture at least 1 icon");
        let restored = restore_desktop_icons(&icons).expect("restore icons");
        assert_eq!(restored, icons.len());
    }

    #[test]
    fn test_timestamp() {
        let ts = current_timestamp_string();
        assert!(!ts.is_empty());
        assert_eq!(ts.len(), 19);
    }
}
