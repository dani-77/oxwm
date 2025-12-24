use oxwm::errors::ConfigError;
use oxwm::errors::MainError;
use std::path::Path;
use std::path::PathBuf;

static CONFIG_FILE: &str = "config.lua";
static TEMPLATE: &str = include_str!("../../templates/config.lua");

enum Args {
    Exit,
    Arguments(Vec<String>),
    Error(MainError),
}

fn main() -> Result<(), MainError> {
    let arguments = match process_args() {
        Args::Exit => return Ok(()),
        Args::Arguments(v) => v,
        Args::Error(e) => return Err(e),
    };

    let (config, config_warning) = load_config(arguments.get(2))?;

    let mut window_manager = match oxwm::window_manager::WindowManager::new(config) {
        Ok(wm) => wm,
        Err(e) => return Err(MainError::CouldNotStartWm(e)),
    };

    if let Some(warning) = config_warning {
        window_manager.show_startup_config_error(warning);
    }

    let should_restart = match window_manager.run() {
        Ok(sr) => sr,
        Err(e) => return Err(MainError::BadRestartStatus(e)),
    };

    drop(window_manager);

    if should_restart {
        use std::os::unix::process::CommandExt;
        let error = std::process::Command::new(&arguments[0])
            .args(&arguments[1..])
            .exec();
        eprintln!("Error: Failed to restart: {}", error);
    }
    Ok(())
}

fn load_config(
    config_path: Option<&String>,
) -> Result<(oxwm::Config, Option<ConfigError>), MainError> {
    let path = match config_path {
        None => {
            let config_dir = get_config_path()?;
            let config_path = config_dir.join(CONFIG_FILE);
            check_convert(&config_path)?;
            config_path
        }
        Some(p) => PathBuf::from(p),
    };

    let config_string = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(e) => return Err(MainError::FailedReadConfig(e)),
    };

    let config_directory = path.parent();

    let (mut config, config_warning) =
        match oxwm::config::parse_lua_config(&config_string, config_directory) {
            Ok(config) => (config, None),
            Err(warning) => {
                let config = match oxwm::config::parse_lua_config(TEMPLATE, None) {
                    Ok(c) => c,
                    Err(e) => return Err(MainError::FailedReadConfigTemplate(e)),
                };
                (config, Some(warning))
            }
        };
    config.path = Some(path);
    Ok((config, config_warning))
}

fn init_config() -> Result<(), MainError> {
    let config_directory = get_config_path()?;
    if let Err(e) = std::fs::create_dir_all(&config_directory) {
        return Err(MainError::CouldNotCreateConfigDir(e));
    }

    let config_template = TEMPLATE;
    let config_path = config_directory.join(CONFIG_FILE);
    if let Err(e) = std::fs::write(&config_path, config_template) {
        return Err(MainError::CouldNotWriteConfig(e));
    }

    println!("✓ Config created at {:?}", config_path);
    println!("  Edit the file and reload with Mod+Shift+R");
    println!("  No compilation needed - changes take effect immediately!");

    Ok(())
}

fn get_config_path() -> Result<PathBuf, MainError> {
    match dirs::config_dir() {
        Some(p) => Ok(p.join("oxwm")),
        None => Err(MainError::NoConfigDir),
    }
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
        None => return Args::Error(MainError::NoProgramName),
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
            Err(e) => Args::Error(e),
        },
        "--config" => match check_custom_config(path) {
            Ok(p) => Args::Arguments(vec![name, switch, p]),
            Err(e) => Args::Error(e),
        },
        _ => Args::Error(MainError::InvalidArguments),
    }
}

fn check_custom_config(path: Option<String>) -> Result<String, MainError> {
    let path = match path {
        Some(p) => p,
        None => {
            return Err(MainError::NoConfigPath);
        }
    };

    match std::fs::exists(&path) {
        Ok(b) => match b {
            true => Ok(path),
            false => Err(MainError::BadConfigPath),
        },
        Err(e) => Err(MainError::FailedCheckExist(e)),
    }
}

fn check_convert(path: &Path) -> Result<(), MainError> {
    let config_directory = get_config_path()?;

    if !path.exists() {
        let ron_path = config_directory.join("config.ron");
        let had_ron_config = ron_path.exists();

        println!("No config found at {:?}", config_directory);
        println!("Creating default Lua config...");
        init_config()?;

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
