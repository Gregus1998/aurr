use crate::lib::{
    azure::AzureStorageMgmt, cloud_storage_managers::{CloudResource, CloudServiceManager, CloudServiceManagerTrait, CloudTypes}, template::CaseTemplate, tools::{MandatorySteps, Tool, ToolConfig}};

use azure_core::error;
use azure_storage_blobs::container::Container;
use colored::{self, Colorize};
use config::Config;
use serde::de::DeserializeOwned;
use tracing::{error, info};
use tracing_subscriber::fmt::format;
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

        vec![format!("cd ../../"),format!("rm -r {}",wd)]
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

        s.push_str("\n\t");
        s.push_str(format!("{:<12}:  {:?}",key, val)
            .replace("\\n\\t", "")
            .replace("\\\"", "")
            .replace("[\"\"]", "None")
            .replace("\"", "")
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
    config:Config,
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
        aurr.add_cloud_manager(CloudTypes::Azure, None).unwrap();

        Ok(aurr)
    }

    /// 
    /// A function to add/Registrer a new cloud service manager. 
    /// If a opt_config is provided this will be used to spawn the manager, 
    /// If a config is not passed, the global config will be used.
    ///  
    pub fn add_cloud_manager(&mut self, cloud_type:CloudTypes ,opt_config:Option<&Config>) -> Result<(), Box<dyn std::error::Error>>{

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
                info!("CloudServiceManger: <{}> set as the running git manager!",key);
            },

            Some(_) => {
                info!("CloudServiceManager <{}> was overwritten by a new CloudServiceManager",key);
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
    /// Function to upload a tool to a specific cloud
    /// If a container is passed, the tool will be uploaded to this location
    /// 
    pub async fn upload_tool(&self, tool:Tool, container:Option<&str>) -> Result<CloudResource, Box<dyn std::error::Error>>{

        match container{
            Some(s) => self.get_mgmr().upload(LocalResource::Tool(tool), s).await,
            None => self.get_mgmr().upload(LocalResource::Tool(tool), "tools").await
        }
        
    }


    pub async fn download_cloud_resource(&self, cloud_resource_path:&str, download_dir:&str) -> Result<(), Box<dyn std::error::Error>>{

        let cr = match self.get_mgmr(){
            CloudServiceManager::Azure(_) => CloudResource::Azure(super::azure::AzureCloudResource::Text(cloud_resource_path.to_string()))
        };

        self.get_mgmr().download(cr, download_dir).await?;

        Ok(())
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
    /// Function to take a set of tasks
    ///     1. Cloudify all the relevant tools
    ///     2. Produce a script to do the following:
    ///         a. Setup the enviroment on a remote system
    ///         b. Download the tools from the cloud
    ///         c. Runtime process required steps and additional resources
    ///         d. Execute the tools based on a the provided config
    ///         e. Cleanup 
    ///
    /// 
    pub async fn tools_push_execute(&self, tools:&mut HashMap<String,Tool>,case_template:CaseTemplate, config:&Config) -> Result<String, Box<dyn std::error::Error>>{

        info!("Running <Tool Push Execute> for template: {}", case_template.name);
        //Fetching and converting the OS for the given task
        let os = OperatingSystem::from_str(&case_template.task_template.os).unwrap();

        //Initiating a vector with the setup steps.
        let mut cmds:Vec<String> = os.get_setup(&config);

        //Not a very beutiful solution here, But it works. Another argument to rework everything >:()
        for (_task,ss) in case_template.task_template.tasks().iter(){

            let sub_tasks = match ss.as_ref(){
                Some(t) => t,
                None => continue
            };

            //Fetching the tool
            for tool_name in sub_tasks.keys(){
                let tool = match tools.get(tool_name){
                    Some(t) => t,
                    None => return Err(format!("Tool: {} Does not exist in the tool index",tool_name).into())
                };

                let mut tool_config = ToolConfig::from_config_by_tags(&config, vec![&tool.config_tag,"CLOUD","AZURE"]).unwrap();


                if !case_template.name.chars().all(|c| c.is_ascii_alphabetic()){
                    error!("CaseTemplate.Name needs to be ascii_alphanumeric -> Azure Cloud does not support container names outside of this bound.");
                    return Err("Fix case template name plz".into())
                }

                //Chanign the upload container to a case specific location.AZURE_UPLOAD_CONTAINER_NAME
                tool_config.edit_entry("CLOUD_DEFAULT_UPLOAD_LOCATION".to_string(), case_template.name().to_string().to_ascii_lowercase()).unwrap();
                //tool_config.edit_entry("AZURE_UPLOAD_CONTAINER_NAME".to_string(), case_template.name().to_string().to_ascii_lowercase()).unwrap();
                //tool_config.edit_entry("CLOUD_DEFAULT_UPLOAD_LOCATION".to_string(), "upload".to_string()).unwrap();

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

        for parameter in generation_steps.iter(){

            if parameter.contains("SAS-UPLOAD-TOKEN"){

                //Will check the config for SURGE_UPLOAD
                let cloud_upload_location = format!("{}_SAS-UPLOAD-TOKEN",tool.config_tag);

                let con = match tool_config.get::<String>("CLOUD_DEFAULT_UPLOAD_LOCATION"){
                    Some(s) => s,
                    None => uuid::Uuid::new_v4().to_string()
                };


                match self.generate_sas_upload_token(
                    CloudResource::Azure(crate::lib::azure::AzureCloudResource::Container(Container::new(&con))),
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
                    CloudResource::Azure(crate::lib::azure::AzureCloudResource::Container(Container::new(&con))),
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


