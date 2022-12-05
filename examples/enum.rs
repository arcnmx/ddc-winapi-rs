use {ddc::Ddc, ddc_winapi::Monitor};

fn main() {
    let monitors = Monitor::enumerate().unwrap();
    for mut m in monitors {
        print!("{:?}: ", m);
        println!("{:?}", m.get_timing_report());
    }
}
