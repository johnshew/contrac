use std::net::IpAddr;
use winping::{Buffer, Pinger};

extern crate native_windows_derive as nwd;
extern crate native_windows_gui as nwg; // Optional. Only if the derive macro is used.

use nwd::NwgUi;
use nwg::NativeUi;

#[derive(Default, NwgUi)]
pub struct BasicApp {
    #[nwg_control(size: (300, 135), position: (300, 300), title: "Basic example", flags: "WINDOW|VISIBLE")]
    #[nwg_events( OnWindowClose: [BasicApp::say_goodbye], OnInit: [BasicApp::setup] )]
    window: nwg::Window,

    #[nwg_control(text: "Heisenberg", size: (280, 35), position: (10, 10), focus: true)]
    name_edit: nwg::TextInput,

    #[nwg_control(text: "Say my name", size: (280, 70), position: (10, 50))]
    #[nwg_events( OnButtonClick: [BasicApp::say_hello] )]
    hello_button: nwg::Button,

    #[nwg_control(interval: 1000, stopped: false)]
    #[nwg_events( OnTimerTick: [BasicApp::timer_tick] )]
    timer: nwg::Timer,
}

impl BasicApp {
    fn say_hello(&self) {
        nwg::modal_info_message(
            &self.window,
            "Hello",
            &format!("Hello {}", self.name_edit.text()),
        );
    }
    fn say_goodbye(&self) {
        nwg::modal_info_message(
            &self.window,
            "Goodbye",
            &format!("Goodbye {}", self.name_edit.text()),
        );
        nwg::stop_thread_dispatch();
    }

    fn setup(&self) {
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
                let result = format!("Response time: {}", rtt);
                self.name_edit.set_text(&result);
            }
            Err(err) => println!("{}.", err),
        }    
    }
}

fn main() {
    nwg::init().expect("Failed to init Native Windows GUI");
    nwg::Font::set_global_family("Segoe UI").expect("Failed to set default font");

    let _app = BasicApp::build_ui(Default::default()).expect("Failed to build UI");

    nwg::dispatch_thread_events();
}

/*
fn main() {
    let ip_arg = std::env::args().nth(1);
    let ip_text = ip_arg.unwrap_or(String::from("8.8.8.8"));
    let ip_addr = ip_text.parse::<IpAddr>().expect("Could not parse IP Address");

    let dst = std::env::args()
        .nth(1)
        .unwrap_or(String::from("1.1.1.1"))
        .parse::<IpAddr>()
        .expect("Could not parse IP Address");

    println!("{}",dst);

    let pinger = Pinger::new().unwrap();
    let mut buffer = Buffer::new();

    for _ in 0..4 {
        match pinger.send(dst, &mut buffer) {
            Ok(rtt) => {
                println!("Response time {} ms.", rtt);
            }
            Err(err) => println!("{}.", err),
        }
    }
    println!("Hello, world!");
}
 */
