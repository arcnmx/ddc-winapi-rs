use ddc_winapi::{DeviceInfoSet, Error};

fn main() -> Result<(), Error> {
    for device_info in DeviceInfoSet::monitors()?.enumerate() {
        let device_info = match device_info {
            Ok(info) => info,
            Err(e) => {
                println!("{e:?}");
                continue
            },
        };
        println!("{device_info:?}");
        for (key, value) in device_info.all_properties()? {
            println!("\t{key:?} = {value}");
        }
    }

    Ok(())
}
