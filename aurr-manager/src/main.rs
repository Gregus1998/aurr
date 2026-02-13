//Imported modules
mod lib;
use azure_storage_blobs::prelude::BlobClient;
use config::ConfigBuilder;
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
use std::process::ExitCode;
use std::process::exit;
use serde::Deserialize;

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
        Ok(conf) => {
            conf
        },
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

    let mut v:Vec<String> = Vec::new();
    for e in config.as_ref().unwrap().cache.clone().to_string().trim_matches('{').trim_matches('}').split(","){
        v.push(e.to_string());
    }

    v.sort();
    println!("{:#?}",v);
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
        match argparser.parse_arguemnts(){
            Ok(s) => s,
            Err(e) => {
                println!("Error in parsing the arguments: {}",e.to_string());
                exit(1);
            }
        };
        
        //Pass the switch to the handle_switch function. This should point to a set of function calls based on what to do. 
        match argparser.parse_switch().await{
            Ok(_) => Ok(()),
            Err(e) => {
                error!("{}",e.to_string());
                exit(2)
            }
        }
    }

    ///
    /// A internal function to check and add the account key if needed.
    /// 
    fn check_add_account_key(&mut self) ->  Result<(), Box<dyn std::error::Error>>{

        //If the key is not empty, we should do something
        if self.config.as_ref().unwrap().get::<String>("AZURE_ACCESS_KEY").unwrap().is_empty(){

            //Getting the key if it is provided. eighter via the optional arguments or the env_variables
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

            //Building a new config where I add the new key
            let new_config = Config::builder()
                .add_source(self.config.clone().unwrap())
                .set_override("AZURE_ACCESS_KEY", access_key)?
                .build()?;

            //overwrites the 
            self.config = Some(new_config);
        }

        Ok(())

    
    }

    ///
    /// A function to check the account key and init the connection to cloud
    /// 
    
    fn init_mgmr(&mut self) -> Result<(), Box<dyn std::error::Error>>{

        self.check_add_account_key()?;
        //this will just create a new azyre clloud thingy. Need to add a support based on a config here. 
        self.aurr_mgmr = Some(AurrCore::new_from_ac(&self.config.as_ref().unwrap()));
        Ok(())   
    }

    ///
    /// A function to get a config value from the current struct.config()
    /// Just because I am lazy and dont want to write self.option.unwrap().get::<T>() every time :()
    /// 
    fn get<'a,T>(&self,key:&'a str) -> Option<T>
    where 
    T: Deserialize<'a>
    {
        match self.config.as_ref().unwrap().get::<T>(key){
            Ok(a) => Some(a),
            Err(_) => None
        }
    }

    ///
    /// A function to parse all the arguments that are passed to the function.
    /// 
    pub fn parse_arguemnts(&mut self) -> Result<(), Box<dyn std::error::Error>>{

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

        //Loading the config based on the provided optional arguments
        self.config = Some(
            match self.options.get("config"){
                Some(path) => load_config(Some(path), None),
                None => load_config(Some("Config.toml"), None)
            }
        );

        //Initiating the logger
        Logger::init(Some(
        self.config.as_ref().unwrap().get::<String>("LOGDIR").unwrap()
        ));

        Ok(())
    }

    ///
    /// Function to handle the actualt switch.
    /// This function should link whatever switch that is used to the acual function calls later in the program.
    /// Needs to do some error handling here. 
    /// 
    pub async fn parse_switch(&mut self) -> Result<(), Box<dyn std::error::Error>>{

        //extracting the switch option and the switch arguments
        let switch = &self.args[1];
        let switch_options = self.args.split_at(2).1.to_owned();

        match switch.as_str(){

            "run-local-setup" => {
                match local_setup::local_setup(){
                    Ok(_) => info!("Local setup was sucessfully!"),
                    Err(e) => {
                        error!("Could not complete local setup due to:  {}",e.to_string());
                        exit(99)
                    }
                }
            }

            //Switch-case for upload
            "upload" => {
                self.init_mgmr().unwrap();
                let lsoption = &switch_options[0].split("::").collect::<Vec<&str>>();

                match *lsoption.first().unwrap(){
                    "tools" => {

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

                        match self.aurr_mgmr.as_ref().unwrap().upload_tool(tool.clone(), Some(&self.config.as_ref().unwrap().get::<String>("CLOUD_DEFAULT_UPLOAD_LOCATION").unwrap())).await{
                            Ok(cr) => {

                                info!("Uploaded: <{}> to <{}> <{}> <{}>", tool.name,self.aurr_mgmr.as_ref().unwrap().get_mgmr().get_type(), self.aurr_mgmr.as_ref().unwrap().get_mgmr().get_name(), cr.get_info().unwrap())

                            },
                            Err(e) => {
                                error!("{}",e.to_string());
                                exit(3)
                            }
                        };

                    },

                    "file" => {
                        //removing the "file option from the switch optios. -> This should have been done before the switch, but whatever :)"
                        let files = switch_options[1..].to_vec();
                        let mut s:String = String::new();

                        println!("Are you sure you want to cloudify the following tools:",);
                        for t in files.iter(){
                            println!("  {}",t);
                        }

                        print!("Answer(yes/no): ");
                        io::stdout().flush().unwrap();
                        std::io::stdin().read_line(&mut s).unwrap();

                        if s.contains("yes"){


                            for t in files.iter(){
                                let ttool = match Tool::new_from_path(t){
                                    Ok(val) => val,
                                    Err(e) => {
                                        error!("Could not toolify path: {} due to: {}", t,e.to_string());
                                        exit(2)
                                    }
                                };

                            }
                        }else{
                            error!("Aborting upload");
                            exit(17)
                        }
                    },

                    _ => {
                    error!("The provided upload option: <{}> is not supported!",*lsoption.first().unwrap())
                    }
                };


            },

            "cloudify" => {

                self.init_mgmr().unwrap();

                let lsoption = &switch_options[0].split("::").collect::<Vec<&str>>();

                match *lsoption.first().unwrap(){
                    
                    "tools" => {
                        //Initiates the tool index
                        let tools = self.load_tools().unwrap();

                        // If the tool is provided via the syntax tools::Some_tool. this supports it.  
                        match lsoption.get(1){
                            Some(val) => {
                                self.options.insert("entry".to_string(), val.to_string());},
                            None => ()
                        };

                        //Some flow to get the tool to upload
                        let tool = match self.options.get("entry") {
                            Some(tool) => match tools.get(tool) {
                                Some(t) => t,
                                None => {
                                    error!("Invalid tool entry - <{}> does not exist in tools index: {}",tool, self.config.as_ref().unwrap().get::<String>("LOCAL_TOOL_INDEX").unwrap_or("N/A".to_string()));
                                    return Err("Invalid tool entry".into());
                                }
                                
                            },
                            None => {
                                error!("Switch 'cloudify' requires '--entry=<a_tool_2_upload>'");
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

                    "file" => {

                        //removing the "file option from the switch optios. -> This should have been done before the switch, but whatever :)"
                        let files = switch_options[1..].to_vec();
                        let mut s:String = String::new();

                        println!("Are you sure you want to cloudify the following tools:",);
                        for t in files.iter(){
                            println!("  {}",t);
                        }

                        print!("Answer(yes/no): ");
                        io::stdout().flush().unwrap();
                        std::io::stdin().read_line(&mut s).unwrap();

                        if s.contains("yes"){
                            for t in files.iter(){
                                let ttool = match Tool::new_from_path(t){
                                    Ok(val) => val,
                                    Err(e) => {
                                        error!("Could not toolify path: {} due to: {}", t,e.to_string());
                                        exit(2)
                                    }
                                };

                                match ttool.cloudify(self.aurr_mgmr.as_ref().unwrap().get_mgmr(), self.config.as_ref().unwrap()).await{
                                    Ok(s)  => info!("Download {} via <{}>",ttool.name, s),
                                    Err(e) => error!("Could not cloudify file: {} due to {}",ttool.name, e.to_string())
                                };

                            }
                        }else{
                            error!("Aborting upload");
                            exit(17)
                        }
                    }

                    _ => {
                        error!("The provided cloudify option: <{}> is not supported!",*lsoption.first().unwrap())
                    }
                }

                
            },

            "run-case" => {

                self.init_mgmr().unwrap();

                let mut tools = self.load_tools().unwrap();

                let case_path = match self.options.get("case"){
                    Some(path) => path,
                    None => {
                        match switch_options.first(){
                            Some(s) => s,
                            None => {
                                error!("Need to provide a valid case template
    To list all availabe cases run: <ls case> -> Provide case via: <run-case path> or <run-case --case=<path>>");
                                
                                exit(4)
                            }
                        }
                    }
                };

                let case = match CaseTemplate::load_from_json(case_path){
                    Ok(ct) => ct,
                    Err(e) => {
                        error!("Could not load case template due to: {}",e.to_string());
                        exit(5)
                    }
                };

                match self.aurr_mgmr.as_ref().unwrap().tools_push_execute(&mut tools, case.clone(), self.config.as_mut().unwrap()).await{
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

                let lsoption = &switch_options[0].split("::").collect::<Vec<&str>>();



                self.init_mgmr().unwrap();

                let cr = match self.options.get("cloud-resource"){
                    Some(s) => &s.replace("::", "/"),
                    None => {
                        match switch_options.get(0){
                            Some(ss) => &ss.replace("::", "/"),
                            None => {
                                error!("Missing or Invalid Cloud resource: <{:?}>
    Provide a resource on the following syntax: 
    --cloud-resource=container::blob || --cloud-resource=container/blob || grant-access container::blob",switch_options.get(0));
                                exit(7)
                            }
                        }
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

                //ls is only supposed to take one argument
                //supports the syntax <lsoption>::<somefilter>
                let lsoption = &switch_options[0].split("::").collect::<Vec<&str>>();

                match *lsoption.first().unwrap(){
                    "tools" => {

                        //Loading the tools
                        let tools = match self.load_tools(){
                            Ok(t) => t,
                            Err(e) => {
                                error!("Could not load tools due to: {}",e.to_string());
                                exit(9)
                            }
                        };

                        let res:bool= match self.options.get("full-info"){
                                    Some(s) => s.to_ascii_lowercase().to_string() == "true",
                                    None => false
                                };

                        //Creating a filter for what tools to use
                        let filter = match lsoption.get(1){
                            None => "",
                            Some(e) => {
                                if *e == "all" || *e == "full"{
                                    ""
                                }else {
                                    e
                                }
                            }
                        };

                        for tool in tools.values(){

                            let print = tool.list_tool(res);

                            if print.contains(filter){
                                println!("{}",print)
                            }

                        }
                    },

                    "config" => {
                        print_config(&self.config);
                    },

                    "case" => {

        
                        // Match statement to support the "--case" optional argument.
                        match self.get::<String>("case"){
                            Some(path) => {
                                let case = match CaseTemplate::load_from_json(&path){
                                    Ok(ct) => ct,
                                    Err(e) => {
                                        error!("Could not load case template due to: \n\t{}",e.to_string());
                                        exit(5)
                                    }
                                };

                                case.ls_case();
                            },

                            None => {
                                let filter = match lsoption.get(1){
                                    None => "",
                                    Some(s) => s
                                };

                                match std::fs::read_dir(self.get::<String>("DEFAULT_CASE_DIR").unwrap()){
                                    Ok(s) => {

                                        for e in s.flatten(){
                                            let apath = e.path().to_string_lossy().to_string();

                                            let case = match CaseTemplate::load_from_json(&apath){
                                                Ok(ct) => ct,
                                                Err(e) => {
                                                    error!("Could not load case template due to: \n\t{}",e.to_string());
                                                    exit(5)
                                                }
                                            };
                                            
                                            let print = case.ls_case();

                                            if print.to_lowercase().contains(&filter.to_ascii_lowercase()){
                                                println!("CASE: <{}>:",apath);
                                                println!("{}",print);
                                            }

                                            

                                            
                                        }
                                            
                                        
                                    },
                                    Err(e) => {
                                        error!("Could not read case dir due to: {}",e.to_string());
                                        exit(1337)
                                    }
                                };

                            }
                        };
                    },
                    

                    "container" => {

                        self.init_mgmr().unwrap();

                        // if any filter is passed. Fix the option.
                        match lsoption.get(1){
                            None => {},
                            Some(s) => {
                                if !s.is_empty(){
                                    self.options.insert("entry".to_string(), s.to_string());
                                }
                            }
                        }
                        
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
    /// This function will extract all optional arguments "--<Key>=<Value>" and add it to the runtime config with the entry <Key> => <Value>
    /// This can be used to alter any variable in the automation or execution.
    /// Use with care
    /// 
    /// The function will remove all optional arguments from the argument vector so that this can be used later for some fanzy stuff. 
    /// 
    pub fn option_parser(&mut self) ->  Result<(), Box<dyn std::error::Error>>{
         //New vector to collect all args that are not optional arguments.
        let mut new_args:Vec<String> = Vec::new();
        
        //mapping over all optional arguments -> Casting them to lowercase
        for args in self.args.iter(){

            if args.ends_with("--help"){
                ArgParser::print_help();
                exit(0)
            }

            if args.starts_with("--"){

                //Adding an incusion for easier list all for arguments.
                if args.ends_with("--list-all") || args.ends_with("--full-info") || args.ends_with("--all"){
                    self.options.insert("full-info".to_string(), "true".to_string());
                    continue;
                }
                
                match args.split_once("=") {
                    None => {
                        return Err(format!("Wrong use of optional argument: {:?}\n\tTo print help: ./aurr --help",args).into())
                    },
                    Some((k,v)) => {
                        &self.options.insert(k.to_ascii_lowercase().replace("--",""), v.replace("\"", "").replace("\'", "").to_string())
                    }
                };


            }else {
                new_args.push(args.to_string());
            }
        }

        //If there are no optional arguments -> just add use-default. 
        if self.options.is_empty(){
            self.options.insert("use-default".to_string(), "True".to_string());
        }

        //Whenever optional arguments is passed, use a set of new arguments for logic. 
        self.args = new_args;

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
    ./aurr <Switch> <Optional Arguments> 


+-------------------------------+--------------------------------------------------+
|   SWITCH                      |  DESCRIPTION                                     |
+-------------------------------+--------------------------------------------------+
    run-local-setup             // Switch to run a local setup in the current folder. 
                                    Only do this if you export Aurr somewhere. 
                                    No Failchecks. Is called -> Does a jobb!

    Upload                      // Upload a local tool/resource to the cloud
                                    Requires: 
                                        --account-key
                                    
                                    Call Options:
                                        - upload tools::<tool_name>
                                        - upload file <filepath1> <filepath2> .. <filepath_N>  

    Cloudify                    // Upload a local tool / resource and return a download URL
                                    Requires: 
                                        --account-key

                                    Call Options:
                                        - upload tools::<tool_name>
                                        - upload file <filepath1> <filepath2> .. <filepath_N> 

    Grant-Access                // Provides access to a cloud resource already in cloud. 
                                    Requires: --account-key

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
                                        - case::<filter>         // List information from the provided case - This prints task tempalte aswell!
                                        - config                 // List current running config. Same as \"print-config\"
                                        - cloud (TODO)           // List basic info about the connected cloud
                                        - container::<filter>    // List available container for the specific azure storage account
                                        - blobs (TODO)
                                        - search container::<filter>       
    
    print-config                //prints the current running config.


+---------------------------+-------------------------------+-------------------------------------------------------------+
|   OPTIONAL-ARGUMENT       |   DEFAULT_VALUES              |   DESCRIPTION                                               |
+---------------------------+-------------------------------+-------------------------------------------------------------+
    --account-key=<Key>                                     // Needer for all interaction with the cloud. 
    --config=<path>         | ./Config.toml                 // Path to the Config.toml -> Default path is ./Config.toml
    --use-default=<bool>    | true                          // Use to run whatever switch with default parameters.  
    --case=<path>                                           // If you want to run a case template. Provide the path to the case template
    --tool-config=<path>    | ./data/templates/tools.json   // Path to tool configuration <INSERT DEFAULT PATH HERE>
    --entry=<VALUE>                                         // ENTRY in the tool-configuration to use. need to be passed together with '--tool-config'
    --full-info|list-all                                    // Used to list more information when ls is used.   

+----------+
| Examples |
+----------+

# Cmdline to run a local setup. This will create the needed folders and unpack some basic files: 
    -> ./aurr --run-local-setup                                                 //Runs a local setup. Should make it easy to pass the tool around

# Examples of Cloudify  
    -> ./aurr --account-key=<key> cloudify tools::<tool_name>                   // Upload a tool to the cloud by config and tool config.    
    -> ./aurr --account-key=<key> cloudify path/to/file1 path/to/file2          //Uploads the targeted files to the cloud.  

# List tools: 
    -> ./aurr ls tools                                                          // Lists all tools based on the provided Tools.json file
    -> ./aurr ls tools::<tool_name>                                             // Lists only information about the specified tool \"Surge-Collect\"
    -> ./aurr ls tools::<tool_name> --list-all                                  // List all available information.

# List blobs in a container (AZURE CLOUD): 
    -> ./aurr ls container                                                      // Lists all containers in the cloud-root 
    -> ./aurr ls container::upload                                              // Lists content of a specific container. \"upload\" can be changed to any container in the cloud-root 

# Example of run a case: 
    -> ./aurr --account-key=<key> run-case <case_path>                          // Runs a set of TaskTemplates based on a case_tempalte. 
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