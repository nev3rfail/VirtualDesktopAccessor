use once_cell::sync::Lazy;
use std::{
    collections::HashMap,
    ffi::{CStr, CString},
    sync::{Arc, Mutex},
    thread,
};
use windows::{core::GUID, Win32::Foundation::HWND};
use winvd::*;

type HWND_ = u32;

#[no_mangle]
pub extern "C" fn GetCurrentDesktopNumber() -> i32 {
    get_current_desktop().map_or(-1, |x| x.get_index().map_or(-1, |x| x as i32))
}

// #[no_mangle]
// pub extern "C" fn GetDesktopNumber() -> i32 {
//     get_current_desktop_index_OLDD().map_or(-1, |x| x as i32)
// }

#[no_mangle]
pub extern "C" fn GetDesktopCount() -> i32 {
    get_desktop_count().map_or(-1, |x| x as i32)
}

#[no_mangle]
pub extern "C" fn GetDesktopIdByNumber(number: i32) -> GUID {
    if number < 0 {
        return GUID::default();
    }
    get_desktop(number).get_id().map_or(GUID::default(), |x| x)
}

#[no_mangle]
pub extern "C" fn GetDesktopNumberById(desktop_id: GUID) -> i32 {
    get_desktop(&desktop_id)
        .get_index()
        .map_or(-1, |x| x as i32)
}

#[no_mangle]
pub extern "C" fn GetWindowDesktopId(hwnd: HWND) -> GUID {
    get_desktop_by_window(hwnd).map_or(GUID::default(), |x| {
        x.get_id().map_or(GUID::default(), |y| y)
    })
}

#[no_mangle]
pub extern "C" fn GetWindowDesktopNumber(hwnd: HWND) -> i32 {
    get_desktop_by_window(hwnd).map_or(-1, |x| x.get_index().map_or(-1, |y| y as i32))
}

#[no_mangle]
pub extern "C" fn IsWindowOnCurrentVirtualDesktop(hwnd: HWND) -> i32 {
    is_window_on_current_desktop(hwnd).map_or(-1, |x| x as i32)
}

#[no_mangle]
pub extern "C" fn MoveWindowToDesktopNumber(hwnd: HWND, desktop_number: i32) -> i32 {
    move_window_to_desktop(desktop_number as u32, &hwnd).map_or(-1, |_| 1)
}

#[no_mangle]
pub extern "C" fn GoToDesktopNumber(desktop_number: i32) {
    switch_desktop(desktop_number as u32).unwrap_or_default()
}

#[no_mangle]
pub extern "C" fn SetDesktopName(desktop_number: i32, in_name_ptr: *const i8) -> i32 {
    let name_str = unsafe { CStr::from_ptr(in_name_ptr).to_string_lossy() };
    get_desktop(desktop_number)
        .set_name(&name_str)
        .map_or(-1, |_| 1)
}

#[no_mangle]
pub extern "C" fn GetDesktopName(
    desktop_number: i32,
    out_utf8_ptr: *mut u8,
    out_utf8_len: usize,
) -> i32 {
    if let Ok(name) = get_desktop(desktop_number).get_name() {
        let name_str = CString::new(name).unwrap();
        let name_bytes = name_str.as_bytes_with_nul();
        if name_bytes.len() > out_utf8_len {
            return -1;
        }
        unsafe {
            out_utf8_ptr.copy_from(name_bytes.as_ptr(), name_bytes.len());
        }
        1
    } else {
        0
    }
}

static LISTENER_HWNDS: Lazy<Arc<Mutex<HashMap<HWND_, u32>>>> =
    Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

#[no_mangle]
pub extern "C" fn RegisterPostMessageHook(listener_hwnd: HWND, message_offset: u32) {
    let mut a = LISTENER_HWNDS.lock().unwrap();

    // !! TODO: START A LISTENER

    a.insert(listener_hwnd.0 as u32, message_offset);
}

#[no_mangle]
pub extern "C" fn UnregisterPostMessageHook(listener_hwnd: HWND) {
    let mut a = LISTENER_HWNDS.lock().unwrap();
    a.remove(&(listener_hwnd.0 as u32));
    if a.len() == 0 {
        // !! TODO: DROP A LISTENER
    }
}
#[no_mangle]
pub extern "C" fn IsPinnedWindow(hwnd: HWND) -> i32 {
    is_pinned_window(hwnd).map_or(-1, |x| x as i32)
}
#[no_mangle]
pub extern "C" fn PinWindow(hwnd: HWND) {
    pin_window(hwnd).unwrap_or_default()
}
#[no_mangle]
pub extern "C" fn UnPinWindow(hwnd: HWND) {
    unpin_window(hwnd).unwrap_or_default()
}
#[no_mangle]
pub extern "C" fn IsPinnedApp(hwnd: HWND) -> i32 {
    is_pinned_app(hwnd).map_or(-1, |x| x as i32)
}
#[no_mangle]
pub extern "C" fn PinApp(hwnd: HWND) {
    pin_app(hwnd).unwrap_or_default()
}
#[no_mangle]
pub extern "C" fn UnPinApp(hwnd: HWND) {
    unpin_app(hwnd).unwrap_or_default()
}
#[no_mangle]
pub extern "C" fn IsWindowOnDesktopNumber(hwnd: HWND, desktop_number: i32) -> i32 {
    is_window_on_desktop(desktop_number, hwnd).map_or(-1, |b| b as i32)
}

#[no_mangle]
pub extern "C" fn CreateDesktop() -> i32 {
    if let Ok(desk) = create_desktop() {
        desk.get_index().map_or(-1, |x| x as i32)
    } else {
        -1
    }
}

#[no_mangle]
pub extern "C" fn RemoveDesktop(remove_desktop_number: i32, fallback_desktop_number: i32) -> i32 {
    if remove_desktop_number == fallback_desktop_number {
        return -1;
    }
    remove_desktop(remove_desktop_number, fallback_desktop_number).map_or(-1, |_| 1)
}

#[no_mangle]
pub extern "C" fn RestartVirtualDesktopAccessor() {
    // ?
}

#[link(name = "User32")]
extern "system" {
    pub fn PostMessageW(inOptHwnd: HWND, inMsg: u32, inWParam: u32, inLParam: i32) -> bool;
    pub fn SendMessageA(inOptHwnd: HWND, inMsg: u32, inWParam: u32, inLParam: i32) -> bool;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dll_get_desktop_name() {
        // Allocate a buffer for the UTF-8 string
        let utf8_buffer_1024 = [0u8; 1024];
        let utf8_buffer_1024_ptr = utf8_buffer_1024.as_ptr() as *mut u8;
        let res = GetDesktopName(0, utf8_buffer_1024_ptr, 1024);

        let name_cstr = unsafe { std::ffi::CStr::from_ptr(utf8_buffer_1024_ptr as *const i8) };
        let name_str = name_cstr.to_str().unwrap();

        assert_eq!(res, 1);
        assert_eq!(name_str, "Oma");
    }
    #[test]
    fn test_dll_set_desktop_name() {
        let current_desktop_name = get_desktop(0).get_name().unwrap();
        let name = "Testi 😉";
        assert_ne!(current_desktop_name, name);

        let name_cstr = std::ffi::CString::new(name).unwrap();
        let res = SetDesktopName(0, name_cstr.as_ptr() as *mut i8);
        let new_name = get_desktop(0).get_name().unwrap();
        get_desktop(0).set_name(&current_desktop_name).unwrap();
        assert_eq!(new_name, name);
        assert_eq!(res, 1);
    }

    #[test]
    fn test_create_desktop() {
        // Creation works
        let count = GetDesktopCount();
        let new_desk_index = CreateDesktop();
        let new_count = GetDesktopCount();
        assert_eq!(count + 1, new_count);

        // Removing works
        let did_it_work = RemoveDesktop(new_desk_index, 0);
        assert_eq!(did_it_work, 1);
        let after_count = GetDesktopCount();
        assert_eq!(count, after_count);
    }
}
/*
* int GetCurrentDesktopNumber()
* int GetDesktopCount()
* GUID GetDesktopIdByNumber(int number) // Returns zeroed GUID with invalid number found
* int GetDesktopNumber(IVirtualDesktop *pDesktop)
* int GetDesktopNumberById(GUID desktopId)
* GUID GetWindowDesktopId(HWND window)
* int GetWindowDesktopNumber(HWND window)
* int IsWindowOnCurrentVirtualDesktop(HWND window)
* BOOL MoveWindowToDesktopNumber(HWND window, int number)
* void GoToDesktopNumber(int number)
* void RegisterPostMessageHook(HWND listener, int messageOffset)
* void UnregisterPostMessageHook(HWND hwnd)
* int IsPinnedWindow(HWND hwnd) // Returns 1 if pinned, 0 if not pinned, -1 if not valid
* void PinWindow(HWND hwnd)
* void UnPinWindow(HWND hwnd)
* int IsPinnedApp(HWND hwnd) // Returns 1 if pinned, 0 if not pinned, -1 if not valid
* void PinApp(HWND hwnd)
* void UnPinApp(HWND hwnd)
* int IsWindowOnDesktopNumber(HWND window, int number) /
* void RestartVirtualDesktopAccessor() // Call this during taskbar created message

*/
