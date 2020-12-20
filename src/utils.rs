#![allow(dead_code)]
#![allow(non_snake_case)]
use chrono::{DateTime, Local, NaiveDateTime, TimeZone, Utc};
use std::mem;

use winapi::shared::minwindef::{BOOL, LPARAM, LRESULT, UINT, WPARAM};
use winapi::shared::windef::HWND;

#[allow(unused_imports)]
use winapi::um::winuser::{
    PostMessageW, SendMessageW, SetWindowPos, HWND_TOP, SWP_NOMOVE, SWP_NOOWNERZORDER, SWP_NOSIZE,
};

pub fn timestamp_to_datetime(timestamp_in_nanoseconds: u128) -> DateTime<Local> {
    let date_time = NaiveDateTime::from_timestamp(
        (timestamp_in_nanoseconds / 1_000_000_000) as i64,
        (timestamp_in_nanoseconds % 1_000_000_000) as u32,
    );
    let date_time_local = DateTime::<Local>::from(DateTime::<Utc>::from_utc(date_time, Utc));
    date_time_local
}

pub fn _datetime_to_timestamp<T: TimeZone>(datetime: &DateTime<T>) -> u128 {
    (datetime.timestamp() * 1_000_000_000 + datetime.timestamp_nanos()) as u128
}

pub fn check_hwnd(handle: &nwg::ControlHandle) -> HWND {
    use winapi::um::winuser::IsWindow;

    if handle.blank() {
        panic!("not bound");
    }
    match handle.hwnd() {
        Some(hwnd) => match unsafe { IsWindow(hwnd) } {
            0 => {
                panic!("The window handle is no longer valid. This usually means the control was freed by the OS");
            }
            _ => hwnd,
        },
        None => {
            panic!("bad_handle");
        }
    }
}

pub fn PostMessage(
    control_handle: &nwg::ControlHandle,
    Msg: UINT,
    wParam: WPARAM,
    lParam: LPARAM,
) -> BOOL {
    let handle = check_hwnd(control_handle);
    unsafe { PostMessageW(handle, Msg, wParam, lParam) }
}
pub fn SendMessage(
    control_handle: &nwg::ControlHandle,
    Msg: UINT,
    wParam: WPARAM,
    lParam: LPARAM,
) -> LRESULT {
    let handle = check_hwnd(control_handle);
    unsafe { SendMessageW(handle, Msg, wParam, lParam) }
}

pub fn _MoveToTop(control_handle: &nwg::ControlHandle) -> BOOL {
    let handle = check_hwnd(control_handle);
    unsafe { SetWindowPos(handle, HWND_TOP, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE) }
}

pub fn VScrollToBottom(control_handle: &nwg::ControlHandle) -> BOOL {
    #[allow(unused_imports)]
    use winapi::um::winuser::{
        GetScrollInfo, SetScrollInfo, GET_WHEEL_DELTA_WPARAM, SB_BOTTOM, SB_CTL, SB_LINEDOWN,
        SB_LINELEFT, SB_LINERIGHT, SB_LINEUP, SB_PAGEDOWN, SB_PAGELEFT, SB_PAGERIGHT, SB_PAGEUP,
        SB_THUMBTRACK, SB_TOP, SB_VERT, SCROLLINFO, SIF_ALL, SIF_POS, WM_HSCROLL, WM_MOUSEWHEEL,
        WM_VSCROLL,
    };
    PostMessage(control_handle, WM_VSCROLL, SB_BOTTOM as WPARAM, 0)
}

pub fn ScrollToBottom(control_handle: &nwg::ControlHandle) {
    #[allow(unused_imports)]
    use winapi::shared::{
        minwindef::{LOWORD, TRUE},
        windef::HWND,
    };
    #[allow(unused_imports)]
    use winapi::um::winuser::{
        GetScrollInfo, SetScrollInfo, GET_WHEEL_DELTA_WPARAM, SB_BOTTOM, SB_CTL, SB_LINEDOWN,
        SB_LINELEFT, SB_LINERIGHT, SB_LINEUP, SB_PAGEDOWN, SB_PAGELEFT, SB_PAGERIGHT, SB_PAGEUP,
        SB_THUMBTRACK, SB_TOP, SB_VERT, SCROLLINFO, SIF_ALL, SIF_POS, WM_HSCROLL, WM_MOUSEWHEEL,
        WM_VSCROLL,
    };
    let handle = check_hwnd(control_handle);
    let mut si: SCROLLINFO;
    unsafe {
        si = mem::zeroed();
    }
    si.cbSize = mem::size_of::<SCROLLINFO>() as u32;
    si.fMask = SIF_ALL;
    unsafe {
        GetScrollInfo(handle, SB_VERT as i32, &mut si);
    }
    si.nPos = si.nMax;
    si.fMask = SIF_POS;
    unsafe {
        SetScrollInfo(handle, SB_VERT as _, &si, TRUE);
    }
}
