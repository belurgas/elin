#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if elin_lib::cli::is_cli(&args) {
        elin_lib::cli::attach_console();
        let code = elin_lib::cli::run(&args);
        std::process::exit(code);
    }
    elin_lib::run();
}
