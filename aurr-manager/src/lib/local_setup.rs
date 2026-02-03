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
const DEFAULT_TOOLS_TEMPALTE: &[u8] = include_bytes!("../../../aurr-manager/data/custom/Tools.json");
const DEFAULT_CASE_TEMPALTE: &[u8] = include_bytes!("../../../aurr-manager/data/custom/CaseTemplate.json");
const DEFAULT_TASK_TEMPALTE: &[u8] = include_bytes!("../../../aurr-manager/data/custom/TaskTemplate.json");
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
    fs::write("./data/templates/Tools.json", DEFAULT_TOOLS_TEMPALTE)?;
    fs::write("./data/templates/case_templates/CaseTemplate.json", DEFAULT_CASE_TEMPALTE)?;
    fs::write("./data/templates/task_templates/TaskTemplate.json", DEFAULT_TASK_TEMPALTE)?;

    Ok(())
}