use crate::lib::{
    azure::AzureStorageMgmt, cloud_storage_managers::{CloudResource, CloudServiceManager, CloudServiceManagerTrait}, template::CaseTemplate, tools::{MandatorySteps, Tool, ToolConfig}};

use azure_storage_blobs::container::Container;
use colored::Colorize;
use config::Config;
use serde::de::DeserializeOwned;
use tracing::error;
use std::{
    collections::{BTreeMap, HashMap}, fmt::{Debug, Display}, fs::{self,DirEntry}, hash::Hash, str::FromStr
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
        "powershell" => Some("POWERSHELL_DOWNLOAD_CMD".to_string()),
        "bash" => Some("BASH_DOWNLOAD_CMD".to_string()),
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

pub fn print_btmap<K,T>(map:&BTreeMap<K,T>) -> String
where
K: Debug + Display,
T: Debug + Display
{
    let mut s = String::new();

    for (key,val) in map.iter(){
        s.push_str("\n\t  ");
        s.push_str(format!("{}:{}",key,val).replace("[\"\"]", "None").as_str());
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
    cloudservicemanager: CloudServiceManager,
    config:Config
}

impl AurrCore{

    pub fn new_from_sas(config:&Config) -> AurrCore{

        AurrCore{
            cloudservicemanager : CloudServiceManager::Azure({
                AzureStorageMgmt::new(
                    config.get::<String>("AZURE_ACCOUNT_STORAGE_NAME").unwrap().as_str(),
                     config.get::<String>("AZURE_SAS_TOKEN").unwrap().as_str()
                    ).unwrap()}
            ),
            config: config.clone()
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
            config: config.clone()
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
    ///     2. Produce a script to do the following:
    /// 
    ///         a. Setup the enviroment on a remote system
    ///         b. Download the tools from the cloud
    ///         c. Runtime process required steps and additional resources
    ///         d. Execute the tools based on a the provided config
    ///         e. Cleanup 
    ///
    /// 
    pub async fn tools_push_execute(&self, tools:&mut HashMap<String,Tool>,case_template:CaseTemplate, config:&Config) -> Result<String, Box<dyn std::error::Error>>{


        //Fetching and converting the OS for the given task
        let os = OperatingSystem::from_str(&case_template.task_template.os).unwrap();

        //Initiating a vector with the setup steps.
        let mut cmds:Vec<String> = os.get_setup(&config);

        // Filtering so I only use the tools present in the task_template
        // Very ugly workaround, but the following 3 lines will filter based on tools provided in the task template.
        // Then it will sort all the tools based on task steps. This should have been done in another way.  
        let filtered_tools = case_template.task_template.get_relevant_tools(tools);
        //let mut ft_vec:Vec<(&String,&Tool)> = filtered_tools.iter().collect();
        //ft_vec.sort_by(|(_,a),(_,b)|a.task.cmp(&b.task));

        for (_name,tool) in filtered_tools.iter(){

            //  Getting the tools config by both tool_name and the GENERAL CLOUD CONFIG
            // Almost there where we can just pass the whole config in between, but that would be too simple and easy to understand. 
            // Atleast this supports a new token generation for each execution. 
            //      -> This means it will be easy/possible to track token to different collections. 
            let mut tool_config = ToolConfig::from_config_by_tags(&config, vec![&tool.config_tag,"CLOUD","AZURE"]).unwrap();

            self.generate_entry_toolconfig(&mut tool_config, tool).await.unwrap();

            //Cloudify and push the tool on the cmds vectord
            let url = tool.cloudify(&self.get_mgmr(), &config).await.unwrap();

            let down_template = config.get::<String>(&get_download_template(&case_template.task_template.shell).unwrap()).unwrap();
            
            //Since this is running in a linux enviroment, then path of the local file will be used to save the file to a given system.
            let remote_download_filename = tool.localpath.split("/").last().unwrap();

            cmds.push(down_template
                .replace("<URL>", &url)
                .replace("<REMOTE_TOOL_FILE_NAME>", remote_download_filename));

            cmds.extend(case_template.build_task(tool, &tool_config));
        }

        cmds.extend(os.cleanup(&config));

        let sp = ShellParser::new(Shell::from_str(&case_template.task_template.shell).unwrap(), cmds);
        
        match sp.get_oneliner(){
            Some(ol) => Ok(ol),
            None => {
                Err("Could not produce oneliner  :(".into())
            }
        }
    }


    /// 
    /// A wrapper function around all the different mandatory steps.
    /// 
    ///  
    pub async fn process_mandatory_step(&self, tool:&Tool, config:&mut ToolConfig, ms:MandatorySteps) -> Option<Vec<String>>{

        //If the function is called without any steps -> None is returned
        let mut steps = match tool.get_mandatory_step_by_type(ms){
            Some(s) => s,
            None => return None
        };

        match ms{
            MandatorySteps::Generate => {
                todo!("Add support for mandatory step generate in AurrCore::process_mandatory_steps");
                },
           
           MandatorySteps::Target => {
                for step in steps.iter_mut(){
                        for (i,v) in config.config.iter(){
                                *step = step.replace(i, v);
                            }
                    }
                },
           
            MandatorySteps::Compile => {
                todo!("Add support for mandatory step Compile in AurrCore::process_mandatory_steps");
                }
            }
            

        Some(steps)
    }


    ///
    /// Function to handle all the different processing steps for config variable generation steps. 
    /// This finctuon should take any parameter input and produce a entry in the tool config. 
    /// Parameter format: "<config_tag>_<somevar1>_<somevar2>_<somevar_n>" 
    ///     Example: SURGE_SAS-UPLOAD-TOKEN -> This will produce a enty in the config: 'SURGE_SAS-UPLOAD-TOKEN' -> 'Some generated SAS-token'
    /// 
    /// For each usecase of this there need to be added a support in in this function.
    /// This is solved with a chain of if else statements 
    /// 
    pub async fn generate_entry_toolconfig(&self, tool_config:&mut ToolConfig, tool:&Tool) -> Result<(), Box<dyn std::error::Error>>{

        //Ekstracting the generation steps. If this is empty, the execution will just continue
        let generation_steps = match tool.get_mandatory_step_by_type(MandatorySteps::Generate){
            Some(s) => s,
            None => return Ok(())
        };

        // Adding a tracking for if anything is happening here since rust does not support "match by substring search"
        let mut did_somthing:bool = false;

        for parameter in generation_steps.iter(){

            if parameter.contains("SAS-UPLOAD-TOKEN"){

                //Will check the config for SURGE_UPLOAD
                let cloud_upload_location = format!("{}_SAS-UPLOAD-TOKEN",tool.config_tag);

                let con = match tool_config.get::<String>("CLOUD_DEFAULT_UPLOAD_LOCATION"){
                    Some(s) => s,
                    None => uuid::Uuid::new_v4().to_string()
                };


                match self.generate_sas_upload_token(
                    CloudResource::AZURE(crate::lib::azure::AzureCloudResource::Container(Container::new(&con))),
                     tool_config.get::<u8>("CLOUD_TOKEN_UPLOAD_TIMEOUT").unwrap())
                     .await{
                        Ok(token) => tool_config.add(cloud_upload_location, token),
                        Err(e) => return Err(e)
                     }

            } else if parameter.contains("SAS-UPLOAD-URI"){

                //Defining the entry where the URI will be stored
                let new_config_entry = format!("{}_SAS-UPLOAD-URI",tool.config_tag);

                let con = match tool_config.get::<String>("CLOUD_DEFAULT_UPLOAD_LOCATION"){
                    Some(s) => s,
                    None => uuid::Uuid::new_v4().to_string()
                };

                match self.gen_sas_upload_url(
                    CloudResource::AZURE(crate::lib::azure::AzureCloudResource::Container(Container::new(&con))),
                     tool_config.get::<u8>("CLOUD_TOKEN_UPLOAD_TIMEOUT").unwrap())
                     .await{
                        Ok(token) => tool_config.add(new_config_entry, token),
                        Err(e) => return Err(e)
                     }
            } else {
                return Err("The provided parameter is not supported".into())
            }
        
        }

        Ok(())        
    }

    ///
    /// Wrapper function to generate a token for a cloud resource.
    /// 
    pub async fn generate_sas_upload_token(&self, cloud_resource:CloudResource, timeout:u8) -> Result<String, Box<dyn std::error::Error>>{
        
        match self.get_mgmr().grant_upload_token(cloud_resource, timeout).await{
            Ok(token) => Ok(token),
            Err(e) => {
                let msg = format!("Could not generate token due to: '{}'",e);
                Err(msg.into())}
        }
    }

    /// 
    /// Wrapper function to generate a sas URI
    ///

    pub async fn gen_sas_upload_url(&self, cloud_resource:CloudResource, timeout:u8) -> Result<String, Box<dyn std::error::Error>>{
        self.get_mgmr().grant_upload_url(cloud_resource, timeout).await

    }

}


