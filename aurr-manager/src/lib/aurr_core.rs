use crate::lib::{
    azure::AzureStorageMgmt, cloud_storage_managers::{CloudResource, CloudServiceManager, CloudServiceManagerTrait}, template::CaseTemplate, tools::Tool};

use config::Config;
use futures::future::ok;
use serde::de::DeserializeOwned;
use tracing::error;
use std::{
    str::FromStr,
    collections::HashMap,
    fs::{self,DirEntry}, hash::Hash
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
    Tool(Tool)
}

#[derive(Debug)]
enum Shell {
    Powershell,
    Bash,
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

        vec![format!("cd C:\\"),format!("rm -r {}",wd)]
    }
}


///
/// A function to provide a default download option for a target shell
/// It is important that if a download option is provided, this needs to be installed via "mandatory_steps"
/// 
pub fn get_download_template(shell:&str) -> Option<String>{
    match shell.to_lowercase().as_str() {
        "powershell" => Some("POWERSHELL_DOWNLOAD_URL".to_string()),
        "bash" => Some("BASH_DOWNLOAD_URL".to_string()),
        &_ => {
            error!("Provided shell option is not supported: {}\n To add support do the following: 1. Add a variable in config. \n2. Update the match statement in aurr_core::get_download_template",shell);
            None
        }
        
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
    pub fn new(shell:Shell, cmdlines:Vec<String>) -> ShellParser{
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
            },
            _ => {
                error!("Defined shell: {:?} is not configurated yet. Do add support, edit aurr_core::ShellParser",self.shell);
                None
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
            LocalResource::Tool(t) => t.name.clone()
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

/// 
/// The Aurr Core structure. 
/// cloudservicemanager: CloudServiceManager 
///     -> Some interface to interact with the cloud
/// config: Config 
///     -> Some random condig file that needs to include all you need to interact with the cloud and the desired tools. 
/// 
pub struct AurrCore<'a> {
    cloudservicemanager: CloudServiceManager,
    config:&'a Config
}

impl AurrCore <'_> {

    pub fn new_from_sas(config:&Config) -> AurrCore{

        AurrCore{
            cloudservicemanager : CloudServiceManager::Azure({
                AzureStorageMgmt::new(
                    config.get::<String>("AZURE_ACCOUNT_STORAGE_NAME").unwrap().as_str(),
                     config.get::<String>("AZURE_SAS_TOKEN").unwrap().as_str()
                    ).unwrap()}
            ),
            config: config
        }
    }

    pub fn new_from_ac(config:&Config) -> AurrCore{

        AurrCore{
            cloudservicemanager : CloudServiceManager::Azure({
                AzureStorageMgmt::from_access_key(
                    config.get::<String>("AZURE_ACCOUNT_STORAGE_NAME").unwrap().as_str(),
                     config.get::<String>("AZURE_ACCESS_KEY").unwrap().as_str()
                    ).unwrap()}
            ),
            config: config
        }
    }



    pub fn mgr_as_azure(&self) -> Option<&AzureStorageMgmt>{
        match &self.cloudservicemanager{
            CloudServiceManager::Azure(s) => Some(s),
            _ => None
        }
    }

    /// Function to expose the self.cloudservecemanager
    pub fn get_mgmr(&self) -> &CloudServiceManager{
        &self.cloudservicemanager
    }


    pub async fn upload_tool(&self, tool:Tool) -> Result<CloudResource, Box<dyn std::error::Error>>{
        self.cloudservicemanager.upload(LocalResource::Tool(tool), "tools").await
    }

    ///
    /// Function to cloudify a vector of tools. 
    /// This will push a set of tools to the cloud and return a set of downloadable urls.
    /// 
    pub async fn cloudify_tools_vec(&self, tools:&mut Vec<Tool>, config:Config) -> Result<HashMap<String,String>, Box<dyn std::error::Error>>{
        let mut urls:HashMap<String,String> = HashMap::new();

        for tool in tools.iter_mut(){
            let s = tool.cloudify(&self.get_mgmr(), &config).await.unwrap();
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
            let s = tool.cloudify(&self.get_mgmr(), &config).await.unwrap();
            urls.insert(name.clone(), s);
        }
        Ok(urls)
    }

    ///
    /// Function to take a set of tools 
    ///     1. Cloudify them
    ///     2. Create and cloudify a download and execute scirpt
    ///     3. Return a URL for this script
    /// 
    pub async fn tmp_name(&self, tools:&mut HashMap<String,Tool>,case_template:CaseTemplate, config:&Config) -> Result<String, Box<dyn std::error::Error>>{


        //Fetching and converting the OS for the given task
        let os = OperatingSystem::from_str(&case_template.task_template.os).unwrap();

        //Initiating a vector with the setup steps.
        let mut cmds:Vec<String> = os.get_setup(&config);

        //Filtering so I only use the tools present in the task_template
        let mut filtered_tools = case_template.task_template.get_relevant_tools(tools);

        for (name,tool) in filtered_tools.iter_mut(){

            //Cloudify and push the tool on the cmds vector
            let url = tool.cloudify(&self.get_mgmr(), &config).await.unwrap();
            
            let down_template = config.get::<String>(&get_download_template(&case_template.task_template.shell).unwrap()).unwrap();
            
            //Since this is running in a linux enviroment, then path of the local file will be used to save the file to a given system.
            let remote_download_filename = tool.localpath.split("/").last().unwrap();

            cmds.push(down_template
                .replace("<URL>", &url)
                .replace("<REMOTE_TOOL_FILE_NAME>", remote_download_filename));


        }


        //extending the cmdline with the execution of the actual tools. 
        cmds.extend(case_template.build_task_list(filtered_tools.clone(), &config));

        cmds.extend(os.cleanup(&config));

        let sp = ShellParser::new(Shell::from_str(&case_template.task_template.shell).unwrap(), cmds);
        println!("{:?}",sp.get_oneliner());
        
        Ok("sa".to_string())
    }


}


