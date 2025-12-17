use std::path::Path;
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
        Args::Error(e) => panic!("Error: Could not get valid arguments:\n{}", e),
    };

    let (config, had_broken_config) = match load_config(arguments.get(2)) {
        Ok((c, hbc)) => (c, hbc),
        Err(e) => panic!("Error: Could not load config:\n{}", e),
    };

    let mut window_manager = match oxwm::window_manager::WindowManager::new(config) {
        Ok(wm) => wm,
        Err(e) => panic!("Error: Could not start window manager:\n{}", e),
    };

    if had_broken_config {
        window_manager.show_migration_overlay();
    }

    let should_restart = match window_manager.run() {
        Ok(sr) => sr,
        Err(e) => panic!("Error: Could not determine restart\n{}", e),
    };

    drop(window_manager);

    if should_restart {
        use std::os::unix::process::CommandExt;
        let error = std::process::Command::new(&arguments[0])
            .args(&arguments[1..])
            .exec();
        eprintln!("Error: Failed to restart: {}", error);
    }
}

fn load_config(
    config_path: Option<&String>,
) -> Result<(oxwm::Config, bool), Box<dyn std::error::Error>> {
    let path = match config_path {
        None => {
            let config_path = get_config_path().join(CONFIG_FILE);
            check_convert(&config_path)
                .map_err(|error| format!("Error: Failed to check old config:\n{}", error))?;
            config_path
        }
        Some(p) => PathBuf::from(p),
    };

    let config_string = std::fs::read_to_string(&path)
        .map_err(|error| format!("Error: Failed to read config file:\n{}", error))?;

    let config_directory = path.parent();

    match oxwm::config::parse_lua_config(&config_string, config_directory) {
        Ok(config) => Ok((config, false)),
        Err(_error) => {
            let config = oxwm::config::parse_lua_config(TEMPLATE, None).map_err(|error| {
                format!("Error: Failed to parse default template config:\n{}", error)
            })?;
            Ok((config, true))
        }
    }
}

fn init_config() -> Result<(), Box<dyn std::error::Error>> {
    let config_directory = get_config_path();
    std::fs::create_dir_all(&config_directory)?;

    let config_template = TEMPLATE;
    let config_path = config_directory.join(CONFIG_FILE);
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
    let mut args = std::env::args();
    let name = match args.next() {
        Some(n) => n,
        None => return Args::Error("Error: Program name can't be extracted from args".to_string()),
    };
    let switch = args.next();
    let path = args.next();

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
        "--init" => match init_config() {
            Ok(_) => Args::Exit,
            Err(e) => Args::Error(format!("Error: Failed to create default config:\n{e}")),
        },
        "--config" => match check_custom_config(path) {
            Ok(p) => Args::Arguments(vec![name, switch, p]),
            Err(e) => Args::Error(e),
        },
        _ => Args::Error(format!("Error: {switch} is an unknown argument")),
    }
}

fn check_custom_config(path: Option<String>) -> Result<String, String> {
    let path = match path {
        Some(p) => p,
        None => {
            return Err("Error: --config requires a valid path argument".to_string());
        }
    };

    match std::fs::exists(&path) {
        Ok(b) => match b {
            true => Ok(path),
            false => Err(format!("Error: {path} does not exist")),
        },
        Err(e) => Err(format!("Error: Failed to check config exists:\n{e}")),
    }
}

fn check_convert(path: &Path) -> Result<(), &str> {
    let config_directory = get_config_path();

    if !path.exists() {
        let ron_path = config_directory.join("config.ron");
        let had_ron_config = ron_path.exists();

        println!("No config found at {:?}", config_directory);
        println!("Creating default Lua config...");
        if init_config().is_err() {
            return Err("Error: Failed to create default lua");
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
