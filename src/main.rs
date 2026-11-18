mod app;
mod settings;
mod keyboard_shortcuts;
mod windows;
mod layouts;
mod services;
mod picker;
fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("--portal") => {
            if let Err(error) = picker::portal::run() { eprintln!("Veyra portal: {error}"); std::process::exit(1); }
        }
        Some("--picker-json") => {
            match serde_json::from_reader(std::io::stdin().lock()) {
                Ok(request) => app::run(Some(request)),
                Err(error) => {eprintln!("Invalid picker request: {error}"); std::process::exit(2);}
            }
        }
        Some("--pick-file") => app::run(Some(picker::Request {
            title: "Choose a file".into(), multiple: args.iter().any(|a| a == "--multiple"),
            directory: args.iter().any(|a| a == "--directory"), ..Default::default()
        })),
        Some("--help" | "-h") => println!("Veyra\n  veyra                         Open file manager\n  veyra --pick-file [--multiple] [--directory]\n                                Pick paths; print a JSON response\n  veyra --portal                Run desktop portal backend"),
        None => app::run(None),
        Some(arg) => {eprintln!("Unknown argument: {arg}. See --help."); std::process::exit(2);}
    }
}

mod selection;

mod thumbnails;

mod components;

mod context_menu;

mod batch_rename;
