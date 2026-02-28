use std::os::windows::process::CommandExt;
use windows::core::{Interface, Result, GUID, HSTRING, PCWSTR};
use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
use windows::Win32::Media::Audio::{
    eConsole, eRender, IAudioClient, IAudioRenderClient, IMMDevice, IMMDeviceCollection,
    IMMDeviceEnumerator, MMDeviceEnumerator, AUDCLNT_SHAREMODE_SHARED, DEVICE_STATE_ACTIVE,
};
use windows::Win32::System::Com::{CoCreateInstance, CoTaskMemFree, CLSCTX_ALL, STGM_READ};

// ─── IPolicyConfig COM Interface (Undocumented) ──────────────────────────────

#[repr(C)]
#[allow(non_snake_case)]
struct IPolicyConfigVtbl {
    pub QueryInterface: unsafe extern "system" fn(
        this: *mut std::ffi::c_void,
        iid: *const GUID,
        interface: *mut *mut std::ffi::c_void,
    ) -> windows::core::HRESULT,
    pub AddRef: unsafe extern "system" fn(this: *mut std::ffi::c_void) -> u32,
    pub Release: unsafe extern "system" fn(this: *mut std::ffi::c_void) -> u32,
    pub GetDeviceFormat:
        unsafe extern "system" fn(this: *mut std::ffi::c_void) -> windows::core::HRESULT,
    pub SetDeviceFormat:
        unsafe extern "system" fn(this: *mut std::ffi::c_void) -> windows::core::HRESULT,
    pub GetProcessingPeriod:
        unsafe extern "system" fn(this: *mut std::ffi::c_void) -> windows::core::HRESULT,
    pub SetProcessingPeriod:
        unsafe extern "system" fn(this: *mut std::ffi::c_void) -> windows::core::HRESULT,
    pub GetShareMode:
        unsafe extern "system" fn(this: *mut std::ffi::c_void) -> windows::core::HRESULT,
    pub SetShareMode:
        unsafe extern "system" fn(this: *mut std::ffi::c_void) -> windows::core::HRESULT,
    pub GetPropertyValue:
        unsafe extern "system" fn(this: *mut std::ffi::c_void) -> windows::core::HRESULT,
    pub SetPropertyValue:
        unsafe extern "system" fn(this: *mut std::ffi::c_void) -> windows::core::HRESULT,
    pub SetDefaultEndpoint: unsafe extern "system" fn(
        this: *mut std::ffi::c_void,
        pszDeviceName: PCWSTR,
        role: i32,
    ) -> windows::core::HRESULT,
}

#[repr(transparent)]
#[derive(Clone)]
struct IPolicyConfig(windows::core::IUnknown);

impl IPolicyConfig {
    pub const IID: GUID = GUID::from_u128(0x870af99c_171d_4f15_af0d_73ed80116693);

    pub unsafe fn set_default_endpoint(&self, psz_device_name: PCWSTR, role: i32) -> Result<()> {
        let vtable = self.0.as_raw() as *const *const IPolicyConfigVtbl;
        unsafe { ((*(*vtable)).SetDefaultEndpoint)(self.0.as_raw(), psz_device_name, role).ok() }
    }
}

unsafe impl Interface for IPolicyConfig {
    type Vtable = IPolicyConfigVtbl;
    const IID: GUID = Self::IID;
}

// ─── Device Enumeration ───────────────────────────────────────────────────────

#[derive(Clone, Debug, Default)]
pub struct AudioDeviceInfo {
    pub id: String,
    pub name: String,
}

pub fn get_audio_output_devices() -> Result<Vec<AudioDeviceInfo>> {
    let mut devices = Vec::new();

    unsafe {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let collection: IMMDeviceCollection =
            enumerator.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)?;
        let count = collection.GetCount()?;

        for i in 0..count {
            let device: IMMDevice = collection.Item(i)?;

            let id_pwstr = device.GetId()?;
            let id = id_pwstr
                .to_string()
                .unwrap_or_else(|_| format!("device-{i}"));
            CoTaskMemFree(Some(id_pwstr.0 as *const _ as *mut _));

            let name = get_device_friendly_name(&device).unwrap_or_else(|_| id.clone());

            devices.push(AudioDeviceInfo { id, name });
        }
    }

    Ok(devices)
}

unsafe fn get_device_friendly_name(device: &IMMDevice) -> Result<String> {
    use windows::Win32::System::Com::StructuredStorage::PropVariantClear;
    use windows::Win32::System::Variant::VT_LPWSTR;

    let store = unsafe { device.OpenPropertyStore(STGM_READ) }?;
    let mut prop = unsafe { store.GetValue(&PKEY_Device_FriendlyName) }?;

    let vt = unsafe { prop.Anonymous.Anonymous.vt };
    if vt != VT_LPWSTR {
        unsafe {
            let _ = PropVariantClear(&mut prop);
        }
        return Err(windows::core::Error::empty());
    }

    let raw_ptr = unsafe { prop.Anonymous.Anonymous.Anonymous.pwszVal };
    let name = if raw_ptr.is_null() {
        String::new()
    } else {
        unsafe {
            let mut len = 0usize;
            while *raw_ptr.0.add(len) != 0 {
                len += 1;
            }
            HSTRING::from_wide(std::slice::from_raw_parts(raw_ptr.0, len)).to_string()
        }
    };

    unsafe {
        let _ = PropVariantClear(&mut prop);
    }
    Ok(name)
}

// ─── Default Device ───────────────────────────────────────────────────────────

#[allow(dead_code)]
pub fn get_default_audio_device_id() -> Result<String> {
    unsafe {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let device = enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;
        let id_pwstr = device.GetId()?;
        let id = id_pwstr.to_string().unwrap_or_default();
        CoTaskMemFree(Some(id_pwstr.0 as *const _ as *mut _));
        Ok(id)
    }
}

pub fn set_default_audio_device(device_id: &str) -> Result<()> {
    // First try COM (IPolicyConfig) with all known CLSIDs
    const CLSIDS: &[u128] = &[
        0x294935ce_f637_4e7c_a41b_abed1990e54c,
        0x6be54be8_a068_4875_a49d_0c2966473b90,
        0xbcde0395_e52f_467c_8e3d_c4579291692e,
        0x870af99c_171d_4f15_af0d_73ed80116693,
    ];

    unsafe {
        let hstring_id = HSTRING::from(device_id);
        let pcwstr_id = PCWSTR(hstring_id.as_ptr());

        for &clsid_u128 in CLSIDS {
            let clsid = GUID::from_u128(clsid_u128);
            if let Ok(policy_config) =
                CoCreateInstance::<_, IPolicyConfig>(&clsid, None, CLSCTX_ALL)
            {
                policy_config.set_default_endpoint(pcwstr_id, 0)?;
                policy_config.set_default_endpoint(pcwstr_id, 1)?;
                policy_config.set_default_endpoint(pcwstr_id, 2)?;
                return Ok(());
            }
        }
    }

    // COM failed — fall back to PowerShell via AudioDeviceCmdlets
    // This works on all Windows 10/11 systems
    let script = format!(
        r#"
$deviceId = '{}'
$code = @'
using System.Runtime.InteropServices;
[Guid("870AF99C-171D-4F15-AF0D-73ED80116693")]
[InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
interface IPolicyConfig {{
    void unused0(); void unused1(); void unused2(); void unused3();
    void unused4(); void unused5(); void unused6(); void unused7();
    void unused8(); void unused9();
    [PreserveSig] int SetDefaultEndpoint([MarshalAs(UnmanagedType.LPWStr)] string deviceId, int role);
}}
[Guid("294935CE-F637-4E7C-A41B-ABED1990E54C")]
[ClassInterface(ClassInterfaceType.None)]
[ComImport]
class CPolicyConfig {{}}
public class AudioSwitcher {{
    public static void SetDefault(string id) {{
        var cfg = (IPolicyConfig)new CPolicyConfig();
        cfg.SetDefaultEndpoint(id, 0);
        cfg.SetDefaultEndpoint(id, 1);
        cfg.SetDefaultEndpoint(id, 2);
    }}
}}
'@
Add-Type -TypeDefinition $code
[AudioSwitcher]::SetDefault($deviceId)
"#,
        device_id.replace('\'', "''")
    );

    let output = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-WindowStyle",
            "Hidden",
            "-ExecutionPolicy",
            "Bypass",
            "-Command",
            &script,
        ])
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .output()
        .map_err(|_e| {
            windows::core::Error::from_hresult(windows::core::HRESULT(0x80004005_u32 as i32))
        })?;

    if output.status.success() {
        Ok(())
    } else {
        let _stderr = String::from_utf8_lossy(&output.stderr);
        Err(windows::core::Error::from_hresult(windows::core::HRESULT(
            0x80004005_u32 as i32,
        )))
    }
}

// ─── Test Beep via WASAPI ─────────────────────────────────────────────────────

pub fn play_test_beep(device_id: &str) -> Result<()> {
    const BEEP_DURATION_SECS: f32 = 0.4;
    const FREQ_HZ: f32 = 440.0;
    const AMPLITUDE: f32 = 0.35;

    unsafe {
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
        let collection: IMMDeviceCollection =
            enumerator.EnumAudioEndpoints(eRender, DEVICE_STATE_ACTIVE)?;
        let count = collection.GetCount()?;

        let mut target_device: Option<IMMDevice> = None;
        for i in 0..count {
            let device: IMMDevice = collection.Item(i)?;
            let id_pwstr = device.GetId()?;
            let id = id_pwstr.to_string().unwrap_or_default();
            CoTaskMemFree(Some(id_pwstr.0 as *const _ as *mut _));
            if id == device_id {
                target_device = Some(device);
                break;
            }
        }

        let device = target_device.ok_or_else(|| {
            windows::core::Error::from_hresult(windows::core::HRESULT(0x80070490_u32 as i32))
        })?;

        let audio_client: IAudioClient = device.Activate(CLSCTX_ALL, None)?;

        let mix_format_ptr = audio_client.GetMixFormat()?;
        let mix_format = &*mix_format_ptr;
        let sample_rate = mix_format.nSamplesPerSec;
        let channels = mix_format.nChannels as usize;

        let buffer_duration: i64 = 5_000_000;
        audio_client.Initialize(
            AUDCLNT_SHAREMODE_SHARED,
            0,
            buffer_duration,
            0,
            mix_format_ptr,
            None,
        )?;

        CoTaskMemFree(Some(mix_format_ptr as *mut _));

        let render_client: IAudioRenderClient = audio_client.GetService()?;
        let buffer_frame_count = audio_client.GetBufferSize()?;

        let total_frames = (sample_rate as f32 * BEEP_DURATION_SECS) as u32;
        let frames_to_write = total_frames.min(buffer_frame_count);

        let data_ptr = render_client.GetBuffer(frames_to_write)?;

        let sample_count = (frames_to_write as usize) * channels;
        let samples = std::slice::from_raw_parts_mut(data_ptr as *mut f32, sample_count);

        for frame in 0..frames_to_write as usize {
            let t = frame as f32 / sample_rate as f32;
            let envelope = if t > BEEP_DURATION_SECS - 0.02 {
                ((BEEP_DURATION_SECS - t) / 0.02).clamp(0.0, 1.0)
            } else {
                1.0
            };
            let sample = (2.0 * std::f32::consts::PI * FREQ_HZ * t).sin() * AMPLITUDE * envelope;
            for ch in 0..channels {
                samples[frame * channels + ch] = sample;
            }
        }

        render_client.ReleaseBuffer(frames_to_write, 0)?;

        audio_client.Start()?;
        let sleep_ms = (BEEP_DURATION_SECS * 1000.0) as u64 + 50;
        std::thread::sleep(std::time::Duration::from_millis(sleep_ms));
        audio_client.Stop()?;
    }

    Ok(())
}
