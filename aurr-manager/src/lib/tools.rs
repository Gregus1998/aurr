use crate::{error, impl_has_name, lib::{aurr_core::{
        print_map,
        HasName,
        load_json_hashmap,
        load_manyjson_hashmap_by_name}, 
    cloud_storage_managers::{CloudServiceManager,CloudServiceManagerTrait}}
};

use config::{Config, Value};
use serde::de::DeserializeOwned;
use tracing::info;
use std::{fmt::{Debug, Display}, str::FromStr};

//Module to handle the setup of all tools. 
use std::{char::ToLowercase, clone, collections::HashMap};
use colored::{self, Colorize};


///
/// Placeholder for all the types of mandatory steps to be executed prior to the use of a tool. 
/// Will support atleast: 
///     Custom compile
///     Cloudify
///     Delete from cloud
/// 
#[derive(serde::Deserialize, Debug, Clone, PartialEq, Eq, Hash,Copy)]
pub enum MandatorySteps{
    Generate,
    Target,
    Compile
}

impl Display for MandatorySteps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self{
            MandatorySteps::Compile => f.write_str("compile"),
            MandatorySteps::Generate  => f.write_str("generate"),
            MandatorySteps::Target => f.write_str("target")
        }
    }
    
}

impl FromStr for MandatorySteps {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "generate" => Ok(MandatorySteps::Generate),
            "target" => Ok(MandatorySteps::Target),
            _ => Err(()),
        }
    }
}

impl MandatorySteps{
    pub fn as_key(&self) -> String{
        match &self {
            MandatorySteps::Generate => "Generate".to_string(),
            MandatorySteps::Target => "Target".to_string(),
            MandatorySteps::Compile => "Compile".to_string()
        }
    }
}



#[derive(serde::Deserialize, Debug, Clone)]
pub struct ToolConfig{
    pub config:HashMap<String,String>
}

impl ToolConfig {

    pub fn new() -> ToolConfig{
        ToolConfig { config: HashMap::new() }
    }

    ///Function to add config parameters based on another config.
    pub fn search_other_config(&mut self, other_config:&Config, search:&str){
        for i in other_config.clone().cache.try_deserialize::<HashMap<String,Value>>().unwrap().keys().filter(|s| s.contains(search) ){
            let val = other_config.get::<String>(i).unwrap();
            self.add(i.to_string(), val);
        }
    }
    
    pub fn add(&mut self, key:String, val:String){
        self.config.insert(key, val);
    }

    pub fn from_config_by_tag(config:Config, tag:&str) -> Option<ToolConfig>{
        let mut t = ToolConfig::new();
        t.search_other_config(&config, tag);
        Some(t)
    }
    
    pub fn from_config_by_tags(config:&Config, tags:Vec<&str>) -> Option<ToolConfig>{
        let mut t = ToolConfig::new();
        for tag in tags.iter(){
            t.search_other_config(&config, tag);
        }
        Some(t)
    }

    ///
    /// Function to edit a entry in the tools config. 
    /// This takes a entry, if it exist, clear the buffer and push on a new value.
    /// 
    pub fn edit_entry(&mut self, key:String, new_val:String) -> Result<(), Box<dyn std::error::Error>>{

        match self.config.get_mut(&key){
            None => Err(format!("Key does not exist: {}",key).into()),
            Some(val) => {
                val.clear();
                val.push_str(&new_val);
                Ok(())
            }
        }


    }

    pub fn get<T>(&self ,key:&str) -> Option<T>
    where
    T: FromStr
    {
        match self.config.get(key){
            None => None,
            Some(val) => {match T::from_str(val){
                Ok(res) => Some(res),
                Err(e) => None
            }}
                
        }

        
    }
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct Tool{
    pub name:String,
    pub author: String,
    pub task:String,
    pub localpath:String,
    pub config_tag:String,
    pub mandatory_steps:Option<HashMap<MandatorySteps,Vec<String>>>,
    pub call:HashMap<String,Vec<String>>
}

impl_has_name!(Tool);

impl Tool {

    pub fn list_tool(&self, full:bool){
        if full {
            println!("
            name: {},
            author {},
            localpath: {},
            config_tag: {},
            exists: {},
            mandatory_steps: {}
            call_options: {}
            ",self.name,self.author,self.localpath.clone(),self.config_tag,
                match std::fs::File::open(self.localpath.clone()){
                    Ok(_) => "[TRUE]".green(),
                    Err(_) => "[FALSE]".red()
                },
                print_map(&self.mandatory_steps.clone().unwrap()),
                print_map(&self.call)
            )

        }else {
            println!("
            name: {},
            author {},
            localpath: {},
            config_tag: {},
            Exists: {}
            ",self.name,self.author,self.localpath.clone(),self.config_tag,
                match std::fs::File::open(self.localpath.clone()){
                    Ok(_) => "[TRUE]".green(),
                    Err(_) => "[FALSE]".red()
                }
            )
        };

    }

    pub fn load_from_json<T>(path:&str) -> Result<HashMap<String, T>,Box<dyn std::error::Error>>
    where
        T: DeserializeOwned + HasName + Clone,
    {
        load_manyjson_hashmap_by_name(path)
    }

    /// 
    /// Function to get a specific command line for a tool based on a call_key. 
    /// To define call keys, edit the specific template.
    /// Replaces each and every instanse of entries in the "tool_config" forthe final cmdlines:
    /// Config:
    ///     "EKSAMPLE_CONFIG" => "VARIABLE"
    /// "some cmdline that contains EKSAMPLE_CONFIG" => "some cmdline that contains VARIABLE" 
    /// 
    pub fn get_cmdline(&self, call_key:&str, tool_config:&ToolConfig) -> Option<String>{
        match self.call.get(call_key) {
            Some(entry) =>{
                let mut cmdline = entry.join(" ");

                for (i,v) in tool_config.config.iter(){
                    cmdline = cmdline.replace(i, v);
                }
                Some(cmdline)
            },
            None => None
        }
    }

    ///
    /// Function to return the mandatory steps to be done for a given mandatory step type. 
    /// If there exists a set of mandatory steps for a type of mandatory steps it will return a vector:vec<String> of the
    /// Steps. This can be changed to vec<T> to add support for more generic steps.
    ///        
    pub fn get_mandatory_step_by_type(&self, mandatory_step_type:MandatorySteps) -> Option<Vec<String>>{

        match self.mandatory_steps.as_ref().unwrap().get(&mandatory_step_type){
            Some(steps ) => Some(steps.clone()),
            None => None
        }
    }

    pub fn process_mandatory_step(&self, mandatory_step_type:MandatorySteps, steps:Vec<String>, config:Option<&ToolConfig>) -> Option<Vec<String>>{

        let mut cloned_steps = steps.clone();

        match mandatory_step_type{
            MandatorySteps::Generate => {
                None
            }

            MandatorySteps::Compile => {
                None
            }

            MandatorySteps::Target => {

                for step in cloned_steps.iter_mut(){
                    match config {
                        Some(c) => {
                            for (i,v) in c.config.iter(){
                                *step = step.replace(i, v);
                            }
                        },
                        None => continue
                    }
                };
                Some(cloned_steps)
            }
        }



    }

    ///
    /// A wrapper function for get_mandatory_step_by_type and process_mandatory_step
    ///Takes self, MandatorySteps and a tool config
    ///
    /// -> a vector of steps to de in that mandatory step context
    ///
    ///  -> If 
    pub fn produce_mandator_steps_by_type(&self, mandatory_step_type:MandatorySteps, config:&ToolConfig) -> Option<Vec<String>>{
        match self.get_mandatory_step_by_type(mandatory_step_type.clone()) {
            Some(ms) => self.process_mandatory_step(mandatory_step_type, ms, Some(config)),
            None => None
        }
    }

    /// 
    /// A function to "cloudify a given tool"
    /// Pass a cloud manager to the tool and it will pipe the tool up in cloud and generate a URL
    /// Only support for AZURE at the moment
    /// 
    pub async fn cloudify(&self, cloud_manager:&CloudServiceManager, config:&Config) -> Result<String, Box<dyn std::error::Error>>{
        
        let cp = config.get::<String>("AZURE_TOOLS_CONTAINER_NAME").unwrap();

        //This returns a cloud resource
        let cr = match cloud_manager.upload(super::aurr_core::LocalResource::Tool(self.clone()), &cp).await{
            
            Ok(t) => {
                info!("Uploaded tool: {} to {}",t.as_azure().unwrap().get_name(), cloud_manager.get_type());
                t
            },
            
            Err(e) => {
                error!("Could not upload tool: {} due to: {}",self.name, e);
                return Err(format!("Could not upload tool: {} due to: {}",self.name, e).into());
            }

        };

        let url = cloud_manager.grant_read_access(cr, config.get::<u8>("CLOUD_TOKEN_READ_TIMEOUT").unwrap()).await.unwrap();

        Ok(url)
    }

}