// #![allow(dead_code)]

use chrono::{DateTime, Duration, Local};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::fs::File;
use std::io::prelude::*;
use std::net::IpAddr;
use std::time::{SystemTime, UNIX_EPOCH};
use winping::{Buffer, Pinger};
use winapi::um::winuser::{ WM_SYSCOMMAND, SC_RESTORE};

extern crate native_windows_derive as nwd;
extern crate native_windows_gui as nwg;
use nwd::NwgUi;
use nwg::stretch::{
    geometry::{Rect, Size},
    style::{AlignItems, Dimension as D, FlexDirection, JustifyContent},
};
use nwg::NativeUi;


const PT_10: D = D::Points(10.0);
const PT_0: D = D::Points(0.0);
const PAD_10: Rect<D> = Rect {
    start: PT_10,
    end: PT_10,
    top: PT_10,
    bottom: PT_10,
};
const PAD_10_TOP_BOTTON: Rect<D> = Rect {
    start: PT_0,
    end: PT_0,
    top: PT_10,
    bottom: PT_10,
};

mod graph;
mod stats;
mod utils;

use crate::graph::*;

pub struct AppData {
    count: u32,
    total: u32,
    min: u16,
    max: u16,
    probes: VecDeque<(u128, Option<u16>)>,
    last_full_update: DateTime<Local>,
    last_sample_timeout: bool,
}

impl Default for AppData {
    fn default() -> Self {
        AppData {
            count: 0,
            total: 0,
            min: u16::MAX,
            max: 0,
            probes: VecDeque::new(),
            last_full_update: Local::now(),
            last_sample_timeout: false,
        }
    }
}

impl AppData {
    fn average(&self) -> f32 {
        return self.total as f32 / self.count as f32;
    }

    fn record_observation(&mut self, timestamp_in_nano: u128, response_time_in_milli: Option<u16>) {
        if let Some(ping) = response_time_in_milli {
            self.count += 1;
            if ping < self.min {
                self.min = ping;
            };
            if ping > self.max {
                self.max = ping;
            };
            self.total += ping as u32;
        }
        self.probes
            .push_back((timestamp_in_nano, response_time_in_milli));
    }
}

#[derive(Default, NwgUi)]
pub struct BasicApp {
    data: RefCell<AppData>,

    #[nwg_control(size: (600, 400), position: (300, 300), title: "Connection Tracker", flags: "MAIN_WINDOW|VISIBLE")]
    #[nwg_events( OnWindowClose: [BasicApp::on_window_close], OnInit: [BasicApp::on_window_init], OnWindowMinimize: [BasicApp::on_window_minimize] )]
    window: nwg::Window,

    #[nwg_control(interval: 1000, stopped: false)]
    #[nwg_events( OnTimerTick: [BasicApp::on_timer_tick] )]
    timer: nwg::Timer,

    #[nwg_resource(source_file: Some("./resources/cog.ico"))]
    icon: nwg::Icon,

    #[nwg_control(icon: Some(&data.icon), tip: Some("Connection Tracker"))]
    #[nwg_events(MousePressLeftUp: [BasicApp::on_tray_mouse_press_left_up], OnContextMenu: [BasicApp::on_tray_show_menu])]
    tray: nwg::TrayNotification,

    #[nwg_control(parent: window, popup: true)]
    tray_menu: nwg::Menu,

    #[nwg_control(parent: tray_menu, text: "Save Reports")]
    #[nwg_events(OnMenuItemSelected: [BasicApp::on_save_report_menu_item_selected])]
    tray_item1: nwg::MenuItem,

    // Main UX
    #[nwg_layout(parent: window, padding: PAD_10, auto_spacing: None, flex_direction: FlexDirection::Column, justify_content: JustifyContent::Center)]
    main_layout: nwg::FlexboxLayout,

    #[nwg_control] // maybe? ( flags:"BORDER")]
    #[nwg_layout_item(layout: main_layout,  min_size: Size { width: D::Percent(1.0), height: D::Points(100.0) }, size: Size { width: D::Percent(1.0), height: D::Points(1000.0)})]
    graph_frame: nwg::Frame,

    #[nwg_partial(parent: graph_frame)]
    graph: GraphUi,

    #[nwg_control( flags:"VISIBLE|HORIZONTAL|RANGE")]
    #[nwg_layout_item(layout: main_layout, min_size: Size { width: D::Percent(1.0), height: D::Points(40.0)}, max_size: Size { width: D::Percent(1.0), height: D::Points(40.0)})]
    slider: nwg::TrackBar,

    #[nwg_control]
    #[nwg_layout_item(layout: main_layout,  min_size: Size { width: D::Percent(1.0), height: D::Points(40.0) }, max_size: Size { width: D::Percent(1.0), height: D::Points(40.0) },)]
    message: nwg::Label,

    #[nwg_control(parent: window, flags: "VISIBLE")]
    #[nwg_layout_item(layout: main_layout, min_size: Size { width: D::Percent(1.0), height: D::Points(60.0)}, max_size: Size { width: D::Percent(1.0), height: D::Points(60.0)})]
    button_frame: nwg::Frame,

    #[nwg_layout(parent: button_frame, padding: PAD_10_TOP_BOTTON,  auto_spacing: None, flex_direction: FlexDirection::Row, align_items: AlignItems::Center, justify_content: JustifyContent::FlexEnd)]
    button_layout: nwg::FlexboxLayout,

    #[nwg_control(parent: button_frame, text: "Reset")]
    #[nwg_layout_item(layout: button_layout,  size: Size { width: D::Points(150.0), height: D::Points(40.0) },)]
    #[nwg_events( OnButtonClick: [BasicApp::on_reset_click] )]
    reset_button: nwg::Button,

    #[nwg_control(parent: button_frame, text: "Save Reports")]
    #[nwg_layout_item(layout: button_layout,  size: Size { width: D::Points(150.0), height: D::Points(40.0) },)]
    #[nwg_events( OnButtonClick: [BasicApp::on_save_report_menu_item_selected] )]
    save_report_button: nwg::Button,
}

impl BasicApp {
    fn on_window_init(&self) {
        self.slider.set_range_min(0);
        self.slider.set_range_max(100);
        self.graph.init(30, 0, 20);
        self.graph.on_resize();
    }

    fn on_reset_click(&self) {
        let mut data = self.data.borrow_mut();
        data.count = 0;
        data.total = 0;
        data.min = u16::MAX;
        data.max = 0;
        data.probes.clear();
    }

    fn on_window_close(&self) {
        self.write_log().expect("Problem writing logs");
        nwg::stop_thread_dispatch();
    }

    fn on_window_minimize(&self) {
        self.window.set_visible(false);
    }

    fn on_timer_tick(&self) {
        let pinger = Pinger::new().unwrap();
        let mut buffer = Buffer::new();
        let dst = String::from("1.1.1.1")
            .parse::<IpAddr>()
            .expect("Could not parse IP Address");
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Bad time value")
            .as_nanos();
        let ping_response;

        match pinger.send(dst, &mut buffer) {
            Ok(rtt) => {
                ping_response = Some(rtt as u16);
            }
            Err(_err) => {
                ping_response = None;
            }
        };

        // Update data and UX
        {
            let mut data = self.data.borrow_mut();
            data.record_observation(timestamp, ping_response);
            if let Some(rtt) = ping_response {
                data.last_sample_timeout = false;
                let message = format!(
                    "{} ({}:{}) {:.1}ms avg",
                    rtt,
                    data.min,
                    data.max,
                    data.average()
                );
                self.message.set_text(&message);
                self.slider.set_pos(rtt as usize);
                self.slider
                    .set_selection_range_pos(data.min as usize..data.max as usize);
            } else {
                self.message.set_text("Disconnected");
                self.slider.set_pos(300);
                if !data.last_sample_timeout {
                    self.display_notification("Disconnected");
                }
                data.last_sample_timeout = true;
            }
            self.graph.set_values(&data.probes);
            let datetime = utils::timestamp_to_datetime(timestamp);
            if datetime > (data.last_full_update + Duration::milliseconds(250)) {
                self.graph.on_resize();
                data.last_full_update = datetime;
            }
        }
    }

    fn on_tray_show_menu(&self) {
        let (x, y) = nwg::GlobalCursor::position();
        self.tray_menu.popup(x, y);
    }

    fn on_tray_mouse_press_left_up(&self) {
        self.window.set_visible(true);
        utils::PostMessage(&self.window.handle, WM_SYSCOMMAND, SC_RESTORE, 0);
    }

    fn write_log(&self) -> std::io::Result<()> {
        {
            let mut f = File::create("log.txt")?;
            let data = self.data.borrow();
            for (time, rtt) in &data.probes {
                let date_time = utils::timestamp_to_datetime(*time);
                let result = if rtt.is_some() {
                    rtt.unwrap().to_string()
                } else {
                    String::from("timeout")
                };
                let message = format!("{:0}, {}\r\n", date_time, result);
                let message = message.as_bytes();
                f.write_all(message).unwrap();
            }
            f.sync_all()?;
        }

        {
            enum TimeoutTracker {
                Active { start: DateTime<Local> },
                Nominal,
            };

            let mut f = File::create("timeouts.txt")?;
            let data = self.data.borrow();
            let mut timeout_status = TimeoutTracker::Nominal;

            for (time, rtt) in &data.probes {
                if rtt.is_some() {
                    if let TimeoutTracker::Active { start } = timeout_status {
                        let end = utils::timestamp_to_datetime(*time);
                        let offline_duration = (end-start).num_milliseconds() as f32 / 1_000.0;
                        let message = format!("{}, {}, {}\r\n", start, end, offline_duration);
                        f.write_all(message.as_bytes()).unwrap();
                        timeout_status = TimeoutTracker::Nominal;
                    } else {
                        continue;
                    }
                } else {
                    if let TimeoutTracker::Active { start: _ } = timeout_status {
                        continue;
                    } else {
                        timeout_status = TimeoutTracker::Active {
                            start: utils::timestamp_to_datetime(*time),
                        }
                    }
                }
            }
            f.sync_all()?;
        }
        Ok(())
    }

    fn on_save_report_menu_item_selected(&self) {
        self.write_log().unwrap();
        let message = format!("Report saved");
        self.display_notification(&message);
    }

    fn display_notification(&self, message: &str) {
        let flags = nwg::TrayNotificationFlags::USER_ICON | nwg::TrayNotificationFlags::LARGE_ICON;
        self.tray
            .show("Status", Some(message), Some(flags), Some(&self.icon));
    }
}

fn main() -> std::io::Result<()> {
    nwg::init().expect("Failed to init Native Windows GUI");
    nwg::Font::set_global_family("Segoe UI").expect("Failed to set default font");

    let _app = BasicApp::build_ui(Default::default()).expect("Failed to build UI");
    nwg::dispatch_thread_events();
    Ok(())
}
