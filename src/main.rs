use chrono::{Duration, DurationRound, Local};
use std::cell::RefCell;
use std::collections::VecDeque;
use std::fs::File;
use std::io::prelude::*;
use std::net::IpAddr;
use std::time::{SystemTime, UNIX_EPOCH};
use winping::{Buffer, Pinger};

extern crate native_windows_derive as nwd;
extern crate native_windows_gui as nwg; // Optional. Only if the derive macro is used.

use nwd::{NwgPartial, NwgUi};
use nwg::NativeUi;

mod stats;
mod utils;

pub struct AppData {
    count: u32,
    total: u32,
    min: u16,
    max: u16,
    probes: VecDeque<(u128, Option<u16>)>,
}

impl Default for AppData {
    fn default() -> Self {
        AppData {
            count: 0,
            total: 0,
            min: u16::MAX,
            max: 0,
            probes: VecDeque::new(),
        }
    }
}

impl AppData {
    fn average(&self) -> f32 {
        return self.total as f32 / self.count as f32;
    }

    fn add_observation(&mut self, timestamp_in_nano: u128, response_time_in_milli: Option<u16>) {
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

    #[nwg_control(size: (300, 135), position: (300, 300), title: "Basic example", flags: "MAIN_WINDOW|VISIBLE")]
    #[nwg_events( OnWindowClose: [BasicApp::say_goodbye], OnInit: [BasicApp::on_init] )]
    window: nwg::Window,

    #[nwg_control(interval: 1000, stopped: false)]
    #[nwg_events( OnTimerTick: [BasicApp::timer_tick] )]
    timer: nwg::Timer,

    #[nwg_resource(source_file: Some("./resources/cog.ico"))]
    icon: nwg::Icon,

    #[nwg_control(icon: Some(&data.icon), tip: Some("Connection Tracker"))]
    #[nwg_events(MousePressLeftUp: [BasicApp::show_menu], OnContextMenu: [BasicApp::show_menu])]
    tray: nwg::TrayNotification,

    #[nwg_control(parent: window, popup: true)]
    tray_menu: nwg::Menu,

    #[nwg_control(parent: tray_menu, text: "Save Report")]
    #[nwg_events(OnMenuItemSelected: [BasicApp::menu_item_save_report])]
    tray_item1: nwg::MenuItem,

    // Main UX
    #[nwg_layout(parent: window, spacing: 1)]
    grid: nwg::GridLayout,

    #[nwg_control( flags:"VISIBLE|HORIZONTAL|RANGE")]
    #[nwg_layout_item(layout: grid, col: 0,  row: 0)]
    slider: nwg::TrackBar,

    #[nwg_control(text: "", h_align: nwg::HTextAlign::Center)]
    #[nwg_layout_item(layout: grid, col: 0,  row: 1)]
    message: nwg::Label,

    #[nwg_control] // ( flags:"BORDER")]
    #[nwg_layout_item(layout: grid, col: 0, row: 2, row_span: 2)]
    graph_frame: nwg::Frame,

    #[nwg_partial(parent: graph_frame)]
    // #[nwg_events( (save_btn, OnButtonClick): [PartialDemo::save] )]
    graph: GraphUi,

    #[nwg_control(text: "Clear")]
    #[nwg_layout_item(layout: grid, col: 0,  row: 4)]
    #[nwg_events( OnButtonClick: [BasicApp::clear] )]
    clear_button: nwg::Button,
    // save_file_dialog: nwg::FileDialog,
}

impl BasicApp {
    fn on_init(&self) {
        self.slider.set_range_min(0);
        self.slider.set_range_max(100);
        self.graph.init(30, 0, 20);
        // nwg::FileDialog::builder()
        // .action(nwg::FileDialogAction::Save)
        // .title("Save a file")
        // .filters("Text(*.txt)|Any(*.*)")
        // .build(&mut &self.save_file_dialog);
    }

    fn _say_hello(&self) {
        nwg::modal_info_message(
            &self.window,
            "Hello",
            &format!("Hello {}", self.message.text()),
        );
    }

    fn clear(&self) {
        let mut data = self.data.borrow_mut();
        data.count = 0;
        data.total = 0;
        data.min = u16::MAX;
        data.max = 0;
        data.probes.clear();
    }

    fn say_goodbye(&self) {
        nwg::stop_thread_dispatch();
    }

    fn timer_tick(&self) {
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

        // Update data
        {
            let mut data = self.data.borrow_mut();
            data.add_observation(timestamp, ping_response);
        }

        //update UX
        {
            let data = self.data.borrow();
            if let Some(rtt) = ping_response {
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
                self.message.set_text("Timeout");
                self.slider.set_pos(300);
            }
            self.graph.set_values(&data.probes);
        }
    }

    fn show_menu(&self) {
        let (x, y) = nwg::GlobalCursor::position();
        self.tray_menu.popup(x, y);
    }

    fn write_log(&self) -> std::io::Result<()> {
        let mut f = File::create("report.txt")?;
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
        Ok(())
    }

    fn menu_item_save_report(&self) {
        self.write_log().unwrap();

        let data = self.data.borrow();
        let message = format!(
            "Report save. {:1} ms ({},{})",
            data.total as f32 / data.count as f32,
            data.min,
            data.max
        );
        self.notification(&message);
    }

    fn notification(&self, message: &str) {
        let flags = nwg::TrayNotificationFlags::USER_ICON | nwg::TrayNotificationFlags::LARGE_ICON;
        self.tray
            .show("Status", Some(message), Some(flags), Some(&self.icon));
    }
}

struct GraphData {
    bar_count: u16,
    min: u16,
    max: u16,
    bars: Vec<stats::Stats<u16>>,
}

impl Default for GraphData {
    fn default() -> Self {
        GraphData {
            bar_count: 0,
            min: u16::MAX,
            max: 0,
            bars: Vec::new(),
        }
    }
}

#[derive(Default, NwgPartial)]
pub struct GraphUi {
    data: RefCell<GraphData>,

    #[nwg_layout( spacing: 1)]
    grid: nwg::GridLayout,

    #[nwg_control(flags: "NONE")]
    #[nwg_layout_item(layout: grid, row: 0, col: 0 )]
    #[nwg_events( OnResize: [GraphUi::on_resize])]
    frame: nwg::Frame,

    // #[nwg_control]
    // tooltip: nwg::Tooltip,
    bars: RefCell<Vec<nwg::ImageFrame>>,
}

impl GraphUi {
    fn init(&self, graph_bars_len: u16, min: u16, max: u16) {
        let mut graph_bars = self.bars.borrow_mut();
        let len = graph_bars.len() as u16;
        if len < graph_bars_len {
            for _i in len..graph_bars_len {
                let mut new_bar = Default::default();
                nwg::ImageFrame::builder()
                    .parent(&self.frame)
                    .background_color(Some([0, 255, 255]))
                    .build(&mut new_bar)
                    .expect("Failed to build button");
                graph_bars.push(new_bar);
            }
        }
        let mut data = self.data.borrow_mut();
        data.bar_count = graph_bars_len;
        data.min = min;
        data.max = max;
        if data.bar_count != data.bars.len() as u16 {
            data.bars
                .resize_with(graph_bars_len as usize, Default::default);
            let len = data.bars.len();
            println!("{}", len);
        }
    }

    fn set_values(&self, probes: &VecDeque<(u128, Option<u16>)>) {
        // Loop backward in time in 10 second intervals aligned to clock.
        // so find now to the nearest forward 10 second aligned time in nanoseconds and then iterate backward in time.alloc
        // if there is a timeout in that interval then that bar should be red.
        // if there is no data in that interval it can be invisible

        let now = Local::now();
        println!("Current time is {}", now);
        let interval = Duration::seconds(10);
        let mut end_of_interval = (now + interval)
            .duration_trunc(interval)
            .expect("time trucation should always work");
        let mut start_of_interval = end_of_interval - interval;

        let mut probe_count_remaining = probes.len();

        let bars = self.bars.borrow();
        let mut bar_is_complete = false;

        for i in (0..bars.len()).rev() {
            println!(
                "Bar {} matching {} to {}",
                i, start_of_interval, end_of_interval
            );
            let mut stats = <stats::Stats<u16> as Default>::default();
            while !bar_is_complete {
                if probe_count_remaining > 0 {
                    // let Some(thing) = probes[probe_count_remaining] {
                    let (timestamp, _ping) = probes[probe_count_remaining - 1]; //**thing;
                    let datetime = utils::timestamp_to_datetime(timestamp);
                    assert!(datetime < end_of_interval, "confirming time");
                    println!(
                        "Found {} with {} samples remaining",
                        datetime, probe_count_remaining
                    );
                    if datetime < start_of_interval {
                        bar_is_complete = true;
                        println!("It is out of the interval");
                    }
                } else {
                    bar_is_complete = true;
                    println!("No more data");
                }
                if bar_is_complete {
                    let mut data = self.data.borrow_mut();
                    data.bars[i] = stats; 
                    bar_is_complete = false;
                    break;
                }
                let (_timestamp, ping) = probes[probe_count_remaining - 1]; 
                probe_count_remaining -= 1;
                println!("Updating stats for bar {}", i);
                stats.update(ping);
            }
            end_of_interval = start_of_interval;
            start_of_interval = end_of_interval - interval;
        }
    }

    fn on_resize(&self) {
        self.frame.set_visible(true);

        let (w, h) = self.frame.size();
        let (l, t) = self.frame.position();

        let data_len;
        {
            data_len = self.data.borrow().bars.len();
        }
        for i in 0..data_len {
            let data = self.data.borrow();
            let bar = &data.bars[i];
            let graph_bars = self.bars.borrow();
            let graph_bar = &graph_bars[i];
            if bar.count > 0 {
                let _pos = bar.average().unwrap();
                let mut low = bar.min;
                let mut high = bar.max;

                // Clip
                if low < data.min {
                    low = data.min;
                }
                if low > data.max {
                    low = data.max;
                }
                if high < data.min {
                    high = data.min;
                }
                if high > data.max {
                    high = data.max;
                }

                let bar_h_ratio = (high - low) as f32 / (data.max - data.min) as f32;
                let mut bar_h = (h as f32 * bar_h_ratio) as u32;
                if bar_h < 2 {
                    bar_h = 2;
                }
                let top_gap_ratio = (data.max - high) as f32 / (data.max - data.min) as f32;
                let top_gap = (h as f32 * top_gap_ratio) as i32;
                {
                    graph_bar.set_size(w / data_len as u32, bar_h);
                    graph_bar
                        .set_position((w / data_len as u32 * i as u32) as i32 + l, top_gap + t);
                    if bar.timeout {
                        // graph_bar set background to red
                    }
                    graph_bar.set_visible(true);
                    // let _tip = format!("{} ({},{})", pos, low, high);
                    // let handle = nwg::ControlHandle::from(bar);
                    // self.tooltip.set_text(&handle, &tip);
                }
            } else {
                graph_bar.set_visible(false);
            }
        }
    }
}

fn main() -> std::io::Result<()> {
    let mut f = File::create("report.txt")?;
    f.write_all(b"Hello, world!")?;
    f.sync_all()?;

    nwg::init().expect("Failed to init Native Windows GUI");
    nwg::Font::set_global_family("Segoe UI").expect("Failed to set default font");

    let _app = BasicApp::build_ui(Default::default()).expect("Failed to build UI");
    nwg::dispatch_thread_events();
    Ok(())
}

// while let (timestamp, ping) = data.next_back() {
//     if let Some(ping) = ping {
//         count+=1;
//         total+=ping;
//         if ping < min { min = ping};
//         if ping > max { max = ping };

//     } else {
//         // bar to red
//     }
// } else {
//     done = true;
// }

//     // {
//     //     data.min = min;
//     //     data.max = max;
//     //     let max_count = 20;
//     //     let _len = bars.len();
//     //     if bars.len() > max_count {
//     //         data.bars = bars[bars.len() - max_count..bars.len()].to_vec();
//     //         let _len = data.bars.len();
//     //     } else {
//     //         data.bars = bars.to_vec();
//     //     }
//     //     len = data.bars.len();
//     // }
//     self.on_resize();
// }
