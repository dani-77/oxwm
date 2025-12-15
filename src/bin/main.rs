use std::path::PathBuf;

static CONFIG_FILE: &str = "config.lua";
static TEMPLATE: &str = include_str!("../../templates/config.lua");

enum Args {
    Exit,
    Arguments(Vec<String>),
    Error(String),
}

fn main() {
    let arguments = match process_args() {
        Args::Exit => return,
        Args::Arguments(v) => v,
        Args::Error(e) => panic!("Could not get valid arguments:\n{}", e),
    };

    let config_path = match arguments.get(2) {
        Some(p) => PathBuf::from(p),
        None => {
            let config_directory = get_config_path();
            let config_path = config_directory.join(CONFIG_FILE);
            PathBuf::from(config_path)
        }
    };

    let (config, had_broken_config) = match load_config(config_path) {
        Ok((c, b)) => (c, b),
        Err(e) => panic!("Could not load config:\n{}", e),
    };

    let mut window_manager = match oxwm::window_manager::WindowManager::new(config) {
        Ok(wm) => wm,
        Err(e) => panic!("Could not start window manager:\n{}", e),
    };

    if had_broken_config {
        window_manager.show_migration_overlay();
    }

    let should_restart = match window_manager.run() {
        Ok(sr) => sr,
        Err(e) => panic!("{}", e),
    };

    drop(window_manager);

    if should_restart {
        use std::os::unix::process::CommandExt;
        let error = std::process::Command::new(&arguments[0])
            .args(&arguments[1..])
            .exec();
        eprintln!("Failed to restart: {}", error);
    }
}

fn load_config(config_path: PathBuf) -> Result<(oxwm::Config, bool), Box<dyn std::error::Error>> {
    check_convert(&config_path)
        .map_err(|error| format!("Failed to check old config:\n{}", error))?;
    let config_string = std::fs::read_to_string(&config_path)
        .map_err(|error| format!("Failed to read config file:\n{}", error))?;

    let config_directory = config_path.parent();

    match oxwm::config::parse_lua_config(&config_string, config_directory) {
        Ok(config) => Ok((config, false)),
        Err(_error) => {
            let config = oxwm::config::parse_lua_config(TEMPLATE, None)
                .map_err(|error| format!("Failed to parse default template config:\n{}", error))?;
            Ok((config, true))
        }
    }
}

fn init_config() -> Result<(), Box<dyn std::error::Error>> {
    let config_directory = get_config_path();
    std::fs::create_dir_all(&config_directory)?;

    let config_template = include_str!("../../templates/config.lua");
    let config_path = config_directory.join("config.lua");
    std::fs::write(&config_path, config_template)?;

    println!("✓ Config created at {:?}", config_path);
    println!("  Edit the file and reload with Mod+Shift+R");
    println!("  No compilation needed - changes take effect immediately!");

    Ok(())
}

fn get_config_path() -> PathBuf {
    dirs::config_dir()
        .expect("Could not find config directory")
        .join("oxwm")
}

fn print_help() {
    println!("OXWM - A dynamic window manager written in Rust\n");
    println!("USAGE:");
    println!("    oxwm [OPTIONS]\n");
    println!("OPTIONS:");
    println!("    --init              Create default config in ~/.config/oxwm/config.lua");
    println!("    --config <PATH>     Use custom config file");
    println!("    --version           Print version information");
    println!("    --help              Print this help message\n");
    println!("CONFIG:");
    println!("    Location: ~/.config/oxwm/config.lua");
    println!("    Edit the config file and use Mod+Shift+R to reload");
    println!("    No compilation needed - instant hot-reload!");
    println!("    LSP support included with oxwm.lua type definitions\n");
    println!("FIRST RUN:");
    println!("    Run 'oxwm --init' to create a config file");
    println!("    Or just start oxwm and it will create one automatically\n");
}

fn process_args() -> Args {
    let name = match std::env::args().nth(0) {
        Some(n) => n,
        None => return Args::Error("Program name can't be extracted from args".to_string()),
    };
    let switch = std::env::args().nth(1);
    let path = std::env::args().nth(2);

    let switch = match switch {
        Some(s) => s,
        None => return Args::Arguments(vec![name]),
    };

    match switch.as_str() {
        "--version" => {
            println!("{name} {}", env!("CARGO_PKG_VERSION"));
            Args::Exit
        }
        "--help" => {
            print_help();
            Args::Exit
        }
        "--init" => {
            init_config().expect("Failed to create default config");
            Args::Exit
        }
        "--config" => {
            if let Some(path) = path
                && std::fs::exists(&path).is_ok()
                && std::fs::exists(&path).unwrap() == true
            {
                Args::Arguments(vec![name, switch, path])
            } else {
                Args::Error("Error: --config requires a valid path argument".to_string())
            }
        }
        _ => Args::Error(format!("Error: {switch} is an unknown argument")),
    }
}

fn check_convert(path: &PathBuf) -> Result<(), &str> {
    let config_directory = get_config_path();

    if !path.exists() {
        let ron_path = config_directory.join("config.ron");
        let had_ron_config = ron_path.exists();

        println!("No config found at {:?}", config_directory);
        println!("Creating default Lua config...");
        if init_config().is_err() {
            return Err("Failed to create default lua");
        }

        if had_ron_config {
            println!("\n NOTICE: OXWM has migrated to Lua configuration.");
            println!("   Your old config.ron has been preserved, but is no longer used.");
            println!("   Your settings have been reset to defaults.");
            println!("   Please manually port your configuration to the new Lua format.");
            println!("   See the new config.lua template for examples.\n");
        }
    }
    Ok(())
}
