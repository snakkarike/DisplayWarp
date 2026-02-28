use windows::Win32::Devices::FunctionDiscovery::PKEY_Device_FriendlyName;
use windows::Win32::Media::Audio::{
    AUDCLNT_SHAREMODE_SHARED, DEVICE_STATE_ACTIVE, IAudioClient, IAudioRenderClient, IMMDevice,
    IMMDeviceCollection, IMMDeviceEnumerator, MMDeviceEnumerator, eConsole, eRender,
};
use windows::Win32::System::Com::{CLSCTX_ALL, CoCreateInstance, CoTaskMemFree, STGM_READ};
use windows::core::{GUID, HSTRING, Interface, PCWSTR, Result};

// ─── IPolicyConfig COM Interface (Undocumented) ───────────────────────────────
//
// Reversed by EreTIk, widely used since Windows 7.
//
// CLSID (CoCreateInstance class):  {870af99c-171d-4f9e-af0d-e63df40c2bc9}
//   NOTE: The "4f9e" segment — many online sources mistakenly write "4f15".
//
// IID  (QueryInterface interface): {f8679f50-850a-41cf-9c72-430f290290c8}
//   NOTE: This is NOT the same value as the CLSID. Using CLSID as IID silently
//   returns a no-op stub on modern Windows instead of failing loudly.
//
// VTABLE (confirmed by C++ header, AHK ComCall(13,...), PureBasic, multiple sources):
//   Slot  0 : QueryInterface      (IUnknown)
//   Slot  1 : AddRef              (IUnknown)
//   Slot  2 : Release             (IUnknown)
//   Slot  3 : GetMixFormat
//   Slot  4 : GetDeviceFormat
//   Slot  5 : ResetDeviceFormat   ← commonly omitted, shifts everything by 1 if missing
//   Slot  6 : SetDeviceFormat
//   Slot  7 : GetProcessingPeriod
//   Slot  8 : SetProcessingPeriod
//   Slot  9 : GetShareMode
//   Slot 10 : SetShareMode
//   Slot 11 : GetPropertyValue
//   Slot 12 : SetPropertyValue
//   Slot 13 : SetDefaultEndpoint  ← confirmed slot 13
//   Slot 14 : SetEndpointVisibility

#[repr(C)]
#[allow(non_snake_case)]
struct IPolicyConfigVtbl {
    // IUnknown
    pub QueryInterface: unsafe extern "system" fn(
        this: *mut std::ffi::c_void,
        iid: *const GUID,
        interface: *mut *mut std::ffi::c_void,
    ) -> windows::core::HRESULT,
    pub AddRef: unsafe extern "system" fn(this: *mut std::ffi::c_void) -> u32,
    pub Release: unsafe extern "system" fn(this: *mut std::ffi::c_void) -> u32,
    // IPolicyConfig — stubs for slots 3-12 (we only call slot 13)
    pub GetMixFormat:
        unsafe extern "system" fn(this: *mut std::ffi::c_void) -> windows::core::HRESULT,
    pub GetDeviceFormat:
        unsafe extern "system" fn(this: *mut std::ffi::c_void) -> windows::core::HRESULT,
    pub ResetDeviceFormat:
        unsafe extern "system" fn(this: *mut std::ffi::c_void) -> windows::core::HRESULT, // slot 5 — DO NOT remove
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
    // Slot 13 — the method we actually use
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
    // IID for QueryInterface — distinct from the CLSID used in CoCreateInstance
    const IID: GUID = GUID::from_u128(0xf8679f50_850a_41cf_9c72_430f290290c8);

    pub unsafe fn set_default_endpoint(&self, psz_device_name: PCWSTR, role: i32) -> Result<()> {
        let vtable = self.0.as_raw() as *const *const IPolicyConfigVtbl;
        unsafe { ((*(*vtable)).SetDefaultEndpoint)(self.0.as_raw(), psz_device_name, role).ok() }
    }
}

unsafe impl Interface for IPolicyConfig {
    type Vtable = IPolicyConfigVtbl;
    const IID: GUID = Self::IID;
}

// CLSID for CoCreateInstance — CPolicyConfigClient
// {870af99c-171d-4f9e-af0d-e63df40c2bc9}  ← "4f9e", not "4f15"
const CLSID_POLICY_CONFIG: GUID = GUID::from_u128(0x870af99c_171d_4f9e_af0d_e63df40c2bc9);

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

// ─── Set Default Audio Device ─────────────────────────────────────────────────

pub fn set_default_audio_device(device_id: &str) -> Result<()> {
    unsafe {
        let hstring_id = HSTRING::from(device_id);
        let pcwstr_id = PCWSTR(hstring_id.as_ptr());

        let policy_config: IPolicyConfig =
            CoCreateInstance(&CLSID_POLICY_CONFIG, None, CLSCTX_ALL)?;

        // Set for all three roles: Console (0), Multimedia (1), Communications (2)
        policy_config.set_default_endpoint(pcwstr_id, 0)?;
        policy_config.set_default_endpoint(pcwstr_id, 1)?;
        policy_config.set_default_endpoint(pcwstr_id, 2)?;
    }

    Ok(())
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
