//Imported modules
mod lib;
//Imports:
use lib::aurr_core::AurrCore;
use lib::azure;
use lib::logging::Logger;
use config::{Config, File, FileFormat};
use once_cell::sync::Lazy;
use lib::tools::{Tool,ToolConfig};
use std::env;
use std::io::{self, Write};



//Global variables. Should not be read!
static mut LOGDIR: Lazy<String> = Lazy::new(||String::new());

/// Function to load the config.toml
/// This function gets called first time in the main. 
/// If global variables should be set, it can be done here. 
fn load_config(path:Option<&str>, access_key: Option<&str>) -> Config{

    let mut builder = Config::builder()
        .add_source(File::new(path.unwrap_or("Config.toml"), FileFormat::Toml).required(true));
    
    if let Some(key) = access_key {
        builder = builder.set_override("AZURE_ACCESS_KEY", key).unwrap();
    }
    
    builder.build().unwrap()
}

#[tokio::main]
async fn main() {
    // Read AZURE_ACCESS_KEY from environment variable first
    let access_key = env::var("AZURE_ACCESS_KEY")
        .expect("AZURE_ACCESS_KEY environment variable not set");
    
    // Initialize the logger with colored terminal output and file logging
    let config = load_config(Some("Config.toml"), Some(&access_key));

    Logger::init(Some(
        config.get::<String>("LOGDIR").unwrap()
    ));


    let tools = Tool::load_from_json::<Tool>("data/templates/tools.json").unwrap();

    let mut toolconfig:ToolConfig = ToolConfig::new();
    toolconfig.search_other_config(config.clone(), "SURGE");
    toolconfig.add("SURGE-SAS-TOKEN".to_string(), "ABCDEFG".to_string());

    let aurr = AurrCore::new_from_ac(&config);

    aurr.upload_tool(tools.get("test").cloned().unwrap()).await.unwrap();

    let a = aurr.get_mgmr().as_azure().unwrap().get_blob_download_url("tools", azure::AzureCloudResource::Text("test".to_string()),10).await;

    println!("{:?}",a.unwrap())
    
}