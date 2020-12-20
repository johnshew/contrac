#![windows_subsystem = "windows"]

use anyhow::{Context, Result};
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

mod graph;
mod stats;
mod utils;

use crate::graph::*;

const GRAPH_REFRESH_MILLIS: i64 = 250;
const MIN_TIMEOUT_INTERVAL_MILLIS: i64 = 1000;
const AUTO_SAVE_MINS: i64 = 5;

pub type Sample = (IpAddr, u128, Option<u16>);

pub struct AppData {
    count: u32,
    total: u32,
    min: u16,
    max: u16,
    samples: VecDeque<Sample>,
    last_full_update: DateTime<Local>,
    last_sample_display_timeout_notification: bool,
    timeout_start: Option<DateTime<Local>>,
    samples_receiver: Receiver<Sample>,
    samples_sender: Sender<Sample>,
    _app_start: DateTime<Local>,
    log_identifier: String,
    last_saved: DateTime<Local>,
}

impl Default for AppData {
    fn default() -> Self {
        let (s, r) = channel::<Sample>();
        let now = Local::now();
        AppData {
            count: 0,
            total: 0,
            min: u16::MAX,
            max: 0,
            samples: VecDeque::new(),
            last_full_update: Local::now(),
            last_sample_display_timeout_notification: false,
            timeout_start: None,
            samples_receiver: r,
            samples_sender: s,
            _app_start: now,
            log_identifier: format!("{}", now.format("%Y-%m-%d %H-%M-%S-%3f %z")),
            last_saved: Local::now(),
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

const _PAD_5: Rect<D> = Rect {
    start: D::Points(10.0),
    end: D::Points(10.0),
    top: D::Points(10.0),
    bottom: D::Points(10.0),
};
const PAD_2: Rect<D> = Rect {
    start: D::Points(2.0),
    end: D::Points(2.0),
    top: D::Points(2.0),
    bottom: D::Points(2.0),
};
const PAD_SHRINK_1: Rect<D> = Rect {
    start: D::Points(-1.0),
    end: D::Points(-1.0),
    top: D::Points(-1.0),
    bottom: D::Points(-1.0),
};

#[derive(Default, NwgUi)]
pub struct App {
    data: RefCell<AppData>,

    #[nwg_control(size: (640, 480), title: "Connection Tracker", flags: "MAIN_WINDOW|VISIBLE")]
    #[nwg_events( OnWindowClose: [App::on_window_close], OnInit: [App::on_window_init], OnWindowMinimize: [App::on_window_minimize] )]
    window: nwg::Window,

    #[nwg_control(interval: 1000, stopped: false)]
    #[nwg_events( OnTimerTick: [App::on_timer_tick] )]
    timer: nwg::Timer,

    #[nwg_resource]
    embed: nwg::EmbedResource,

    #[nwg_resource(source_embed: Some(&data.embed), source_embed_str: Some("MAINICON"))]
    icon: nwg::Icon,

    #[nwg_control(icon: Some(&data.icon), tip: Some("Connection Tracker"))]
    #[nwg_events(MousePressLeftUp: [App::on_tray_mouse_press_left_up], OnContextMenu: [App::on_tray_show_menu])]
    tray: nwg::TrayNotification,

    #[nwg_control(parent: window, popup: true)]
    tray_menu: nwg::Menu,

    #[nwg_control(parent: tray_menu, text: "Save Samples")]
    #[nwg_events(OnMenuItemSelected: [App::on_save_report_menu_item_selected])]
    tray_item1: nwg::MenuItem,

    // Main UX
    #[nwg_layout(parent: window, auto_spacing: None, flex_direction: FlexDirection::Column, justify_content: JustifyContent::Center)]
    main_layout: nwg::FlexboxLayout,

    #[nwg_control(text: "Latency", flags:"VISIBLE")]
    #[nwg_layout_item(layout: main_layout, margin: PAD_2, min_size: Size { width: D::Percent(0.96), height: D::Points(25.0) }, max_size: Size { width: D::Percent(1.0), height: D::Points(25.0) },)]
    graph_label: nwg::Label,

    #[nwg_control(flags: "VISIBLE")]
    #[nwg_layout_item(layout: main_layout, margin: PAD_SHRINK_1, min_size: Size { width: D::Percent(1.0), height: D::Points(100.0) }, size: Size { width: D::Percent(1.0), height: D::Points(1000.0)})]
    graph_frame: nwg::Frame,

    #[nwg_partial(parent: graph_frame)]
    graph: GraphUi,

    #[nwg_control(text: "", flags:"NONE")]
    #[nwg_layout_item(layout: main_layout, margin: PAD_2, min_size: Size { width: D::Percent(0.96), height: D::Points(25.0) }, max_size: Size { width: D::Percent(1.0), height: D::Points(25.0) },)]
    log_spacer: nwg::Label,

    #[nwg_control(text: "Log", flags:"VISIBLE")]
    #[nwg_layout_item(layout: main_layout, margin: PAD_2, min_size: Size { width: D::Percent(0.96), height: D::Points(25.0) }, max_size: Size { width: D::Percent(1.0), height: D::Points(25.0) },)]
    log_label: nwg::Label,

    #[nwg_control(text: "", flags:"VISIBLE|VSCROLL")]
    #[nwg_layout_item(layout: main_layout, margin: PAD_SHRINK_1,  min_size: Size { width: D::Percent(0.96), height: D::Points(100.0) }, max_size: Size { width: D::Percent(1.0), height: D::Points(100.0) },)]
    log: nwg::TextBox,

    #[nwg_control(flags: "VISIBLE")]
    #[nwg_layout_item(layout: main_layout, min_size: Size { width: D::Percent(1.0), height: D::Points(60.0) }, max_size: Size { width: D::Percent(1.0), height: D::Points(60.0)})]
    status_frame: nwg::Frame,

    #[nwg_layout(parent: status_frame, auto_spacing: None, flex_direction: FlexDirection::Row, align_items: AlignItems::Center, justify_content: JustifyContent::FlexEnd)]
    status_layout: nwg::FlexboxLayout,

    #[nwg_control(parent: status_frame, text: "Reset Stats")]
    #[nwg_layout_item(layout: status_layout,  margin: PAD_2, min_size: Size { width: D::Points(150.0), height: D::Points(40.0) },)]
    #[nwg_events( OnButtonClick: [App::on_reset_click] )]
    reset_button: nwg::Button,

    #[nwg_control(parent: status_frame, focus: true, text: "Close")]
    #[nwg_layout_item(layout: status_layout, margin: PAD_2, size: Size { width: D::Points(150.0), height: D::Points(40.0) },)]
    #[nwg_events( OnButtonClick: [App::on_window_close] )]
    close_button: nwg::Button,

    #[nwg_control(text: "", flags:"NONE")]
    #[nwg_layout_item(layout: main_layout, min_size: Size { width: D::Percent(0.96), height: D::Points(25.0) }, max_size: Size { width: D::Percent(1.0), height: D::Points(25.0) },)]
    statusbar_spacer: nwg::Label,

    #[nwg_control(parent: window)]
    message: nwg::StatusBar,

    // Tracbar for displaying - not currently used.
    #[nwg_control( flags:"HORIZONTAL|RANGE")] // not visible
    // #[nwg_layout_item(layout: main_layout, min_size: Size { width: D::Percent(1.0), height: D::Points(40.0)}, max_size: Size { width: D::Percent(1.0), height: D::Points(40.0)})]
    slider: nwg::TrackBar,
}

impl App {
    fn on_window_init(&self) {
        self.slider.set_range_min(0);
        self.slider.set_range_max(100);
        self.graph.init(40, 0, 50);
        self.graph.on_resize();
        let message = &format!(
            "Started at {}",
            self.data.borrow()._app_start.format("%F at %r")
        );
        self.log.set_text(message);
    }

    fn app_log_write(&self, message: &str) {
        let mut text = self.log.text();
        text.push_str(&format!(
            "\r\n{} {}",
            Local::now().format("%F at %r: "),
            message
        ));
        self.log.set_text(&text);
    }

    fn on_reset_click(&self) {
        let mut data = self.data.borrow_mut();
        data.count = 0;
        data.total = 0;
        data.min = u16::MAX;
        data.max = 0;
    }

    fn on_window_close(&self) {
        self.write_timeouts_log().unwrap();
        nwg::stop_thread_dispatch();
    }

    fn on_window_minimize(&self) {
        self.window.set_visible(false);
    }

    fn process_sample(&self, sample: Sample) {
        {
            let mut data = self.data.borrow_mut();
            data.record_observation(sample);
            let (_dst, timestamp, ping_response) = sample;
            if let Some(rtt) = ping_response {
                if data.last_sample_display_timeout_notification {
                    let mut text = self.log.text();
                    text.push_str(&format!(
                        "\r\nDisconnected at {} for {} seconds",
                        data.timeout_start.unwrap().format("%r"),
                        (utils::timestamp_to_datetime(timestamp as u128)
                            - data.timeout_start.unwrap())
                        .num_seconds()
                    ));
                    self.log.set_text(&text);
                }
                data.last_sample_display_timeout_notification = false;
                data.timeout_start = None;
                let message = format!(
                    "{} ms ({}:{}) {:.1}",
                    rtt,
                    data.min,
                    data.max,
                    data.average(),
                    // dst,
                );
                self.message.set_text(0, &message);
                self.slider.set_pos(rtt as usize);
                self.slider
                    .set_selection_range_pos(data.min as usize..data.max as usize);
            } else {
                self.message.set_text(0, "Disconnected");
                let datetime = utils::timestamp_to_datetime(timestamp as u128);
                if data.last_sample_display_timeout_notification == false
                    && data.timeout_start.is_some()
                    && datetime
                        > (data.timeout_start.unwrap()
                            + Duration::milliseconds(MIN_TIMEOUT_INTERVAL_MILLIS))
                {
                    self.display_notification("Disconnected");
                    data.last_sample_display_timeout_notification = true;
                }
                if let None = data.timeout_start {
                    data.timeout_start = Some(datetime);
                }
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
        {
            let mut data = self.data.borrow_mut();
            if datetime > (data.last_full_update + Duration::milliseconds(GRAPH_REFRESH_MILLIS)) {
                data.sort();
                self.graph.set_values(&data.samples);
                self.graph.on_resize();
                data.last_full_update = datetime;
            }
        }

        let auto_save =
            { self.data.borrow().last_saved + Duration::minutes(AUTO_SAVE_MINS) > datetime };
        if auto_save {
            if let Err(err) = self.write_timeouts_log() {
                self.app_log_write(&format!("{}", err));
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

    fn write_samples_log(&self) {
        self.app_log_write("Saving samples");
        {
            let mut data = self.data.borrow_mut();
            data.sort();
        }

        let data = self.data.borrow();
        let mut file = File::create(format!("{} samples.log", &data.log_identifier))
            .expect("file create failed");

        for (address, time, rtt) in &data.samples {
            let date_time = utils::timestamp_to_datetime(*time);
            let result = if let Some(rtt) = rtt {
                rtt.to_string()
            } else {
                String::from("timeout")
            };
            let message = format!("{:0}, {}, {}\r\n", date_time, result, address);
            let message = message.as_bytes();
            file.write_all(message).unwrap();
        }
        file.sync_all().expect("file sync failed");
    }

    fn write_timeouts_log(&self) -> Result<()> {
        match || -> Result<()> {
            // use closure to capture errors and enable '?' syntax
            enum TimeoutTracker {
                Active { start: DateTime<Local> },
                Nominal,
            };
            let data = self.data.borrow();
            let mut file = File::create(format!("{} timeouts.log", &data.log_identifier))
                .context(format!("unable to open '{}'", &data.log_identifier))?;
            let mut timeout_status = TimeoutTracker::Nominal;
            for (_address, time, rtt) in &data.samples {
                if rtt.is_some() {
                    if let TimeoutTracker::Active { start } = timeout_status {
                        let end = utils::timestamp_to_datetime(*time);
                        let offline_duration = (end - start).num_milliseconds() as f32 / 1_000.0;
                        timeout_status = TimeoutTracker::Nominal;
                        // if offline_duration < 1.0 { continue; }  // uncomment to ignore small duration timeouts
                        let message = format!("{}, {}, {}\r\n", start, end, offline_duration);
                        file.write_all(message.as_bytes())
                            .context(format!("write failed"))?;
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
            Ok(())
        }() {
            Ok(_) => self.data.borrow_mut().last_saved = Local::now(),
            Err(err) => self.app_log_write(&format!("error saving timesout log {:#?}", err)),
        };
        Ok(())
    }

    fn on_save_report_menu_item_selected(&self) {
        self.write_samples_log();
        self.display_notification("Report saved");
    }

    fn display_notification(&self, message: &str) {
        let flags = nwg::TrayNotificationFlags::USER_ICON | nwg::TrayNotificationFlags::LARGE_ICON;
        self.tray
            .show("Status", Some(message), Some(flags), Some(&self.icon));
    }

    pub fn spawn_pinger(&self, address: &str, delay_millis: u32) -> Result<thread::JoinHandle<()>> {
        let sender = self.data.borrow().samples_sender.clone();
        let dst = String::from(address)
            .parse::<IpAddr>()
            .context("Could not parse IP Address")?;
        let handle = thread::spawn(move || {
            let pinger = Pinger::new().unwrap();
            let mut buffer = Buffer::new();
            loop {
                let ping_response;
                let timestamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos();
                match pinger.send(dst, &mut buffer) {
                    Ok(rtt) => {
                        ping_response = Some(rtt as u16);
                    }
                    Err(_err) => {
                        ping_response = None;
                    }
                };
                if let Err(_err) = sender.send((dst, timestamp, ping_response)) {
                    break; // stop the loop if there is an error.
                }
                #[allow(deprecated)]
                thread::sleep_ms(delay_millis);
            }
        });
        Ok(handle)
    }
}

fn main() -> Result<()> {
    nwg::init().context("Failed to init app")?;
    nwg::Font::set_global_family("Segoe UI").context("Failed to set default font")?;
    let app = App::build_ui(Default::default()).context("Failed to build UI")?;
    let _pingers = vec![
        app.spawn_pinger("1.1.1.2", 1010)?,        // CloudFlare
        app.spawn_pinger("8.8.8.8", 1010)?,        // Google
        app.spawn_pinger("208.67.222.222", 1010)?, // Cisco OpenDNS
        app.spawn_pinger("9.9.9.9", 1010)?,        // Quad9
    ];
    nwg::dispatch_thread_events();
    Ok(())
}
