#![windows_subsystem = "windows"]

use anyhow::{bail, Context, Result};
use chrono::{DateTime, Duration, Local};
use directories::UserDirs;
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
use winreg::enums::*;
use winreg::RegKey;

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
use crate::utils::GetHostName;

const MIN_PING_TIME_MILLIS: u32 = 1010;
const GRAPH_REFRESH_MILLIS: i64 = 250;
const MIN_TIMEOUT_INTERVAL_MILLIS: i64 = 1000;
const AUTO_SAVE_MINS: i64 = 5;
const GRAPH_BAR_COUNT: u16 = 40;

pub type Sample = (IpAddr, u128, Option<u16>);

pub struct AppData {
    count: u32,
    total: u32,
    min: u16,
    max: u16,
    samples: VecDeque<Sample>,
    registry_loaded: bool,
    graph_min: u16,
    graph_max: u16,
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
        let hostname = GetHostName().into_string().expect("Not a string");
        AppData {
            count: 0,
            total: 0,
            min: u16::MAX,
            max: 0,
            samples: VecDeque::new(),
            registry_loaded: false,
            graph_min: 0,
            graph_max: 100,
            last_full_update: Local::now(),
            last_sample_display_timeout_notification: false,
            timeout_start: None,
            samples_receiver: r,
            samples_sender: s,
            _app_start: now,
            log_identifier: format!("{} {}", hostname, now.format("%Y-%m-%d %H-%M-%S-%3f %z")),
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

const PAD_NONE: Rect<D> = Rect {
    start: D::Points(0.0),
    end: D::Points(0.0),
    top: D::Points(0.0),
    bottom: D::Points(0.0),
};

const PAD_SHRINK_LEFT: Rect<D> = Rect {
    start: D::Points(-1.0),
    end: D::Points(0.0),
    top: D::Points(0.0),
    bottom: D::Points(0.0),
};


#[derive(Default, NwgUi)]
pub struct App {
    data: RefCell<AppData>,

    #[nwg_control(size: (640, 480), title: "Connection Tracker", flags: "MAIN_WINDOW|VISIBLE")]
    #[nwg_events( OnWindowClose: [App::on_window_close], OnInit: [App::on_window_init], OnWindowMinimize: [App::on_window_minimize] )]
    window: nwg::Window,

    #[nwg_control(interval: std::time::Duration::from_millis(300), active: true)]
    #[nwg_events( OnTimerTick: [App::on_timer_tick] )]
    timer: nwg::AnimationTimer,

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
    #[nwg_events( (max_select, OnTextInput): [App::on_graph_min_max_change],
                  (min_select, OnTextInput): [App::on_graph_min_max_change] )]
    graph: GraphUi,

    #[nwg_control(text: "", flags:"NONE")]
    #[nwg_layout_item(layout: main_layout, margin: PAD_2, min_size: Size { width: D::Percent(0.96), height: D::Points(25.0) }, max_size: Size { width: D::Percent(1.0), height: D::Points(25.0) },)]
    log_spacer: nwg::Label,

    #[nwg_control(text: "Log", flags:"VISIBLE")]
    #[nwg_layout_item(layout: main_layout, margin: PAD_2, min_size: Size { width: D::Percent(0.96), height: D::Points(25.0) }, max_size: Size { width: D::Percent(1.0), height: D::Points(25.0) },)]
    log_label: nwg::Label,

    #[nwg_control(text: "", flags:"VISIBLE|VSCROLL")]
    #[nwg_layout_item(layout: main_layout, margin: PAD_SHRINK_LEFT,  min_size: Size { width: D::Percent(0.96), height: D::Points(100.0) }, max_size: Size { width: D::Percent(1.0), height: D::Points(100.0) },)]
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
}

impl App {
    fn on_window_init(&self) {
        self.log.set_text("Starting");
        let result = self.load_registry_settings();
        if let Err(e) = result {
            self.app_log_write(&format!("Registry settings: {}", e));
        }
        self.data.borrow_mut().registry_loaded = true;
        let (min, max) = {
            let data = self.data.borrow();
            (data.graph_min, data.graph_max)
        };
        self.message.set_min_height(25); // not settable above
        self.graph.init(GRAPH_BAR_COUNT, min, max);
        self.graph.on_resize();
        self.app_log_write("Running");
    }

    fn load_registry_settings(&self) -> Result<()> {
        let reg = RegKey::predef(HKEY_CURRENT_USER);
        let subkey = reg.open_subkey("SOFTWARE\\Vivitap\\Contrac")?;
        let min: u32 = subkey.get_value("GraphMin")?;
        let max: u32 = subkey.get_value("GraphMax")?;
        let mut data = self.data.borrow_mut();
        data.graph_min = min as u16;
        data.graph_max = max as u16;
        Ok(())
    }

    fn save_registry_settings(&self) -> Result<()> {
        if !self.data.borrow().registry_loaded {
            bail!("not loaded yet");
        }
        let reg = RegKey::predef(HKEY_CURRENT_USER);
        let subkey = match reg.create_subkey("SOFTWARE\\Vivitap\\Contrac") {
            Ok((key, _disposition)) => key,
            Err(e) => bail!("read error {:?}",e),
        };
        {
            let data = self.data.borrow();
            if let Err(e) = subkey.set_value("GraphMin", &(data.graph_min as u32)) { bail!("write error {}", e)};
            if let Err(e) = subkey.set_value("GraphMax", &(data.graph_max as u32)) { bail! ("write error {}", e)};
        }
        Ok(())
    }

    fn on_graph_min_max_change(&self) {
        let (min, max) = self.graph.get_min_max();
        let mut changed = false;
        {
            let mut data = self.data.borrow_mut();
            if min != data.graph_min || max != data.graph_max {
                data.graph_min = min;
                data.graph_max = max;
                changed = true;
            }
        }
        if !changed { return; }
        if let Err(e) = self.save_registry_settings() { self.app_log_write(&format!("Problem saving to the registry: {:?}", e)) };
    }

    fn app_log_write(&self, message: &str) {
        let mut text = self.log.text();
        text.push_str(&format!(
            "\r\n{}: {}",
            Local::now().format("%F %r"),
            message
        ));
        self.log.set_text(&text);
        utils::VScrollToBottom(&self.log.handle);
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
                    self.app_log_write(&format!(
                        "was disconnected for {} seconds",
                        (utils::timestamp_to_datetime(timestamp as u128)
                            - data.timeout_start.unwrap())
                        .num_milliseconds() as f32
                            / 1_000 as f32
                    ));
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

        if (self.data.borrow().last_saved + Duration::minutes(AUTO_SAVE_MINS)) < datetime {
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
            }
            let data = self.data.borrow();
            let path = UserDirs::new()
                .unwrap()
                .document_dir()
                .unwrap()
                .join(format!("contrac {} timeouts.log", &data.log_identifier));
            let full_path = String::from(path.to_str().unwrap());
            // self.app_log_write(&format!("writing to {}", full_path));
            let mut file = File::create(&path) // was format!("{} timeouts.log", documents.join(path: P), &data.log_identifier))
                .context(format!("unable to open '{:?}'", &full_path))?;
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
        app.spawn_pinger("1.1.1.2", MIN_PING_TIME_MILLIS)?, // CloudFlare
        app.spawn_pinger("8.8.8.8", MIN_PING_TIME_MILLIS)?, // Google
        app.spawn_pinger("208.67.222.222", MIN_PING_TIME_MILLIS)?, // Cisco OpenDNS
        app.spawn_pinger("9.9.9.9", MIN_PING_TIME_MILLIS)?, // Quad9
    ];
    nwg::dispatch_thread_events();
    Ok(())
}
