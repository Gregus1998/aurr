// The local setup will be a module to create the folder structure locally. 
// The goal will be that the raw executable can be exported to any system.

use std::{fs,process::exit};
/// root/
///     aurr.exe/aurr (Binary)
///     Config.toml
///     data/
///         templates/
///         tools/
/// 

const DEFAULT_CONFIG: &str = include_str!("../../../aurr-manager/data/custom/Config.toml");
const DEFAULT_TOOLS_TEMPALTE: &[u8] = include_bytes!("../../../aurr-manager/data/custom/tools.json");
const DEFAULT_WINDOWS_CASE_TEMPLATE: &[u8] = include_bytes!("../../../aurr-manager/data/templates/case_templates/windows_case_template.json");
const DEFAULT_LINUX_CASE_TEMPLATE: &[u8] = include_bytes!("../../../aurr-manager/data/templates/case_templates/linux_case_template.json");
const DEFAULT_WINDOWS_TASK_TEMPLATE: &[u8] = include_bytes!("../../../aurr-manager/data/templates/task_templates/Windows_Generic.json");
const DEFAULT_LINUX_TASK_TEMPLATE: &[u8] = include_bytes!("../../../aurr-manager/data/templates/task_templates/Linux_Generic.json");
const DEFAULT_README:&str = include_str!("../../../aurr-manager/data/custom/README.md");

///
/// Function to setup a local envorioment with included packet default example files. 
/// 
pub fn local_setup() -> std::io::Result<()>{

    //Adding some failsafes to ensure that you dont overwrite your current config :()
    println!("Do you want to run a local setup? This will potentially overwrite files in the current dir/subdir? (yes/no)");

    let mut s = String::new();
    std::io::stdin().read_line(&mut s)?;
    
    if !(s.to_ascii_lowercase().contains("yes")){
        println!("You passed {} - Exiting Local Setup!", s);
        exit(1336)
    };

    match fs::File::open("./Config.toml"){
        Ok(_) => {
            println!("ConfigFile DETECTED: ./Config.toml - By continuing you will lose this content.\nDo you want to continue? (yes/no)");
            let mut r = String::new();
            std::io::stdin().read_line(&mut r)?;

            if !(r.to_ascii_lowercase().contains("yes")){
                println!("You passed {} - Exiting Local Setup!", r);
                exit(1336)
            };
        },
        Err(_) => ()
    }
    
    //Creating some directories
    fs::create_dir_all("./data/tools")?;
    fs::create_dir_all("./data/templates/case_templates/")?;
    fs::create_dir_all("./data/templates/task_templates/")?;
    
    //Writing the content to different files. 
    fs::write("./Config.toml", DEFAULT_CONFIG)?;
    fs::write("README.md", DEFAULT_README)?;
    fs::write("./data/templates/tools.json", DEFAULT_TOOLS_TEMPALTE)?;
    fs::write("./data/templates/case_templates/windows_case_template.json", DEFAULT_WINDOWS_CASE_TEMPLATE)?;
    fs::write("./data/templates/case_templates/linux_case_template.json", DEFAULT_LINUX_CASE_TEMPLATE)?;
    fs::write("./data/templates/task_templates/windows_task_template.json", DEFAULT_WINDOWS_TASK_TEMPLATE)?;
    fs::write("./data/templates/task_templates/linux_task_template.json", DEFAULT_LINUX_TASK_TEMPLATE)?;

    Ok(())
}