use std::cell::RefCell;
use std::collections::VecDeque;
use std::net::IpAddr;
use std::time::{SystemTime, UNIX_EPOCH};
use winping::{Buffer, Pinger};

extern crate native_windows_derive as nwd;
extern crate native_windows_gui as nwg; // Optional. Only if the derive macro is used.

use nwd::NwgUi;
use nwg::NativeUi;

#[derive(Default, NwgUi)]
pub struct BasicApp {
    data: RefCell<MyData>,

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

    #[nwg_control(text: "Heisenberg", focus: true)]
    #[nwg_layout_item(layout: grid, row: 0, col: 0)]
    message_edit: nwg::TextInput,

    #[nwg_control( flags:"VISIBLE|HORIZONTAL|RANGE")]
    #[nwg_layout_item(layout: grid, col: 0, row: 1)]
    slider: nwg::TrackBar,

    #[nwg_control(text: "Clear")]
    #[nwg_layout_item(layout: grid, col: 0, row: 2)]
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
            &format!("Hello {}", self.message_edit.text()),
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
                &format!("Goodbye {}", self.message_edit.text()),
            );
        }
        nwg::stop_thread_dispatch();
    }

    fn timer_tick(&self) {
        let mut data = self.data.borrow_mut();

        let dst = std::env::args()
            .nth(1)
            .unwrap_or(String::from("1.1.1.1"))
            .parse::<IpAddr>()
            .expect("Could not parse IP Address");

        let pinger = Pinger::new().unwrap();
        let mut buffer = Buffer::new();

        match pinger.send(dst, &mut buffer) {
            Ok(rtt) => {
                let result = format!("Response time: {}", rtt);
                self.message_edit.set_text(&result);
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
                    rtt,
                ));
                self.slider.set_pos(rtt as usize);
                self.slider
                    .set_selection_range_pos(data.min as usize..data.max as usize);
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

pub struct MyData {
    count: u32,
    total: u32,
    min: u32,
    max: u32,
    probes: VecDeque<(u128, u32)>,
}

impl Default for MyData {
    fn default() -> Self {
        MyData {
            count: 0,
            total: 0,
            min: u32::MAX,
            max: 0,
            probes: VecDeque::new(),
        }
    }
}

fn main() {
    nwg::init().expect("Failed to init Native Windows GUI");
    nwg::Font::set_global_family("Segoe UI").expect("Failed to set default font");

    let _app = BasicApp::build_ui(Default::default()).expect("Failed to build UI");
    nwg::dispatch_thread_events();
}
