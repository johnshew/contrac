use chrono::{Duration, DurationRound, Local};
use std::cell::RefCell;
use std::collections::VecDeque;

extern crate native_windows_derive as nwd;
extern crate native_windows_gui as nwg;

use nwd::NwgPartial;

use super::stats;
use super::utils;
use super::callbacks::Callbacks;
use crate::Sample;


const GRAPH_INTERVAL_MILLIS: i64 = 1000;

pub struct GraphData {
    bar_count: u16,
    min: u16,
    max: u16,
    bars: Vec<stats::Stats<u16>>,
    events: Callbacks<'static, (), ()>,
}

impl<'a> Default for GraphData {
    fn default() -> Self {
        GraphData {
            bar_count: 0,
            min: u16::MAX,
            max: 0,
            bars: Vec::new(),
            events: Callbacks::new(),
        }
    }
}

#[derive(Default, NwgPartial)]
pub struct GraphUi {
    data: RefCell<GraphData>,

    #[nwg_layout( margin: [0,0,0,0], spacing: 0)]
    grid: nwg::GridLayout,

    #[nwg_control(flags: "VISIBLE")]
    #[nwg_layout_item(layout: grid, row: 0, col: 0 )]
    #[nwg_events( OnResize: [GraphUi::on_resize])]
    outer_frame: nwg::Frame,

    #[nwg_control(parent: outer_frame, flags: "VISIBLE")]
    frame: nwg::Frame,

    #[nwg_control(parent: outer_frame, size: (50,25), text: "30", limit:3,  flags: "NUMBER")]
    #[nwg_events( OnTextInput: [GraphUi::on_min_max_click])]
    pub max_select: nwg::TextInput,

    #[nwg_control(parent: outer_frame, size: (50,25), text: "0", limit:3, flags: "NUMBER")]
    #[nwg_events( OnTextInput: [GraphUi::on_min_max_click])]
    pub min_select: nwg::TextInput,

    bars: RefCell<Vec<nwg::ImageFrame>>,
    // tooltips: nwg::Tooltip,
}

impl GraphUi {
    pub fn init(&self, graph_bars_len: u16, min: u16, max: u16) {
        {
            let mut graph_bars = self.bars.borrow_mut();
            let len = graph_bars.len() as u16;
            if len < graph_bars_len {
                for _i in len..graph_bars_len {
                    let mut new_bar = Default::default();
                    nwg::ImageFrame::builder()
                        .parent(&self.frame)
                        .background_color(Some([127, 127, 127]))
                        .build(&mut new_bar)
                        .expect("Failed to build button");
                    // let handle = new_bar.handle;
                    // self.tooltips.register(handle, "");
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
            }
        }

        self.min_select.set_text(&format!("{}", min));
        self.max_select.set_text(&format!("{}", max));
        self.min_select.set_visible(true);
        self.max_select.set_visible(true);
    }

    pub fn on_min_max_click(&self) {
        let mut max = if let Ok(v) = self.max_select.text().parse::<u16>() {
            v
        } else {
            300
        };
        let mut min = if let Ok(v) = self.min_select.text().parse::<u16>() {
            v
        } else {
            0
        };
        if max < min {
            max = min + 10;
        }
        if min > max {
            min = max - 10;
        }
        {
            let mut data = self.data.borrow_mut();
            data.min = min;
            data.max = max;
        }
    }

    pub fn get_min_max(&self) -> (u16, u16) {
        let data = self.data.borrow();
        (data.min, data.max)
    }

    pub fn set_values(&self, samples: &VecDeque<Sample>) {
        // Loop backward in time in 1 second intervals aligned to clock.
        // so find now to the nearest forward 1 second aligned time in nanoseconds and then iterate backward in time.
        // if there is no data in that interval it can be invisible

        let now = Local::now();
        let interval = Duration::milliseconds(GRAPH_INTERVAL_MILLIS);
        let mut end_of_interval = (now + interval)
            .duration_trunc(interval)
            .expect("time trucation should always work");
        let mut start_of_interval = end_of_interval - interval;

        let mut probe_count_remaining = samples.len();

        let bars = self.bars.borrow();
        let mut bar_is_complete = false;

        for i in (0..bars.len()).rev() {
            let mut stats = <stats::Stats<u16> as Default>::default();
            while !bar_is_complete {
                if probe_count_remaining > 0 {
                    // let Some(thing) = probes[probe_count_remaining] {
                    let (_address, timestamp, _ping) = samples[probe_count_remaining - 1]; //**thing;
                    let datetime = utils::timestamp_to_datetime(timestamp);
                    assert!(datetime < end_of_interval, "confirming time");
                    if datetime < start_of_interval {
                        bar_is_complete = true;
                    }
                } else {
                    bar_is_complete = true;
                }
                if bar_is_complete {
                    let mut data = self.data.borrow_mut();
                    data.bars[i] = stats;
                    bar_is_complete = false;
                    break;
                }
                let (_address, _timestamp, ping) = samples[probe_count_remaining - 1];
                probe_count_remaining -= 1;
                stats.update(ping);
            }
            end_of_interval = start_of_interval;
            start_of_interval = end_of_interval - interval;
        }
    }

    pub fn on_resize(&self) {
        
        let (ow, oh) = self.outer_frame.size();

        let (_max_w, max_h) = self.max_select.size();
        let (_min_w, min_h) = self.min_select.size();
        self.frame.set_size(ow + 1, oh - max_h - min_h);
        self.frame.set_visible(true);
        self.frame.set_position(0, max_h as i32);

        let (w, h) = self.frame.size();
        self.max_select.set_position(0, 0);
        self.max_select.set_size(ow + 1, max_h);
        self.min_select.set_position(0, (oh - min_h) as i32);
        self.min_select.set_size(ow + 1, min_h);

        let data_len = { self.data.borrow().bars.len() };

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
                    graph_bar.set_size(1 + (w / data_len as u32), bar_h);
                    graph_bar.set_position(w as i32 * (i as i32) / data_len as i32, top_gap);
                    if bar.timeout {
                        // graph_bar set background to red
                    }
                    graph_bar.set_visible(true);
                    // let tip = format!("{} ({},{})", _pos, low, high);
                    // self.tooltips.set_text(graph_bar, &tip);
                }
            } else {
                graph_bar.set_visible(false);
            }
        }     
    }
}
