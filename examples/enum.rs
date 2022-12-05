use {
    ddc::Ddc,
    ddc_winapi::{Error, Output},
};

fn main() -> Result<(), Error> {
    for output in Output::enumerate()? {
        println!("{output:#?}");
        match output.info() {
            Ok(info) => println!("\t{info:?}"),
            Err(e) => println!("\t{e:?}"),
        }
        let monitors = match output.enumerate_monitors() {
            Ok(m) => m,
            Err(e) => {
                println!("\t{e:?}");
                continue
            },
        };
        for mut m in monitors {
            println!("\t{:?}", m.get_timing_report());
        }
    }

    Ok(())
}
