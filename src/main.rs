// #![allow(dead_code)]

use chrono::{DateTime, Duration, Local};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::fs::File;
use std::io::prelude::*;
use std::net::IpAddr;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread; // ::{spawn, JoinHandle};
use std::time::{SystemTime, UNIX_EPOCH};
use winapi::um::winuser::{SC_RESTORE, WM_SYSCOMMAND};
use winping::{Buffer, Pinger};

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

pub type Sample = (IpAddr, u128, Option<u16>);

pub struct AppData {
    count: u32,
    total: u32,
    min: u16,
    max: u16,
    samples: VecDeque<Sample>,
    last_full_update: DateTime<Local>,
    last_sample_timeout: bool,
    samples_receiver: Receiver<Sample>,
    samples_sender: Sender<Sample>,
}

impl Default for AppData {
    fn default() -> Self {
        let (s, r) = channel::<Sample>();
        AppData {
            count: 0,
            total: 0,
            min: u16::MAX,
            max: 0,
            samples: VecDeque::new(),
            last_full_update: Local::now(),
            last_sample_timeout: false,
            samples_receiver: r,
            samples_sender: s,
        }
    }
}

impl AppData {
    fn average(&self) -> f32 {
        return self.total as f32 / self.count as f32;
    }

    fn record_observation(&mut self, sample: Sample) {
        let (address, timestamp_in_nano, response_time_in_milli) = sample;
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
        self.samples
            .push_back((address, timestamp_in_nano, response_time_in_milli));
    }

    fn sort(&mut self) {
        self.samples
            .make_contiguous()
            .sort_by(|(_aa, at, _ap), (_ba, bt, _bp)| at.cmp(bt));
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
        data.samples.clear();
    }

    fn on_window_close(&self) {
        self.write_log();
        nwg::stop_thread_dispatch();
    }

    fn on_window_minimize(&self) {
        self.window.set_visible(false);
    }

    fn process_sample(&self, sample: Sample) {
        {
            let mut data = self.data.borrow_mut();
            data.record_observation(sample);
            let (dst, _timestamp, ping_response) = sample;
            if let Some(rtt) = ping_response {
                data.last_sample_timeout = false;
                let message = format!(
                    "{}: {} ms ({}:{}) {:.1} avg",
                    dst,
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
        }
    }

    fn on_timer_tick(&self) {
        // check for info on the channel
        let mut done = false;
        while !done {
            let sample;
            {
                let receiver = &self.data.borrow().samples_receiver;
                sample = receiver.try_recv();
            }
            if let Ok(s) = sample {
                self.process_sample(s);
            } else {
                done = true;
            }
        }

        let datetime = Local::now();
        let mut data = self.data.borrow_mut();
        if datetime > (data.last_full_update + Duration::milliseconds(250)) {
            data.sort();
            self.graph.set_values(&data.samples);
            self.graph.on_resize();
            data.last_full_update = datetime;
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

    fn write_log(&self) {
        {
            let mut data = self.data.borrow_mut();
            data.sort();
        }
        let data = self.data.borrow();
        let mut file = File::create("log.txt").expect("file create failed");

        for (address, time, rtt) in &data.samples {
            let date_time = utils::timestamp_to_datetime(*time);
            let result = if rtt.is_some() {
                rtt.unwrap().to_string()
            } else {
                String::from("timeout")
            };
            let message = format!("{:0}, {}, {}\r\n", date_time, result, address);
            let message = message.as_bytes();
            file.write_all(message).unwrap();
        }
        file.sync_all().expect("file sync failed");

        enum TimeoutTracker {
            Active { start: DateTime<Local> },
            Nominal,
        };

        let mut file = File::create("timeouts.txt").expect("file create failed");
        let mut timeout_status = TimeoutTracker::Nominal;
        // let samples = data.samples.sort_by()

        for (_address, time, rtt) in &data.samples {
            if rtt.is_some() {
                if let TimeoutTracker::Active { start } = timeout_status {
                    let end = utils::timestamp_to_datetime(*time);
                    let offline_duration = (end - start).num_milliseconds() as f32 / 1_000.0;
                    timeout_status = TimeoutTracker::Nominal;
                    // if offline_duration < 1.0 { continue; }  // uncomment to ignore small duration timeouts
                    let message = format!("{}, {}, {}\r\n", start, end, offline_duration);
                    file.write_all(message.as_bytes()).unwrap();
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
        file.sync_all().expect("file sync failed")
    }

    fn on_save_report_menu_item_selected(&self) {
        self.write_log();
        self.display_notification("Report saved");
    }

    fn display_notification(&self, message: &str) {
        let flags = nwg::TrayNotificationFlags::USER_ICON | nwg::TrayNotificationFlags::LARGE_ICON;
        self.tray
            .show("Status", Some(message), Some(flags), Some(&self.icon));
    }

    pub fn spawn_pinger(&self, address: &str, delay_millis: u32) -> thread::JoinHandle<()> {
        let sender = self.data.borrow().samples_sender.clone();
        let dst = String::from(address)
            .parse::<IpAddr>()
            .expect("Could not parse IP Address");
        let handle = thread::spawn(move || {
            let pinger = Pinger::new().unwrap();
            loop {
                let mut buffer = Buffer::new();
                let ping_response;
                let timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .expect("Bad time value")
                    .as_nanos();
                match pinger.send(dst, &mut buffer) {
                    Ok(rtt) => {
                        ping_response = Some(rtt as u16);
                    }
                    Err(_err) => {
                        ping_response = None;
                    }
                };
                if let Err(_e) = sender.send((dst, timestamp, ping_response)) {
                    break; // stop the loop if there is an error.
                }
                #[allow(deprecated)]
                thread::sleep_ms(delay_millis);
            }
        });
        handle
    }
}

fn main() -> std::io::Result<()> {
    nwg::init().expect("Failed to init Native Windows GUI");
    nwg::Font::set_global_family("Segoe UI").expect("Failed to set default font");
    let app = BasicApp::build_ui(Default::default()).expect("Failed to build UI");
    app.spawn_pinger("1.1.1.2", 600);
    app.spawn_pinger("8.8.8.8", 600);
    app.spawn_pinger("208.67.222.222", 600);
    nwg::dispatch_thread_events();
    Ok(())
}
