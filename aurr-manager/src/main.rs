//Imported modules
mod lib;

use azure_core::time;
use clap::ArgGroup;
use colored::Colorize;
use crossterm::style::Stylize;
use futures::future::ok;
//Imports:
use lib::aurr_core::AurrCore;
use lib::template::*;
use lib::logging::Logger;
use config::{Config, File, FileFormat};
use lib::tools::{Tool};
use tracing_subscriber::filter::targets;
use std::collections::HashMap;
use std::env;
use std::fmt::Debug;
use std::fmt::Display;
use std::io::{self, Write};
use std::process::exit;
use serde::Deserialize;
use clap::{Parser, Subcommand};


use crate::lib::azure::AzureCloudResource;
use crate::lib::cloud_storage_managers::CloudResource;
use crate::lib::cloud_storage_managers::CloudServiceManagerTrait;
use crate::lib::local_setup;
use crate::lib::argparser as Arg;
use crate::lib::local_setup::local_setup;
use crate::lib::tools;

/// Function to load the config.toml
/// This function gets called first time in the main. 
/// If global variables should be set, it can be done here. 
fn load_config(path:&Option<String>, access_key: &Option<String>) -> Option<Config>{

    let new_key = Some(access_key.clone().unwrap_or("".to_string()));

    let mut builder = Config::builder()
        .add_source(File::new(&path.clone().unwrap_or("Config.toml".to_string()), FileFormat::Toml).required(true));
    
    
    if let Some(key) = new_key {
        builder = builder.set_override("AZURE_ACCESS_KEY", key).unwrap();
    }
    
    match builder.build(){
        Ok(conf) => {
            Some(conf)
        },
        Err(e) => {
            println!("Could not load config due to: {}
            If it is the first time running -> Setup a local enviroment with \"./aurr-manager run-local-setup\"
            If config file does exists, pass it via an optional argument: \"--config=<path/to/Config.toml>\"  
            ",e.to_string());
            None
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
                    println!("\t{}",i)
                }
            }
        }
    }
}

#[derive(Parser)]
#[command(name = "Aurr")]
#[command(about = "Test Aurr clap cli")]
struct Cli{



    // Cmdlines variables
    #[arg(long, default_value = "./Config.toml")]
    config: Option<String>,

    #[arg(long, long, short, env = "AURR_KEY", help = "Access Key to a Cloud API.")]
    key: Option<String>,

    #[arg(long, default_value = "./log/")]
    log_dir:Option<String>,

    #[command(subcommand)]
    switch: Switch,

    #[arg(long, default_value = "azure")]
    csm:String,

    #[arg(long, default_value = "data/templates/tools.json")]
    tools:String,

    #[arg(long, default_value = "data/templates/case_templates/")]
    case_dir:Option<String>,

    #[arg(long, default_value = "data/templates/task_templates/" )]
    task_dir:Option<String>,

}


#[derive(Subcommand)]
#[derive(Clone)]

enum Switch {

    #[command(about = "Print Version")]
    Version,

    #[command(about = "Run a local setup in the current working directory -> COULD overwrite existing files")]
    LocalSetup,

    #[command(about = "Upload a local resource to a specified cloud location")]
    Upload {

        #[command(subcommand)]
        local_path:LocalResource,
        
        #[arg(help = "Cloud_Path_Like String to target", default_value = "upload", long, short)]
        remote_path:Option<String>
    },

    #[command(about = "Download a specified cloud resource to a local path/folder")]
    Download {
        #[arg(help = "Cloud_Path_Like String")]
        remote_path:String,
        
        #[arg(help = "Local_DirPath_Like String")]
        download_dir:String
    },

    #[command(about = "\"Cloudify\" A local resource  -> Returns a URL that gives access to the file with a given timeout")]
    Cloudify{
        
        #[command(subcommand)]
        local_path:LocalResource,

        #[arg(help = "Cloud_Path_Like String to target", default_value = "upload", long, short)]
        remote_path:Option<String>,

        #[arg(help = "Timeout of token validity in Hours", default_value = "6", long,short)]
        timeout:Option<u8>
    },

    #[command(about = "Grant permissions to a target cloud resource", aliases = ["Grant-Access", "ga"])]
    GrantAccess{

        #[arg(help = "Cloud_Path_Like String to target",)]
        remote_path:String,

        #[arg(help = "Permission string (r|rw)", default_value = "r")]
        permission:String,

        #[arg(help = "Timeout of token validity in Hours", default_value = "6")]
        timeout:u8
    },
    
    #[command(about = "Run a sync against a remote cloudborn location. Will download all new elements to the specified local_path")]
    Sync {
       #[arg(help = "Cloud_Path_Like String to target", long, short)]
        remote_path:String,
        
        #[arg(help = "Lokal directory to save results to",long, short)]
        local_path:String,

        #[arg(help = "Timeout in Hours to monitor a cloud resource", default_value = "4", long,short)]
        timeout:i64,

        #[arg(help = "Interval between each check in min", default_value = "5", long, short)]
        interval:i64
    },

    #[command(about = "Run a Case_Template")]
    Run {
        #[command(subcommand)]
        obj:RunObject,

        #[arg(help = "Timout of the token validity", default_value = "12")]
        timeout:u8
    },

    #[command(about = "List infomation about different objects")]
    Ls {
        #[command(subcommand)]
        switch:ListObject,

        #[arg(long, global = true, default_value = "false", long, short)]
        fullinfo:Option<bool>

    }
}

impl Switch{
    fn is_ls(&self) -> bool{
        match self{
            Switch::LocalSetup => true,
            _ => false
        }
    }
}

#[derive(Subcommand)]
#[derive(Clone)]
pub enum ListObject{

    #[command(about = "Cloudborn objects")]
    Cloud {
        #[arg(help = "Pathlike String -> \"path/to/a/cloudresource\"")]
        cloud_string:Option<String>
    },

    #[command(about = "Config Object")]
    Config,

    #[command(about = "Tools Object")]
    Tools {
        #[arg(help = "String. Will return all \"tools\" with the provided string")]
        filter:Option<String>
    },

    #[command(about = "Case Object")]
    Case {
        #[arg(help = "Pathlike String -> \"path/to/case_template\"")]
        path:Option<String>
    },

    #[command(about = "Cloud Service Manager Objects (CSM)")]
    Csm {
        #[arg(help = "Need to implement this stuff")]
        filter:Option<String>
    },

}

#[derive(Subcommand)]
#[derive(Clone)]
pub enum RunObject{

    #[command(about = "To run a case template",
    group = ArgGroup::new("case")
        .args(["path", "name"])
        .required(true)
        .multiple(false))]
    
    Case {
        #[arg(help = "Local_PathLike_String - Path/to/case/template.json", short,long)]
        path:Option<String>,

        #[arg(help = "Local_PathLike_String - Path/to/case/template.json",  short,long)]
        name:Option<String>
    }
}


#[derive(Subcommand)]
#[derive(Clone)]
#[derive(Debug)]
pub enum LocalResource {

    #[command(about = "File Object ( File Path ) ")]
    File{
        #[arg(help = "Local_FilePath_Like_String")]
        path:String
    }, 

    #[command(about = "Tool Object")]
    Tool{
        #[arg(help = "LocalResource - Name_Tool_Object")]
        name:String
    },

    #[command(about = "Content of a folder - This is not implemented atm")]
    Folder
}

impl Cli{

    pub async fn init() -> Result<(), Box<dyn std::error::Error>>{
        let cli = Cli::parse();
        
        Logger::init(cli.log_dir.clone());

        // Loading the config
        let config = load_config(
            &cli.config,
            &cli.key
        );

        // Easy workaround
        if config.is_none() && cli.switch.is_ls(){
            local_setup()?;
            exit(1)
        }

        //loading the aurr_core manager
        let aurr = AurrCore::new(&config.unwrap()).await?;

        match cli.switch.clone(){

            Switch::Version => Ok(Cli::print_version()),

            Switch::Ls { switch, fullinfo} => cli.ls( aurr, switch, fullinfo.unwrap()).await,

            Switch::LocalSetup => Ok(local_setup().unwrap()),

            Switch::Upload {local_path, remote_path} => cli.upload_local_resource(&aurr, local_path, &remote_path.unwrap()).await,
            
            Switch::Download { remote_path,download_dir } => cli.download_cloud_resource(&aurr, &remote_path, &download_dir).await,
 
            Switch::Sync { remote_path, local_path ,timeout, interval} => cli.sync(&aurr, &remote_path, &local_path, timeout, interval).await,

            Switch::Cloudify {local_path, remote_path, timeout} => cli.cloudify_local_resource(&aurr, local_path, &remote_path.unwrap(), timeout.unwrap()).await,

            Switch::Run { obj , timeout} => cli.run(&aurr, obj, timeout).await,

            Switch::GrantAccess { remote_path, permission ,timeout} => cli.grant_permissions(&aurr, &remote_path, &permission, timeout).await,

        }

    }

    /// Function to load the config.toml
    /// This function gets called first time in the main. 
    /// If global variables should be set, it can be done here. 
    fn load_config(path:Option<String>, access_key: Option<String>) -> Config{

        let mut builder = Config::builder()
            .add_source(File::new(&path.unwrap_or("Config.toml".to_string()), FileFormat::Toml).required(true));
        
        
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

    fn get_tool(&self, local_resource:LocalResource) -> Option<Tool>{

        match local_resource {

            LocalResource::File { path } => {

                
                match Tool::new_from_path(&path){
                        Ok(t) => Some(t.clone()),
                        Err(_) => None
                }
            },
            LocalResource::Tool { name } => {

                //loading all the tools
                let tools = self.load_tools().unwrap();
                
                tools.get(&name).cloned()
            },
            LocalResource::Folder => None
        }
    }

    /// Function to print a config on a nice format. (Not very nice tho :())
    fn print_config(config:&Option<Config>){

        let mut v:Vec<String> = Vec::new();
        for e in config.as_ref().unwrap().cache.clone().to_string().trim_matches('{').trim_matches('}').split(","){
            v.push(e.to_string());
        }

        v.sort();
        println!("{:#?}",v);
    }

    /// 
    /// Function to print the version of the software
    /// 
    fn print_version(){

        println!(
"
+---------------------------------------------------------------+
|                    &   &%   &&                                |
|                    && &&  & && && &                           |
|                && &///&|& ()|/ @, && &                        |
|                &//(/&/&||/& /_/)_&/_& && &&                   |
|            &() &///&|()|/&// '% // () &  &                    |
|            &_&_&&_& |& |&&/&__//_/_& && &&                    |
|            &&   && & &| &| /|| & % ()& /&& &                  |
|        ()&_---////&//|&&-&&--%///-()~                         |
|            &&     |||||///                                    |
|                        ||||                                   |
|                        |||/                                   |
|                        ||||/                                  |
|                        |||||||                                |
|                    /||||||||||||//                            |
|     -=-~, -=-~ //-^-//|| ,||-=-~ //_//-~  .-^ , -=-~  .-^     |  
| ///-()~///-() {}  -~ //-~. //-~-~ . |
| || ,||-=-~ // {}//-//-~. //-~-~~  |
|  -~  .-^- ~- ~{} //-^-///-~.      |
|-~, -=-~ //-^-///-~. //-~-~/|| ,-~, //-~.// -~-~-=-~ //-^-// , |
+---------------------------------------------------------------+    
|    An Yggdrasil soil project                                  |
|     Version 1.0                                               |
|     By: Jonas Sørensen                                        |
+---------------------------------------------------------------+
", "▄████▄ ██  ██ █████▄  █████▄".dark_green(),"██▄▄██ ██  ██ ██▄▄██▄ ██▄▄██▄ ".dark_green(), "██  ██ ▀████▀ ██   ██ ██   ██ ".dark_cyan())

    }

    /// Function to load the tools
    /// Default path is ./data/template/tools.json -> Can be changed via the argument --tools
    fn load_tools(&self) -> Result<HashMap<String,Tool>, Box<dyn std::error::Error>>{
        let tools = Tool::load_from_json(&self.tools).expect("Could not load tools - Check if tools file exist!");
        Ok(tools)
    }
    
    /// Function to load a set of tools
    fn print_tools(&self,filter: Option<String>, fullinfo:bool) -> Result<(), Box<dyn std::error::Error>>{

        let tools = self.load_tools().unwrap();

        let f = match filter{
            None => "".to_string(),
            Some(s) => s
        };

        for tool in tools.values(){

            let print = tool.list_tool(fullinfo);

            if print.contains(&f){
                println!("{}",print)
            }

        }

        Ok(())
    }

    /// Function to list a case
    fn list_case(&self, path:&Option<String>) -> Result<(), Box<dyn std::error::Error>>{

        match path{
            Some(_path) => {

                match CaseTemplate::load_from_json(&_path){
                    Err(e) => Err(format!("Could not load CaseTemplate due to: {}",e.to_string()).into()),
                    Ok(ct) => {
                        let s = ct.ls_case();
                        println!("<{}>{}",_path.clone().green(),s);
                        Ok(())
                    }
                }
           },

            None => match std::fs::read_dir(self.case_dir.clone().unwrap()){

                Ok(entry) => {
                    for e in entry.flatten(){
                        let apath = e.path().to_string_lossy().to_string();
                        let case = CaseTemplate::load_from_json(&apath)?;
                        println!("<{}>{}",apath.green(),case.ls_case());
                    };

                    Ok(())

                },
                Err(e) => Err(format!("Could not read case directory due to: {}",e.to_string()).into())
            }
        }


        
    }

    async fn list_csm(&self, aurr:&AurrCore, filter:&Option<String>) -> Result<(), Box<dyn std::error::Error>>{

        println!("{}",aurr.list_managers().await?);
        Ok(())
    }

    /// Function to pass all the information to list in the cloud down to the aurr-core!
    async fn list_containers(&self, aurr:&AurrCore, cloud_string:Option<String>) -> Result<(), Box<dyn std::error::Error>>{

        let r = match cloud_string.clone(){
            None => aurr.get_mgmr().list_containers().await,
            Some(path) => aurr.get_mgmr().list_blobs_container(&path).await
        };

        match r {
            Ok(val) => {

                let s = match cloud_string{
                    None => format!("<{}>",aurr.get_mgmr().get_name().green()),
                    Some(ss) => format!("<{}> <{}>",aurr.get_mgmr().get_name().green(),ss.green())
                };

                println!("<{}> {}",aurr.get_mgmr().get_type().green(),s);
                PrintResults::Vec(val).print(None)},
            Err(e) => return Err(format!("Could not list container due to: {}",e.to_string()).into())
        }

        Ok(())
    }

    async fn ls(&self, aurr:AurrCore, switch:ListObject, fullinfo:bool) -> Result<(), Box<dyn std::error::Error>>{

        match switch{

            ListObject::Config => Cli::print_config(&Some(aurr.config)),

            ListObject::Tools { filter } => self.print_tools(filter,fullinfo)?,

            ListObject::Case { path } => self.list_case(&path)?,

            ListObject::Cloud { cloud_string } => self.list_containers(&aurr, cloud_string).await?,

            ListObject::Csm { filter } => self.list_csm(&aurr, &filter).await?,

            _ => return Err("The provided ls option is not supported!".into())

        }


        Ok(())
    }

    /// Function to upload a local resource 
    async fn upload_local_resource(&self, aurr:&AurrCore, local:LocalResource, remote:&str)  -> Result<(), Box<dyn std::error::Error>>{

        let atool:Tool = match local {

            LocalResource::File { path } => {

                let mut s = String::new();

                println!("Are you sure you want to upload the following:\n{}", path.clone().red());

                print!("Answer(yes/no): ");
                io::stdout().flush().unwrap();
                std::io::stdin().read_line(&mut s).unwrap();

                if s.eq("yes"){
                    match Tool::new_from_path(&path){
                        Ok(t) => t.clone(),
                        Err(e) => return Err(format!("Could not toolify path: {}",e.to_string()).into())
                    }

                }else{
                    return Err("Upload aborted".into())
                }

            },

            LocalResource::Tool { name } => {

                //loading all the tools
                let tools = self.load_tools().unwrap();
                //Extracting the tool
                let t = match tools.get(&name){
                    Some(a) => a,
                    None => return Err("Provided tool name does not exist in the tool index".into())
                };
                t.clone()
            },
            LocalResource::Folder => todo!()

        };

        match aurr.upload_tool(atool.clone(), Some(remote)).await{
                    Ok(cr) => info!("Uploaded: <{}> to <{}> <{}> <{}>", atool.name,aurr.get_mgmr().get_name(), aurr.get_mgmr().get_type(), cr.get_info().unwrap()),
                    Err(e) => {
                        error!("Could not upload tool due to {}",e.to_string());
                        return Err(e)
                    }
                }


        Ok(())
    }

    /// A function to pass whatever you want to download down to the download function in the aurrcore
    async fn download_cloud_resource(&self, aurr:&AurrCore, remote:&str, local:&str) -> Result<(), Box<dyn std::error::Error>>{

        aurr.download_cloud_resource(remote, local).await?;

        Ok(())
    }

    async fn cloudify_local_resource(&self, aurr:&AurrCore, local:LocalResource, remote:&str, timeout:u8) -> Result<(), Box<dyn std::error::Error>>{
        
        let tool = match self.get_tool(local.clone()){
            Some(t) => t,
            None => return Err(format!("The provided tool: {:?} is not a valid path or in the tool index",local).into())
        };

        let url = tool.cloudify(aurr.get_mgmr(), &remote.to_string(), timeout).await?;

        println!("{}",url);



        Ok(())
    }

    /// A wrapper function to pass the sync arguments down to the manager.
    async fn sync(&self, aurr:&AurrCore, remote:&str, local:&str, timeout:i64, check_interval:i64) -> Result<(), Box<dyn std::error::Error>>{

        let r = CloudResource::from_path(remote, &aurr.get_mgmr().get_type())?;
        aurr.get_mgmr().pull_sync(r, local, timeout, check_interval).await
    }

    /// 
    /// Function to run a predefined automatic task. 
    /// 
    async fn run(&self, aurr:&AurrCore, run_object:RunObject, timeout:u8) -> Result<(), Box<dyn std::error::Error>>{

        match run_object{
            RunObject::Case { path, name } => {

                let mut tools = self.load_tools()?;

                let case = match path{
                    Some(p) => CaseTemplate::load_from_json(&p)?,
                    None => {
                        match name{
                            Some(n) => CaseTemplate::load_from_path_name(&n, &aurr.config.get::<String>("DEFAULT_CASE_DIR").unwrap())?,
                            None => return Err("Error - We should not be in this situation. Error in code".into())
                        }
                    }
                };

                match aurr.tools_push_execute(&mut tools, case.clone(), &aurr.config, timeout).await{
                    Ok(s) => {
                        info!("Run the following oneliner <Timeout UTC+{}> on the target system:",timeout );
                        println!("\t<{}>",s.blue());
                        Ok(())
                    },
                    Err(e) => Err(format!("Could not run case: {} due to: {}",case.name, e.to_string()).into())
                }
            }
        }
    }

    /// 
    /// Function to pass variables and grant permissions to a given target
    /// 
    async fn grant_permissions(&self, aurr:&AurrCore, remote:&str, perm:&str, timeout:u8) -> Result<(), Box<dyn std::error::Error>>{

        let cr = CloudResource::from_path(remote, &aurr.get_mgmr().get_type())?;

        match perm {

            "r" => {
                let s = aurr.get_mgmr().grant_read_access(cr, timeout).await?;
                info!("Resource can be downloaded <Timeout: UTC+{}>, via :",timeout);
                println!("\t<{}>",s.blue())
            },

            "rw" | "wr" => {
                let s = aurr.generate_sas_upload_token(cr, timeout).await?;
                info!("Resource can be written too <Timeout: UTC+{}>, via :",timeout);
                println!("\t<{}>",s.blue())
            },

            _ => return Err("The supported permission string is not supported!".into())
        }




        Ok(())
    }

}


#[tokio::main]
async fn main() {
    Cli::init().await.expect("You did something wrong :(");

}