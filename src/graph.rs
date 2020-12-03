use std::cell::RefCell;
use std::collections::VecDeque;
use chrono::{Local, Duration, DurationRound};

extern crate native_windows_derive as nwd;
extern crate native_windows_gui as nwg; // Optional. Only if the derive macro is used.

use nwd::{NwgPartial};
// use nwg::NativeUi;

use super::stats;
use super::utils;
use crate::{Sample};

pub struct GraphData {
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
    pub fn init(&self, graph_bars_len: u16, min: u16, max: u16) {
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
        }
    }

    pub fn set_values(&self, samples: &VecDeque<Sample>) {
        // Loop backward in time in 10 second intervals aligned to clock.
        // so find now to the nearest forward 10 second aligned time in nanoseconds and then iterate backward in time.alloc
        // if there is a timeout in that interval then that bar should be red.
        // if there is no data in that interval it can be invisible

        let now = Local::now();
        let interval = Duration::seconds(10);
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
