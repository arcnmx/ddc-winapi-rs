use ddc_winapi::DisplayDevice;

fn main() {
    for display in DisplayDevice::enumerate() {
        println!("{display:#?}");
        for monitor in display.enumerate_monitors() {
            println!("Monitor {:#?}", *monitor);
        }
        println!();
    }
}
