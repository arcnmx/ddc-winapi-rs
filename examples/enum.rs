extern crate ddc;
use ddc::Ddc;

extern crate ddc_winapi;
use ddc_winapi::Monitor;

fn main() {
    let monitors = Monitor::enumerate().unwrap();
    for mut m in monitors {
        print!("{:?}: ", m);
        println!("{:?}", m.get_timing_report());
    }
}
