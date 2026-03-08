use crate::lib::{
    azure::AzureStorageMgmt, cloud_storage_managers::{CarrierTypes, CloudResource, CloudServiceManager, CloudServiceManagerTrait}, new_tools::{AurrObject, AurrObjectConfig}, template::CaseTemplate};

use clap::builder::Str;
use colored::{self, Colorize};
use config::Config;
use serde::de::DeserializeOwned;
use tracing::{error, info};
use std::{
    error::Error,
    collections::{BTreeMap, HashMap}, fmt::{Debug, Display}, fs::{self,DirEntry}, hash::Hash, process::exit, str::FromStr
};

///
/// Module for the AURR structure and core features. 
/// 
/// The goal of the builder to to take a config and based on that config produce a onliner that can be passed on to download a staging script.

///Trait "HashName" -> just to make sure that a given struct has the 
pub trait HasName {
    fn name(&self) -> &str;
}

///enum to structure local resources
/// 

pub enum LocalResource {
    Text(String),
    Entry(DirEntry),
    AurrObject(AurrObject)
}

impl LocalResource {
    pub fn get_base_name(&self) -> String{

        match self{
            LocalResource::AurrObject(ao) => ao.local_path.replace("\\", "/").split("/").last().unwrap().to_string(),
            _ => self.get_name()
        }
    }
}


#[derive(Debug)]
#[derive(Clone)]
#[derive(serde::Deserialize)]
pub enum Shell {
    Powershell,
    Bash,
}

impl Display for Shell{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {

        match self{
            Shell::Bash => f.write_str("Bash"),
            Shell::Powershell => f.write_str("Powershell")
        }
        
    }
}

impl FromStr for Shell {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "powershell" => Ok(Shell::Powershell),
            "bash" => Ok(Shell::Bash),
            _ => Err(()),
        }
    }
}

impl Shell{
    ///
    /// Function to get the download template for a specific shell.
    /// 
    pub fn get_download_template(&self) -> Result<String,Box<dyn std::error::Error>>{
        match self{
            Shell::Bash => Ok(String::from("curl -L \"<URL>\" -o ./<REMOTE_TOOL_FILE_NAME>")),
            Shell::Powershell => Ok(String::from("iwr -useb '<URL>' -Outfile <REMOTE_TOOL_FILE_NAME>"))
        }
    }
}

pub enum OperatingSystem{
    Windows,
    Linux
}

impl FromStr for OperatingSystem {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.to_lowercase().contains("windows"){
            Ok(OperatingSystem::Windows)
        }else if s.to_lowercase().contains("linux") {
            Ok(OperatingSystem::Linux)
        }else {
            Err(())
        }
    }
}

impl OperatingSystem{

    ///
    /// For each task, there needs to be done some setup so that we can start working. 
    /// Steps: 
    ///     1. Create working directory
    ///     2. Move context to working directory
    /// 
    pub fn get_setup(&self,config:&Config) -> Vec<String>{
        
        let wd = match self{
            OperatingSystem::Linux => {
                config.get::<String>("LINUX_WORKDIR_REMOTE").unwrap()
            },
            OperatingSystem::Windows => {
                config.get::<String>("WINDOW_WORKDIR_REMOTE").unwrap()
            }
        };
        vec![format!("mkdir {}",wd), format!("cd {}",wd)]
    }

    /// 
    /// Whenever a given task is completed there needs to be done some cleanup
    /// Steps: 
    ///     1. Delete working directory 
    pub fn cleanup(&self, config:&Config) -> Vec<String>{

        let wd = match self{
            OperatingSystem::Linux => {
                config.get::<String>("LINUX_WORKDIR_REMOTE").unwrap()
            },
            OperatingSystem::Windows => {
                config.get::<String>("WINDOW_WORKDIR_REMOTE").unwrap()
            }
        };

        vec![format!("cd ../../"),format!("rm -r {}",wd)]
    }
}


///
/// A struct to handle all the interactions between a vector of standalone cmdlines to a shell oneliner.
/// 
pub struct ShellParser{
    shell:Shell,
    cmdlines:Vec<String>
}

impl ShellParser {
    fn new(shell:Shell, cmdlines:Vec<String>) -> ShellParser{
        ShellParser { shell: shell, cmdlines : cmdlines }
    }

    pub fn get_oneliner(&self) -> Option<String>{

        match self.shell{
            Shell::Bash => {
                let mut oneliner = String::new();
                for cmdlines in self.cmdlines.iter(){
                    oneliner.push_str(cmdlines);
                    oneliner.push_str(";");
                }
                Some(oneliner)
            },
            Shell::Powershell => {
                let mut oneliner = String::new();
                for cmdlines in self.cmdlines.iter(){
                    oneliner.push_str(cmdlines);
                    oneliner.push_str(";");
                }
                Some(oneliner)
            }
        }
    }
}

pub trait GetName{
    fn get_name(&self) -> String;
}

impl GetName for LocalResource{
    fn get_name(&self) -> String {
        match self{
            LocalResource::Entry(s) => s.file_name().into_string().unwrap(),
            LocalResource::Text(s) => s.split_terminator("/")
                                                .collect::<Vec<&str>>()
                                                .last()
                                                .unwrap()
                                                .to_string(),
            LocalResource::AurrObject(ao) => ao.name.to_string()
        }
    }
}


//Macro to implement HasName for structs
#[macro_export]macro_rules! impl_has_name {
    ($t:ty) => {
        impl HasName for $t {
            fn name(&self) -> &str {
                &self.name
            }
        }
    };
}

/// 
/// Function to load a Json file as a single object
/// 
pub fn load_json<T>(path: &str) -> Result<T, Box<dyn std::error::Error>>
    where
        T: DeserializeOwned,
    {
        let data = fs::read_to_string(path)?;
        let value: T = serde_json::from_str(&data)?;
        Ok(value)
    }

/// 
/// Function to load a Json file as a vector
/// 
pub fn load_json_vec<T>(path: &str) -> Result<Vec<T>, Box<dyn std::error::Error>>
    where
        T: DeserializeOwned,
    {
        let data = fs::read_to_string(path)?;
        let values: Vec<T> = serde_json::from_str(&data)?;
        Ok(values)
    }

/// 
/// Function to load a jsonfile as a hashmap
///  
pub fn load_json_hashmap<T>(path:&str) -> Result<HashMap<String,T>, Box<dyn std::error::Error>>
    where 
    T : DeserializeOwned + Hash + Eq,
    {
        let data = fs::read_to_string(path)?;
        let values: HashMap<String,T> = serde_json::from_str(&data)?;
        Ok(values)
    }

pub fn load_json_btreemap<T>(path:&str) -> Result<BTreeMap<String,T>, Box<dyn std::error::Error>>
    where 
    T : DeserializeOwned + Hash + Eq,
    {
        let data = fs::read_to_string(path)?;
        let values: BTreeMap<String,T> = serde_json::from_str(&data)?;
        Ok(values)
    }

/// 
/// Function to load a jason dict as a hashmap where the "name" field
/// is the key of the map. 
/// 
pub fn load_manyjson_hashmap_by_name<T>(path:&str) -> Result<HashMap<String, T>,Box<dyn std::error::Error>>
    where
        T: DeserializeOwned + HasName + Clone,
        {
            let data = fs::read_to_string(path)?;
            let values: Vec<T> = serde_json::from_str(&data)?;
            
            let mut map:HashMap<String, T> = HashMap::new();

            for i in values.iter(){
                let a = i.name();
                map.insert(a.to_string(), i.clone());
            }
            
            Ok(map)

        }

pub fn print_map<K,T>(map:&HashMap<K,T>) -> String
where
K: Debug + Display,
T: Debug
{
    let mut s = String::new();

    for (key,val) in map.iter(){
        s.push_str("\n\t\t");
        s.push_str(format!(" {} => {:?}",key,val).replace("[\"\"]", "None").as_str());
    };

    s
}


///
/// Function to print out a btreemap. 
/// Does introduce some strange event when BTreemap includes a betreemap.
/// Dont know if it is possible to create a generic approach. 
/// 
pub fn print_btmap<K,T>(map:&BTreeMap<K,T>) -> String
where
K: Debug + Display + Clone,
T: Debug
{
    let mut s = String::new();

    for (key,val) in map.iter(){

        s.push_str(format!(
"{:<5}:  {:?}",key, val)
            .as_str());
    };

    s
}

/// 
/// The Aurr Core structure. 
/// cloudservicemanager: CloudServiceManager 
///     -> Some interface to interact with the cloud
/// config: Config 
///     -> Some random condig file that needs to include all you need to interact with the cloud and the desired tools. 
/// 
pub struct AurrCore {
    pub config:Config,
    cloudservicemanagers: HashMap<String,CloudServiceManager>,
    csm:String
}


impl AurrCore{

    ///
    /// New functin to create an empty shell
    /// Need to add different types of carriers/cloudmanagers
    /// 
    pub async fn new(config:&Config) -> Result<AurrCore, Box<dyn Error>>{
        let mut aurr = AurrCore {config: config.clone(), cloudservicemanagers: HashMap::new(), csm: String::new()};
        
        //Adding the default azure CloudManager
        aurr.add_cloud_manager(CarrierTypes::Azure, None).unwrap();

        Ok(aurr)
    }

    /// 
    /// A function to add/Registrer a new cloud service manager. 
    /// If a opt_config is provided this will be used to spawn the manager, 
    /// If a config is not passed, the global config will be used.
    ///  
    pub fn add_cloud_manager(&mut self, cloud_type:CarrierTypes ,opt_config:Option<&Config>) -> Result<(), Box<dyn std::error::Error>>{

        // Finding the correct config, if a opt_config is not passed, the program should try to use the currently running config
        let conf = match opt_config{
            Some(c) => c,
            None => &self.config
        };

        let mgmr = CloudServiceManager::new(cloud_type, &conf)?;
        //Creating a key for the specific manager. 
        let key = format!("{}_{}", mgmr.get_type(), mgmr.get_name());
        match self.cloudservicemanagers.insert(key.clone(), mgmr){
            None => {
                self.set_csm(&key).unwrap();
                info!("CloudServiceManger: <{}> set as the running Cloud Service Manager (CSM)",key);
            },

            Some(_) => {
                info!("CloudServiceManager <{}> was overwritten by a new Cloud Service Manager",key);
            }
        }

        Ok(())
    }

    ///
    /// A function to list all the managers present in the CloudserviceManger variable. 
    /// Should list manager name and run a function to collect some basic info. 
    /// 
    pub async fn list_managers(&self) -> Result<String,Box<dyn Error>>{

        let mut s = String::new();

        for (key,val) in self.cloudservicemanagers.iter(){

            let reachable = match val.test_connection().await{
                Ok(bool) => {

                    match bool{
                        true => "REACHABLE".green(),
                        false => "UNREACHABLE".red()
                    }

                },
                Err(e) => "ERROR".yellow()

            };

            let ss = format!("{} -> {} - [{:}]", key, val.get_info(), reachable);
            s.push_str(&ss);
        };

        Ok(s)
    }

    /// 
    /// A function to set the taraget csm
    /// If the CSM does not exist in the CMS hashmap, an error is printed and returned.
    ///   
    pub fn set_csm(&mut self, target_csm:&str) -> Result<(), Box<dyn Error>>{
        
        match self.cloudservicemanagers.get(target_csm){
            None => return {
                error!("The provided target cloud manager does not exist in the cloud service manager map. Consider to import this!");
                Err("The provided target cloud manager does not exist in the cloud service manager map. Consider to import this!".into())}
                ,
            Some(_) => ()
        };
        
        self.csm = target_csm.to_string();
        Ok(())
    }

    ///
    /// A wrapper function to get the inner manager as azure
    /// 
    pub fn mgr_as_azure(&self) -> Option<&AzureStorageMgmt>{
        match &self.get_mgmr(){
            CloudServiceManager::Azure(s) => Some(s)
        }
    }

    /// Function to expose the self.cloudservecemanager
    pub fn get_mgmr(&self) -> &CloudServiceManager{
        &self.cloudservicemanagers.get(&self.csm).unwrap()
    }

    ///
    /// Function to download a cloud resource 
    /// 
    pub async fn download_cloud_resource(&self, cloud_resource_path:&str, download_dir:&str) -> Result<(), Box<dyn std::error::Error>>{

        let cr = match self.get_mgmr(){
            CloudServiceManager::Azure(_) => CloudResource::Azure(super::azure::AzureCloudResource::Text(cloud_resource_path.to_string()))
        };

        self.get_mgmr().download(cr, download_dir).await?;

        Ok(())
    }
/*
    ///
    /// Function to cloudify a vector of tools. 
    /// This will push a set of tools to the cloud and return a set of downloadable urls.
    /// 
    pub async fn cloudify_tools_vec(&self, tools:&mut Vec<Tool>, config:Config) -> Result<HashMap<String,String>, Box<dyn std::error::Error>>{
        let mut urls:HashMap<String,String> = HashMap::new();

        for tool in tools.iter_mut(){
            let s = tool.cloudify(&self.get_mgmr(), todo!(), 4).await.unwrap();
            urls.insert(tool.name.clone(), s);
        }
        Ok(urls)
    }

    ///
    /// Function to cloudify a hashmap<string,tool> of tools. 
    /// This will push a set of tools to the cloud and return a set of downloadable urls.
    /// 
    pub async fn cloudify_tools_hashmap(&self, tools:&mut HashMap<String,Tool>, config:Config) -> Result<HashMap<String,String>, Box<dyn std::error::Error>>{
        let mut urls:HashMap<String,String> = HashMap::new();

        for (name,tool) in tools.iter_mut(){
            let s = tool.cloudify(&self.get_mgmr(), todo!(), 4).await.unwrap();
            urls.insert(name.clone(), s);
        }
        Ok(urls)
    }*/

    /// New function to handle NEW_TOOLS
    /// 
    pub async fn run_case(&self, tools:&mut HashMap<String,AurrObject>,case_template:CaseTemplate, config:&Config, timeout:u8) -> Result<String, Box<dyn std::error::Error>>{


        info!("Running \"run_case\" for case template: {}", case_template.name);

        //Fetching and converting the OS for the given task
        let os = OperatingSystem::from_str(&case_template.task_template.os).unwrap();

        //Initiating a vector with the setup steps.
        let mut cmds:Vec<String> = os.get_setup(&config);


        // Here we should run all the processing, cloudification and generating the cmdline to run the tool. 

        //Fetching the Cloud root storage for a specific case. 
        let case_container = case_template.name().to_string().to_ascii_lowercase();

        //Checking if Name is valid
        if !case_container.chars().all(|c| c.is_ascii_alphabetic()){
                    error!("CaseTemplate.Name needs to be ascii_alphanumeric -> Azure Cloud does not support container names outside of this bound.");
                    return Err("Fix case template name plz".into())
                }

        // For all the different tasks in the task template
        for (_task,ss) in case_template.task_template.tasks().iter(){

            let sub_tasks = match ss.as_ref(){
                Some(t) => t,
                None => continue
            };

            //Fetching the tool
            for (tool_name, call_options) in sub_tasks.iter(){

                let tool = match tools.get_mut(tool_name){
                    Some(t) => t,
                    None => return Err(format!("Tool: {} Does not exist in the tool index",tool_name).into())
                };

                //Producing the tools config. 
                let mut tool_config = AurrObjectConfig::from_config_by_tags(&config, vec![&tool.config_tag,"CLOUD","AZURE","LOCAL"]).unwrap();

                // Adding the upload container to the cloud default upload location
                tool_config.add("CLOUD_DEFAULT_UPLOAD_LOCATION".to_string(), case_container.clone());

                let mut a = tool.process_all_tasks_cloudify(self.get_mgmr(), &mut tool_config).await?;

                cmds.append(&mut a);

                // Extending the cmds with the call option for a traget tool.
                for t in call_options.iter(){
                    let new_cmd = tool.get_cmdline(&t, &tool_config);
                    cmds.extend(tool.get_cmdline(&t, &tool_config));
                }
            }
        }

            



        // Adding the cleanup steps for the shell
        cmds.extend(os.cleanup(&config));

        let sp = ShellParser::new(case_template.task_template.shell, cmds);
        
        match sp.get_oneliner(){
            Some(ol) => Ok(ol),
            None => {
                Err("Could not produce oneliner  :(".into())
            }
        }

    }

}


