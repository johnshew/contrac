use chrono::{DateTime, Local, NaiveDateTime, TimeZone, Utc};

use winapi::shared::minwindef::{BOOL, LPARAM, UINT, WPARAM};
use winapi::shared::windef::HWND;
use winapi::um::winuser::{
    PostMessageW, SetWindowPos, HWND_TOP, SWP_NOMOVE, /* SWP_NOOWNERZORDER, */ SWP_NOSIZE,
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

fn check_hwnd(handle: &nwg::ControlHandle) -> HWND {
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
#[allow(non_snake_case)]
pub fn PostMessage(
    control_handle: &nwg::ControlHandle,
    Msg: UINT,
    wParam: WPARAM,
    lParam: LPARAM,
) -> BOOL {
    let handle = check_hwnd(control_handle);
    unsafe { PostMessageW(handle, Msg, wParam, lParam) }
}

#[allow(non_snake_case)]
pub fn MoveToTop(control_handle: &nwg::ControlHandle) -> BOOL {
    let handle = check_hwnd(control_handle);
    unsafe {
        SetWindowPos(
            handle,
            HWND_TOP,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE ,
        )
    }
}
