//Imported modules
mod lib;

use chrono::Local;
//Imports:
//use lib::azure;
use lib::logging::Logger;
use config::{Config, File, FileFormat};
use once_cell::sync::Lazy;
use lib::tools::{Tool,ToolConfig};



//Global variables. Should not be read!
static mut LOGDIR: Lazy<String> = Lazy::new(||String::new());

///Placeholder for all different loaded variables on the local system
enum LoadVariable {
    Tool
}

/// Function to load the config.toml
/// This function gets called first time in the main. 
/// If global variables should be set, it can be done here. 
fn load_config(path:Option<&str>) -> Config{

    let conf = Config::builder()
        .add_source(File::new(path.unwrap_or("Config.toml"), FileFormat::Toml).required(true))
        .build().unwrap();
    conf
}

fn main() {
    // Initialize the logger with colored terminal output and file logging
    let config = load_config(Some("Config.toml"));

    Logger::init(Some(
        config.get::<String>("LOGDIR").unwrap()
    ));

    let tools = Tool::load_from_json::<Tool>("data/templates/tools.json");

    let mut toolconfig:ToolConfig = ToolConfig::new();
    toolconfig.search_other_config(config, "SURGE");
    toolconfig.add("SURGE-SAS-TOKEN".to_string(), "ABCDEFG".to_string());

    println!("{:#?}",tools.unwrap().get("Surge-Collect").unwrap().get_cmdline("Default_Windows_Upload_Azure", toolconfig));

}