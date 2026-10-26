#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if std::env::args().any(|arg| arg == "--cinderdock-smoke-test") {
        match cinderdock_lib::smoke_test() {
            Ok(report) => match serde_json::to_string_pretty(&report) {
                Ok(json) => println!("{json}"),
                Err(error) => {
                    eprintln!("CinderDock smoke test could not serialize report: {error}");
                    std::process::exit(1);
                }
            },
            Err(error) => {
                eprintln!("CinderDock smoke test failed: {error}");
                std::process::exit(1);
            }
        }
        return;
    }
    cinderdock_lib::run();
}
