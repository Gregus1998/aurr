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
use std::fmt::Debug;
use std::fmt::Display;
use std::io::{self, Write};
use std::process::exit;

use crate::lib::aurr_core::print_map;
use crate::lib::cloud_storage_managers::CloudResource;
use crate::lib::cloud_storage_managers::CloudServiceManagerTrait;
use crate::lib::template;
use crate::lib::local_setup;



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
    
    match builder.build(){
        Ok(conf) => conf,
        Err(e) => {
            println!("Could not load config due to: {}
            If it is the first time running -> Setup a local enviroment with \"./aurr-manager run-local-setup\"
            If config file does exists, pass it via an optional argument: \"--config=<path/to/Config.toml>\"  
            ",e.to_string());
            exit(13)
        }
    }
}

fn print_config(config:&Option<Config>){
    for e in config.as_ref().unwrap().cache.clone().to_string().trim_matches('{').trim_matches('}').split(","){
        println!("{}",e);
    }
}

#[derive(Debug)]
enum PrintResults<T>{
    Str(String),
    Vec(Vec<T>)
}

///
/// Function to handle a generic print results stuff
/// Will be used to print results to the gui
/// 
impl <T> PrintResults <T> {
    fn print(&self, headder:Option<&str>)
    where
    T: Debug + Display
    {
        match self{
            PrintResults::Str(s) => println!("{}",s),
            PrintResults::Vec(vec) => {
                println!("{}", headder.unwrap_or(""));
                for i in vec.iter(){
                    println!("{}",i)
                }
            }
        }
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
                println!("Error in parsing the arguments: {}",e.to_string());
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

        if self.args.is_empty(){
            ArgParser::print_help();
            exit(1337)
        }

        //Parsing the optional arguments
        match self.option_parser(){
            Ok(_) => (),
            Err(e) => {
                return Err(e);
            }
        }

        let access_key = match self.options.get::<String>(&"account-key".to_string()){
            Some(key) => key.to_string(),
            None => {
                match env::var("AZURE_ACCESS_KEY"){
                    Ok(key) => key,
                    Err(e) => {
                        println!("CLOUD ACCOUNT KEY DOES NOT EXIST - {} - provide key via argument: --account-key=<key>  or ENV_VAR: AZURE_ACCESS_KEY=<key>",e.to_string());
                        exit(8)
                    }
                }
            }
        };


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

        // if any of the arguemnts are "ls" force the ls switch option to be the next value. 
        let switch = match self.args.iter().position(|arg| arg == "ls") {
            None => self.args.last().unwrap().to_ascii_lowercase(),
            Some(i) => {
                if let Some(next) = self.args.get(i + 1) {
                    self.options
                        .insert("ls-option".to_string(), next.to_string());
                }
                "ls".to_string()
            }
        };


        self.aurr_mgmr = Some(AurrCore::new_from_ac(&self.config.as_ref().unwrap()));

        Ok(switch)
    }

    ///
    /// Function to handle the actualt switch.
    /// This function should link whatever switch that is used to the acual function calls later in the program.
    /// Needs to do some error handling here. 
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

                let case_path = match self.options.get("case"){
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

            "grant-access" => {

                let cr = match self.options.get("cloud-resource"){
                    Some(s) => s,
                    None => {
                        error!("Need to provide a ClourdResource: <--cloud-respource=<pathto/cloudresource>>");
                        exit(7)
                    }
                };

                let ct = self.aurr_mgmr.as_ref().unwrap().get_mgmr().get_type();

                match self.aurr_mgmr
                    .as_ref()
                    .unwrap()
                    .get_mgmr()
                    .grant_read_access(
                        CloudResource::from_path(cr, &ct).unwrap(),
                         self.config.as_ref()
                         .unwrap()
                         .get::<u8>("CLOUD_TOKEN_READ_TIMEOUT")
                    .unwrap()).await{
                    Ok(s) => {
                        info!("Access Granted Successfully - Access via: <{}>",s);
                    },
                    Err(e) => {
                        error!("Could not grant-access to cloud resource due to {}",e.to_string());
                        exit(7)
                    }
                }
            },

            "print-config" => {
                print_config(&self.config)
            },

            "help" => {
                ArgParser::print_help()
            },

            "ls" => {

                //Extracting and assigning a new of this thing. 
                let tmpvalue = match &self.options.clone().get("ls-option"){
                    Some(val) => val.to_string(),
                    None => {
                        ArgParser::print_ls_error();
                        exit(123)
                    }
                };

                let lsoption = tmpvalue.split("::").collect::<Vec<&str>>();

                if lsoption.len() == 2{
                    self.options.insert("entry".to_string(), lsoption.last().unwrap().to_string());
                }

                match lsoption.first().unwrap().to_string().as_str(){
                    "tools" => {

                        //Loading the tools
                        let tools = match self.load_tools(){
                            Ok(t) => t,
                            Err(e) => {
                                error!("Could not load tools due to: {}",e.to_string());
                                exit(9)
                            }
                        };

                        // If a entry is listed. this can be displayed isntead of all available tools
                        match self.options.get("entry"){
                            Some(e) => {
                                let tool = match tools.get(e){
                                    Some(t) => t,
                                    None => {
                                        error!("The provided list option: tools::{} is invalid",e);
                                        exit(11)
                                    }
                                };
                                let res:bool= match self.options.get("full-info"){
                                    Some(s) => s.to_ascii_lowercase().to_string() == "true",
                                    None => false
                                };

                                tool.list_tool(res);
                            }

                            None => {
                                let res:bool= match self.options.get("full-info"){
                                    Some(s) => s.to_ascii_lowercase().to_string() == "true",
                                    None => false
                                };
                                for t in tools.values(){
                                    t.list_tool(res);
                                }
                            }
                        }
                    },

                    "config" => {
                        print_config(&self.config);
                    },

                    "case" => {
                        let case_path = match self.options.get("case"){
                            Some(path) => path,
                            None => {
                                error!("Need to provide a valid case template path!\n\tProvide argument: --case=<path>");
                                exit(4)
                            }
                        };
                        let case = match CaseTemplate::load_from_json(case_path){
                            Ok(ct) => ct,
                            Err(e) => {
                                error!("Could not load case template due to: \n\t{}",e.to_string());
                                exit(5)
                            }
                        };
                        case.ls_case();
                    },

                    "container" => {

                        //Checek if the resolution of the list option is set. 
                        match self.options.get("entry"){

                            None => {
                                match self.aurr_mgmr.as_ref().unwrap().get_mgmr().list_containers().await{
                                    Ok(con) => {
                                        PrintResults::Vec(con).print(Some("Containers:"));
                                    },
                                    Err(e) => {
                                        error!("Could not list containers due to: {}",e.to_string());
                                        exit(11)
                                    }
                                };
                            },
                            Some(res) => {

                                match self.aurr_mgmr.as_ref().unwrap().get_mgmr().list_blobs_container(res).await{
                                    Ok(names) => {
                                        if !names.is_empty(){
                                            PrintResults::Vec(names).print(Some(format!("Container: {}",res).as_str()));

                                        }else {
                                            println!("EMPTY");
                                        }
                                    }
                                    Err(e) => {
                                        error!("Could not list blobs in container: {} due to: {}", res, e.to_string());
                                        exit(12)
                                    }
                                }
                            }
                        }



                        
                        
                    }
        
                    _ => {
                        ArgParser::print_ls_error();
                        exit(10)
                    },
                }   
            },

            _ => {
                error!("Invalid or Missing Switch useage: {}",switch);
                ArgParser::print_help()},

        };

        Ok(())
    }

    ///
    /// Function to parse all optional arguments.
    /// 
    pub fn option_parser(&mut self) ->  Result<(), Box<dyn std::error::Error>>{
        
        //mapping over all optional arguments -> Casting them to lowercase
        for args in self.args.iter(){
            
            // If run-local-setup is passed as argument, run local setup and exit with leet+1
            // If any type of "help" is passed -> print help and exit
            if args == "run-local-setup"{
                local_setup::local_setup()?;
                exit(1338)
            }else if args.contains("help"){
                ArgParser::print_help();
                exit(1337);
            };

            if args.starts_with("--"){

                //Adding an incusion for easier list all for arguments.
                if args.ends_with("--list-all") || args.ends_with("--full-info"){
                    &self.options.insert("full-info".to_string(), "true".to_string());
                    continue;
                }
                
                let a = match args.split_once("=") {
                    None => {
                        return Err(format!("Wrong use of optional ARGG >:( uments: {:?}",args).into())
                    },
                    Some((k,v)) => {
                        &self.options.insert(k.to_ascii_lowercase().replace("--",""), v.replace("\"", "").replace("\'", "").to_string())
                    }
                };
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
+-----------------------------------------------+
|                                               |
|       AURR - A Yggdrasil soil project.        |
|       Version: 1.0                            |
|       POC: Jonas S (Mr.Bøttehatt)                           |
|                                               |
+-----------------------------------------------+
+--------+
| Syntax |
+--------+
    ./aurr <Optional Arguments> <Switch>


+-------------------------------+--------------------------------------------------+
|   SWITCH                      |  DESCRIPTION                                     |
+-------------------------------+--------------------------------------------------+
    run-local-setup             // Switch to run a local setup in the current folder. 
                                    Only do this if you export Aurr somewhere. 
                                    No Failchecks. Is called -> Does a jobb!

    Upload                      // Upload a local tool to the cloud
                                    Requires: MA + --tool-config + --entry

    Cloudify                    // Upload and return a URL for a
                                    Requires: MA + --tool-config + --entry

    Grant-Access                // Provides access to a cloud resource already in cloud. 
                                    Requires: MA + --entry

    Run-Case                    // Process a case-template. 
                                    Requires: MA + --case-template

                                    Can be used to full automate a wide set of remote tasks.
                                        - Collect Memory
                                        - Take traige
                                        - Image Disk
                                        - Run Custom tools
                                        - Run Scripts

                                    To set up a custom case-template. Read docs <insert path to guide>

    ls <ls-option>              // Switch to list information about different elements of the framework. 
                                    ls-options:
                                        - tools::<filter>        // List all available tools based on the provided config
                                        - case                   // List information from the provided case - This prints task tempalte aswell!
                                        - config                 // List current running config. Same as \"print-config\"
                                        - cloud (TODO)           // List basic info about the connected cloud
                                        - containers::<filter>   // List available container for the specific azure storage account
                                        - blobs (TODO)           // List a set of blobs from the specific azure storage account        
    
    print-config                //prints the current running config.


+---------------------------+-------------------------------+-------------------------------------------------------------+
|   OPTIONAL-ARGUMENT       |   DEFAULT_VALUES              |   DESCRIPTION                                               |
+---------------------------+-------------------------------+-------------------------------------------------------------+
    --account-key=<Key>                                     // Needer for all interaction with the cloud. 
    --config=<path>         | ./Config.toml                 // Path to the Config.toml -> Default path is ./Config.toml
    --use-default=<bool>    | true                          // Use to run whatever switch with default parameters.  
    --case=<path>                                           // If you want to run a case template. Provide the path to the case template
    --tool-config=<path>    | <INSERT DEFAULT PATH HERE>    // Path to tool configuration <INSERT DEFAULT PATH HERE>
    --entry=<VALUE>                                         // ENTRY in the tool-configuration to use. need to be passed together with '--tool-config'
    --full-info|list-all                                    // Used to list more information when ls is used.   

    # Cloud Specific:
    --blob_name=<VALUE>                                     // Define what blob to list via SWITCH \"ls\"
    --container=<value>                                     // Define what container to list via SWITCH \"ls\"

+----------+
| Examples |
+----------+

# Cmdline to run a local setup. This will create the needed folders and unpack some basic files: 
    -> ./aurr --run-local-setup

# Cmdline to push Surge-Collect to the cloud and return a URL for download.  
    -> ./aurr --config=./Config.toml --tool-config=./data/templates/tools.json --entry=Surge-Collect Cloudify   

# List tools: 
    -> ./aurr ls tools                                      // Lists all tools based on the provided Tools.json file
    -> ./aurr ls tools::Surge-Collect                       // Lists only information about the specified tool \"Surge-Collect\"
    -> ./aurr ls tools::Surge-Collect --list-all            // List all available information.

# List blobs in a container (AZURE CLOUD): 
    -> ./aurr --container=tools ls container 
        ");

        exit(1337)
    }
    
    ///
    /// Just a function to print a error if LS is used wrong :)
    pub fn print_ls_error(){
        error!("Need to provide a valid ls-option LIKE: \n
    - tools::<filter>        // List all available tools based on the provided config
    - case                   // List information from the provided case - This prints task tempalte aswell!
    - config                 // List current running config. Same as \"print-config\"
    - cloud (TODO)           // List basic info about the connected cloud
    - containers::<filter>   // List available container for the specific azure storage account
    - blobs (TODO)           // List a set of blobs from the specific azure storage account        
                        ");
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
}