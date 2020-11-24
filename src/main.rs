use std::cell::RefCell;
use std::collections::VecDeque;
use std::net::IpAddr;
use std::time::{SystemTime, UNIX_EPOCH};
use winping::{Buffer, Pinger};

extern crate native_windows_derive as nwd;
extern crate native_windows_gui as nwg; // Optional. Only if the derive macro is used.

use nwd::{NwgPartial, NwgUi};
use nwg::NativeUi;

pub struct AppData {
    count: u32,
    total: u32,
    min: u32,
    max: u32,
    probes: VecDeque<(u128, u16)>,
}

impl Default for AppData {
    fn default() -> Self {
        AppData {
            count: 0,
            total: 0,
            min: u32::MAX,
            max: 0,
            probes: VecDeque::new(),
        }
    }
}

impl AppData {
    fn average(&self) -> f32 {
        return self.total as f32 / self.count as f32;
    }
    // fn intervals(&self,  start: u128, end: u128) {}
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

    #[nwg_control(icon: Some(&data.icon), tip: Some("Hello"))]
    #[nwg_events(MousePressLeftUp: [BasicApp::show_menu], OnContextMenu: [BasicApp::show_menu])]
    tray: nwg::TrayNotification,

    #[nwg_control(parent: window, popup: true)]
    tray_menu: nwg::Menu,

    #[nwg_control(parent: tray_menu, text: "Hello")]
    #[nwg_events(OnMenuItemSelected: [BasicApp::hello_menu_item])]
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
}

impl BasicApp {
    fn on_init(&self) {
        self.slider.set_range_min(0);
        self.slider.set_range_max(100);
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
        data.min = u32::MAX;
        data.max = 0;
        data.probes.clear();
    }

    fn say_goodbye(&self) {
        if false {
            nwg::modal_info_message(
                &self.window,
                "Goodbye",
                &format!("Goodbye {}", self.message.text()),
            );
        }
        nwg::stop_thread_dispatch();
    }

    fn timer_tick(&self) {
        let dst = std::env::args()
            .nth(1)
            .unwrap_or(String::from("1.1.1.1"))
            .parse::<IpAddr>()
            .expect("Could not parse IP Address");

        let pinger = Pinger::new().unwrap();
        let mut buffer = Buffer::new();

        match pinger.send(dst, &mut buffer) {
            Ok(rtt) => {
                {
                    let mut data = self.data.borrow_mut();

                    if rtt < data.min {
                        data.min = rtt;
                    }
                    if rtt > data.max {
                        data.max = rtt;
                    }
                    data.count += 1;
                    data.total += rtt;
                    data.probes.push_back((
                        SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .expect("Clock issue")
                            .as_millis(),
                        rtt as u16,
                    ));

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
                }

                {
                    let data = self.data.borrow();
                    let mut graph: Vec<(u16, (u16, u16))> = Vec::new();
                    let mut count = 0;
                    let mut min = u16::MAX;
                    let mut max = 0;
                    let mut total = 0;
                    let mut avg = 0;
                    for item in &data.probes {
                        let (_time, ping) = item;
                        count += 1;
                        if *ping < min {
                            min = *ping;
                        };
                        if *ping > max {
                            max = *ping;
                        };
                        total += ping;
                        avg = (total / count) as u16;
                        if count >= 5 {
                            let item = (avg, (min, max));
                            graph.push(item);
                            count = 0;
                            min = u16::MAX;
                            max = 0;
                            total = 0;
                            avg = 0;
                        }
                    }
                    if count > 0 {
                        let item = (avg, (min, max));
                        graph.push(item);
                    }
                    self.graph.set_values(0, 20, graph);
                }
            }
            Err(err) => println!("{}.", err),
        }
    }

    fn show_menu(&self) {
        let (x, y) = nwg::GlobalCursor::position();
        self.tray_menu.popup(x, y);
    }

    fn hello_menu_item(&self) {
        let flags = nwg::TrayNotificationFlags::USER_ICON | nwg::TrayNotificationFlags::LARGE_ICON;
        let data = self.data.borrow();
        let message = format!("{}ms ({},{})", data.total / data.count, data.min, data.max);
        self.tray
            .show("Status", Some(&message), Some(flags), Some(&self.icon));
    }
}

struct GraphData {
    min: u16,
    max: u16,
    bars: Vec<(u16, (u16, u16))>,
}

impl Default for GraphData {
    fn default() -> Self {
        GraphData {
            min: 0,
            max: u16::MAX,
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
    fn set_values(&self, min: u16, max: u16, bars: Vec<(u16, (u16, u16))>) {
        let len;
        {
            let mut data = self.data.borrow_mut();
            data.min = min;
            data.max = max;
            data.bars = bars.to_vec();

            len = data.bars.len();
        }
        let graph_bars_len;
        {
            graph_bars_len = self.bars.borrow().len();
        }
        if len > graph_bars_len {
            let mut graph_bars = self.bars.borrow_mut();
            for _i in graph_bars_len..len {
                let mut new_bar = Default::default();
                nwg::ImageFrame::builder()
                    .parent(&self.frame)
                    .background_color(Some([0, 255, 255]))
                    .build(&mut new_bar)
                    .expect("Failed to build button");
                graph_bars.push(new_bar);
                // self.tooltip.register(&new_bar, "");
            }
            assert_eq!(graph_bars.len(), bars.len());
        }
        self.on_resize();
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
            let bar = data.bars[i];
            let (pos, (mut low, mut high)) = bar;

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
            let bar_h = (h as f32 * bar_h_ratio) as u32;
            let top_gap_ratio = (data.max - high) as f32 / (data.max - data.min) as f32;
            let top_gap = (h as f32 * top_gap_ratio) as i32;
            {
                let bars = self.bars.borrow();
                let bar = bars.get(i).unwrap();
                bar.set_size(w / data_len as u32, bar_h);
                bar.set_position((w / data_len as u32 * i as u32) as i32 + l, top_gap + t);
                let _tip = format!("{} ({},{})", pos, low, high);
                // let handle = nwg::ControlHandle::from(bar);
                // self.tooltip.set_text(&handle, &tip);
            }
        }
    }
}

fn main() {
    nwg::init().expect("Failed to init Native Windows GUI");
    nwg::Font::set_global_family("Segoe UI").expect("Failed to set default font");

    let _app = BasicApp::build_ui(Default::default()).expect("Failed to build UI");
    nwg::dispatch_thread_events();
}
