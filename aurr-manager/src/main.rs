//Imported modules
mod lib;
use azure_storage_blobs::prelude::BlobClient;
//Imports:
use lib::aurr_core::AurrCore;
use lib::azure;
use lib::template::*;
use lib::logging::Logger;
use config::{Config, File, FileFormat};
use once_cell::sync::Lazy;
use lib::tools::{Tool,ToolConfig};
use std::collections::HashMap;
use std::env;
use std::io::{self, Write};
use std::process::exit;

use crate::lib::cloud_storage_managers::CloudServiceManagerTrait;
use crate::lib::template;



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

fn print_config(config:&Option<Config>){
    for e in config.as_ref().unwrap().cache.clone().to_string().split(","){
        println!("{}",e);
    }
}


struct ArgParser{
    args:Vec<String>,
    options: HashMap<String,String>,
    config:Option<Config>,
    aurr_mgmr:Option<AurrCore>
}


///
/// A Argument Parser struct to handle all the interaction between the calling of the program and the execution flow
/// This will be the shell to decide the further execution. 
/// 
impl ArgParser{

    pub async fn new() -> Result<(), Box<dyn std::error::Error>>{
        //Creating the argparser
        let mut argparser = ArgParser { args: std::env::args().collect::<Vec<String>>(), options: HashMap::new(), config:None, aurr_mgmr:None };     
        
        //Parsing the argiuments + initiating the switch + loading the config and returning switch statement
        let switch = match argparser.parse_arguemnts(){
            Ok(s) => s,
            Err(e) => {
                error!("{}",e.to_string());
                exit(1);
            }
        };
        
        //Pass the switch to the handle_switch function. This should point to a set of function calls based on what to do. 
        match argparser.parse_switch(switch).await{
            Ok(_) => Ok(()),
            Err(e) => {
                error!("{}",e.to_string());
                exit(2)
            }
        }
    }

    pub fn parse_arguemnts(&mut self) -> Result<String, Box<dyn std::error::Error>>{

        // Read AZURE_ACCESS_KEY from environment variable first
        let access_key = env::var("AZURE_ACCESS_KEY")
            .expect("AZURE_ACCESS_KEY environment variable not set");

        //Parsing the optional arguments
        match self.option_parser(){
            Ok(_) => (),
            Err(e) => {
                error!("{}",e);
                ArgParser::print_help();
            }
        }


        //Loading the config based on the provided optional arguments
        self.config = Some(
            match self.options.get("config"){
                Some(path) => load_config(Some(path), Some(&access_key)),
                None => load_config(Some("Config.toml"), Some(&access_key))
            }
        );

        //Initiating the logger
        Logger::init(Some(
        self.config.as_ref().unwrap().get::<String>("LOGDIR").unwrap()
        ));

        let switch = self.args.last().unwrap().to_ascii_lowercase().to_string();

        self.aurr_mgmr = Some(AurrCore::new_from_ac(&self.config.as_ref().unwrap()));

        Ok(switch)
    }

    ///
    /// Function to handle the actualt switch.
    /// This function should link whatever switch that is used to the acual function calls later in the program. 
    /// 
    pub async fn parse_switch(&mut self, switch:String) -> Result<(), Box<dyn std::error::Error>>{
        
        match switch.as_str(){
            //Switch-case for upload
            "upload" => {
                let tools = self.load_tools().unwrap();

                //Some flow to get the tool to upload
                let tool = match self.options.get("entry") {
                    Some(tool) => match tools.get(tool) {
                        Some(t) => t,
                        None => {
                            error!("Invalid tool entry for OA --entry={}",tool);
                            return Err("Invalid tool entry".into());
                        }
                        
                    },
                    None => {
                        error!("Switch 'cloudify' requires '--entry=<a_tool_2_upload>'");
                        return Err("missing OA '--entry'".into());
                    }
                };

                match self.aurr_mgmr.as_ref().unwrap().upload_tool(tool.clone()).await{
                    Ok(cr) => {

                        info!("Uploaded: <{}> to <{}> <{}> <{}>", tool.name,self.aurr_mgmr.as_ref().unwrap().get_mgmr().get_type(), self.aurr_mgmr.as_ref().unwrap().get_mgmr().get_name(), cr.get_info().unwrap())

                    },
                    Err(e) => {
                        error!("{}",e.to_string());
                        exit(3)
                    }
                };

            },

            "cloudify" => {
                let tools = self.load_tools().unwrap();

                //Some flow to get the tool to upload
                let tool = match self.options.get("entry") {
                    Some(tool) => match tools.get(tool) {
                        Some(t) => t,
                        None => {
                            eprint!("Invalid tool entry for OA --entry={}",tool);
                            return Err("Invalid tool entry".into());
                        }
                        
                    },
                    None => {
                        eprint!("Switch 'cloudify' requires '--entry=<a_tool_2_upload>'");
                        return Err("missing OA '--entry'".into());
                    }
                };

                let url = match tool.cloudify(self.aurr_mgmr.as_ref().unwrap().get_mgmr(), self.config.as_ref().unwrap()).await{
                    Ok(url) => url,
                    Err(e) => {
                        error!("{:?}",e);
                        return Err(e);
                    }
                };

                info!("{} Download via: <{}>", tool.name, url)
            },

            "run-case" => {
                let mut tools = self.load_tools().unwrap();
                let path:String = self.options.get("case-template").unwrap().to_string();

                let case_path = match self.options.get("case-template"){
                    Some(path) => path,
                    None => {
                        error!("Need to provide a valid case template");
                        exit(4)
                    }
                };

                let case = match CaseTemplate::load_from_json(case_path){
                    Ok(ct) => ct,
                    Err(e) => {
                        error!("Could not load case template due to: {}",e.to_string());
                        exit(5)
                    }
                };

                match self.aurr_mgmr.as_ref().unwrap().tools_push_execute(&mut tools, case.clone(), self.config.as_ref().unwrap()).await{
                    Ok(results) => {

                        info!("Run at target: <{}>", results);
                    },

                    Err(e) => {
                        error!("Could not generate pull'n execute script for case: {} due to: {}", case.name, e.to_string());
                        exit(6)
                    }
                };

            },
            "grant-access" => {todo!("same as the other switch cases")},
            "print-config" => print_config(&self.config),
            _ => ArgParser::print_help(),

        };

        Ok(())
    }

    ///
    /// Function to parse all optional arguments.
    /// 
    pub fn option_parser(&mut self) ->  Result<(), Box<dyn std::error::Error>>{
        
        //mapping over all optional arguments -> Casting them to lowercase
        for args in self.args.iter(){
            if args.starts_with("--"){
                let a = args.split("=").collect::<Vec<&str>>();

                if a.len() != 2{
                    return Err(format!("Wrong use of optional ARGG >:( uments: {:?}",args).into());
                }

                //Inserting option arguments to the optional argument map. removes extrac chars and so on.
                let _r = &self.options.insert(a.first().unwrap().to_ascii_lowercase().replace("--",""), a.last().unwrap().replace("\"", "").replace("\'", "").to_string());
            }
        }

        //If there are no optional arguments -> just add use-default. 
        if self.options.is_empty(){
            self.options.insert("use-default".to_string(), "True".to_string());
        }

        Ok(())

    }

    pub fn print_help(){

        println!("

AURR - A Yggdrasil soil project.
    Version: 1.0
    POC: Jonas S

Syntax: 
    <Somebinary> <Mandatory Arguments (MA)> <Optional Arguments (OA)> <Switch>

Switch: 
    Upload                      // Upload a local tool to the cloud
                                    Requires: MA + --tool-config + --entry

    Cloudify                    // Upload and return a URL for a
                                    Requires: MA + --tool-config + --entry

    Grant-Access                // Provides access to a cloud resource already in cloud. 
                                    Requires: MA + --cloud-resource

    Run-Case                    // Process a case-template. 
                                    Requires: MA + --case-template

                                    Can be used to full automate a wide set of remote tasks.
                                        - Collect Memory
                                        - Take traige
                                        - Image Disk
                                        - Run Custom tools
                                        - Run Scripts

                                    To set up a custom case-template. Read docs <insert path to guide>

Mandatory Arguments (MA):
    --account-key=<Key>         // Needer for all interaction with the cloud. 

Optional Arguemnts (OA):
    --config=<path>             //Path to the Config.toml -> Default path is ./Config.toml
    --use-default=bool          // Use to run whatever switch with default parameters.  
    --case-template=<path>      // If you want to run a case template. Provide the path to the case template
    --tool-config=<path>        // Path to tool configuration <INSERT DEFAULT PATH HERE>
    --entry=<VALUE>             // ENTRY in the tool-configuration to use. need to be passed together with '--tool-config'

    --cloud-resource=<PATH>     // A cloud resource. Provide a cloud resource path: tools/Surge-Collect

Example: 
# Cmdline to push Surge-Collect to the cloud and return a URL for download.  
    ./aurr --config=./Config.toml --tool-config=./data/templates/tools.json --entry=Surge-Collect Cloudify    

        ");

        exit(1337)
    }
    
    ///
    /// Function to load the tools eighter based on running config or provided arguments. 
    /// Provided arguments should be default.
    /// 
    pub fn load_tools(&self) -> Result<HashMap<String,Tool>, Box<dyn std::error::Error>>{

        let a = match self.config.clone().unwrap().get::<String>("LOCAL_TOOL_INDEX"){
                    Ok(path) => path,
                    Err(e) => {
                        return Err(e.into());
                    }
                };

        let toolconfig = match self.options.get("tool-config"){
            Some(conf) => conf,
            None => &a
        };


        let tools = Tool::load_from_json(toolconfig).unwrap();
        Ok(tools)
    }
}

#[tokio::main]
async fn main() {

    let  ap = ArgParser::new().await.unwrap();

    let mut tools = Tool::load_from_json::<Tool>("data/templates/tools.json").unwrap();

    //let case = template::CaseTemplate::load_from_json("/home/cyfjonass/aurr/aurr-manager/data/templates/case_template.json");

    //let tasks = case.unwrap().build_task_list(tools.clone(), &config);
    //let aurr = AurrCore::new_from_ac(&config);
 
    //aurr.tools_push_execute(&mut tools, case.unwrap().clone(), &config).await.unwrap();

    /*
    let mut toolconfig:ToolConfig = ToolConfig::new();
    toolconfig.search_other_config(config.clone(), "SURGE");
    toolconfig.add("SURGE-SAS-TOKEN".to_string(), "ABCDEFG".to_string()); 
    let aurr = AurrCore::new_from_ac(&config);

    aurr.upload_tool(tools.get("test").cloned().unwrap()).await.unwrap();

    let b = aurr.get_mgmr().as_azure().unwrap().list_blobs("tools").await;
    let a = aurr.get_mgmr().as_azure().unwrap().get_blob_download_url("tools", azure::AzureCloudResource::Text("kape.zip".to_string()),10).await;

    println!("{:?}",a.unwrap())
    */
}