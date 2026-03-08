//Import from local crate
use crate::{error, impl_has_name, info, lib::{aurr_core::{
        AurrCore, HasName, Shell, load_manyjson_hashmap_by_name, print_btmap, print_map}, 
    cloud_storage_managers::{CloudServiceManager,CloudServiceManagerTrait}}
};
use regex::Regex;
use azure_storage_blobs::prelude::RetentionPolicy;
use clap::builder::Str;
use config::{Config, Value};
use serde::de::DeserializeOwned;
use std::{collections::BTreeMap, env, fmt::{Debug, Display, format}, fs::{self, File}, path::Path, str::FromStr};
use crate::CloudResource;
//Module to handle the setup of all tools. 
use std::collections::HashMap;
use colored::{self, Colorize};

#[derive(serde::Deserialize, Debug, Clone, PartialEq, Eq, Hash,Copy, PartialOrd,Ord)]
pub enum AurrObjectTaskList{
    GenEnvVar,      // Used to generate enviroment variables 
    GenConfVar,     // Used to add new entries to a config
    Build,          // Used to run a Build script (Should be using some sort of enviroment variables)
    Alter,          // Used to alter the content of a specific object. Can use Env or Conf
    ReqObj,         // Used to signal what other objects that is needed for a parent object. (Dont create reqursive circle of doom)
    AtTarget,       // Other commands to run at a target prior to the parent call option
}

impl Display for AurrObjectTaskList{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self{
            AurrObjectTaskList::Alter => f.write_str("Alter"),
            AurrObjectTaskList::AtTarget => f.write_str("AtTarget"),
            AurrObjectTaskList::Build => f.write_str("Build"),
            AurrObjectTaskList::GenConfVar => f.write_str("GenConfVar"),
            AurrObjectTaskList::GenEnvVar => f.write_str("GenEnvVar"),
            AurrObjectTaskList::ReqObj => f.write_str("ReqObj")
        }
    }
        
}

impl FromStr for AurrObjectTaskList {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "genenvvar" => Ok(AurrObjectTaskList::GenEnvVar),
            "genconfvar" => Ok(AurrObjectTaskList::GenConfVar),
            "build" => Ok(AurrObjectTaskList::Build),
            "alter" => Ok(AurrObjectTaskList::Alter),
            "reqobj" => Ok(AurrObjectTaskList::ReqObj),
            "attarget" => Ok(AurrObjectTaskList::AtTarget),
            _ => Err(()),
        }
    }
}

#[derive(serde::Deserialize, Debug, Clone)]
pub enum AurrObjectType{
    Tool,
    File
}

impl Display for AurrObjectType{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self{
            AurrObjectType::File => f.write_str("File"),
            AurrObjectType::Tool => f.write_str("Tool")
        }
    }
}

///
/// A structure to handle individual configs for a set of tools. 
/// 
#[derive(serde::Deserialize, Debug, Clone)]
pub struct AurrObjectConfig{
    pub config:HashMap<String,String>
}

impl AurrObjectConfig{
    pub fn new_empty() -> AurrObjectConfig{
        AurrObjectConfig { config: HashMap::new() }
    }

    pub fn add(&mut self, key:String, val:String){
        self.config.insert(key, val);
    }

    ///Function to add config parameters based on another config.
    pub fn search_other_config(&mut self, other_config:&Config, search:&str){
        for i in other_config.clone().cache.try_deserialize::<HashMap<String,Value>>().unwrap().keys().filter(|s| s.contains(search) ){
            let val = other_config.get::<String>(i).unwrap();
            self.add(i.to_string(), val);
        }
    }

    pub fn from_config_by_tags(config:&Config, tags:Vec<&str>) -> Option<AurrObjectConfig>{
        let mut t = AurrObjectConfig::new_empty();
        for tag in tags.iter(){
            t.search_other_config(&config, tag);
        }
        Some(t)
    }

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
                Err(_) => None
            }}
                
        }
    }


}

/// A structure to embed all objects to be used in case templates, task templates and or other stuff. 

#[derive(serde::Deserialize, Debug, Clone)]
pub struct AurrObject{
    pub name:String,
    object_type:AurrObjectType,
    author:String,
    pub config_tag:String,
    metadata:String,
    target_shell:Shell,
    pub local_path: String,
    task_list:BTreeMap<AurrObjectTaskList,Option<Vec<String>>>,
    call: BTreeMap<String,Vec<String>>
}

impl_has_name!(AurrObject);

impl AurrObject {

    /// Function to create a new AurrObject from a file
    /// Will be used when you just want to pass a local file somewhere
    pub fn new_from_path(path:&str, target_shell:Shell) -> Result<AurrObject,Box<dyn std::error::Error>>{
        let apath = Path::new(path);
        
        if !apath.exists(){
            return Err("Could not create AurrObject - File does not exist! >:(".into())
        }

        //Collecting the basename as the name of the tool
        let name = apath.file_name().unwrap().to_string_lossy().to_string();

        Ok(AurrObject { 
            name,
            object_type: AurrObjectType::File,
            author: "".to_string(),
            config_tag: "".to_string(),
            target_shell: target_shell,
            metadata: "".to_string(),
            local_path: path.to_string(),
             task_list: BTreeMap::new(),
              call: BTreeMap::new()
            })
    } 
    
    ///Function to list the object with status. This is used in the LS switch
    pub fn ls(&self, full:bool) -> String{
        match full{
            true => {
                format!(
"
        {:<15} {}
        {:<15} {}
        {:<15} {}
        {:<15} {}
        {:<15} {}
        {:<15} {}
        {:<15} {}
        {:<15} {}
        {:<15} {}
        {:<15} {}
",
"name:", self.name,
"object_type:", self.object_type,
"author:", self.author,
"config_tag:", self.config_tag,
"target_shell:", self.target_shell,
"metadata:", self.metadata,
"local_path:", self.local_path,
"exists:", match std::fs::File::open(self.local_path.clone()) {
    Ok(_) => "[TRUE]".green(),
    Err(_) => "[FALSE]".red(),
},
"task_list:" , {
    let mut s = String::new();

    for (i,v) in self.task_list.iter(){
        s.push_str(&format!(
"
            {:<50}: {}", i, v.iter().map(|s| format!("{:?}",s)).collect::<Vec<_>>().join(", ")
        ));
    }
    s
},
"call:", {
    let mut s = String::new();

    for (i,v) in self.call.iter(){
        s.push_str(&format!(
"
            {} {}", i, v.iter().map(|s| format!("{:?}",s)).collect::<Vec<_>>().join(", ")
        ));
    }
    s
}


)
            },

            false => {
                format!(
"
        {:<15} {}
        {:<15} {}
        {:<15} {}
        {:<15} {}
        {:<15} {}
        {:<15} {}
        {:<15} {}
        {:<15} {}
",
"name:", self.name,
"object_type:", self.object_type,
"author:", self.author,
"config_tag:", self.config_tag,
"target_shell:", self.target_shell,
"metadata:", self.metadata,
"local_path:", self.local_path,
"exists:", match std::fs::File::open(self.local_path.clone()) {
    Ok(_) => "[TRUE]".green(),
    Err(_) => "[FALSE]".red(),
},)
                }
        }
    }

    ///Function to load a hashmap of objects from a jsonfile
    pub fn load_from_json<T>(path:&str) -> Result<HashMap<String, T>,Box<dyn std::error::Error>>
    where
        T: DeserializeOwned + HasName + Clone,
    {
        load_manyjson_hashmap_by_name(path)
    }

    pub fn load_from_conf_env<T>(config:HashMap<String,String>) -> Result<HashMap<String, T>,Box<dyn std::error::Error>>
    where
        T: DeserializeOwned + HasName + Clone,
    {
        let locations = match env::var("LOCAL_AURROBJECT_INDEX"){
            Ok(v) => v,
            Err(_) => {
                match config.get::<String>(&"LOCAL_AURROBJECT_INDEX".to_string()){
                    Some(v) => v.to_string(),
                    None => return Err("Could not find LOCAL_AURROBJECT_INDEX".into()) 
                }
            }
        };

        AurrObject::load_from_json(&locations)
    }


    /// This function will bould the target cmdline for a specific tool 
    /// Variables in the cmdline can be passed eighter via env or 
    pub fn get_cmdline(&self, call_key:&str, config:&AurrObjectConfig) -> Option<String>{

        match self.call.get(call_key){
            Some(entry) => {

                let re = Regex::new(r"\$([A-Za-z_][A-Za-z0-9_]*)").unwrap();

                let mut s = String::new();

                for i in entry.iter(){

                    let mut placeholder = i.clone();

                    //Vector of all 
                    let var_to_replace:Vec<String> = re.captures_iter(i).map(|c|c[1].to_string()).collect();

                    for v in var_to_replace.iter(){

                        let new_value = match env::var(v){
                            Ok(e) => e,
                            Err(_) => match config.get::<String>(v){
                                Some(e) => e,
                                None => format!("<CLOUD NOT FIND ENTRY: \"{}\" in env or provided config>", v)
                            }
                        };

                        placeholder = placeholder.replace(&format!("${}",v), &new_value);
                    }

                    s.push_str(&placeholder);
                    s.push_str(" ");
                }

                Some(s)

            },
            None => None
        }
    }

    /// Function to cloudify a given object. 
    pub async fn cloudify(&self, cloud_manager:&CloudServiceManager, cloud_location:&str, timeout:u8) -> Result<String, Box<dyn std::error::Error>>{

        let cr = match cloud_manager.upload(super::aurr_core::LocalResource::AurrObject(self.clone()), cloud_location).await{
            Ok(t) => {
                info!("Uploaded object {} to {}:{:?}", self.name, cloud_manager.get_type(), t.get_info());
                t
            },
            Err(e) => {
                error!("Could not upload {} due to: {}", self.name, e.to_string());
                return  Err("Upload error".into());
            }
        };

        let url = cloud_manager.grant_read_access(cr, timeout).await?;
    
    Ok(url)
    }

    pub async fn process_all_tasks_cloudify(&mut self, cloud_manager:&CloudServiceManager, config:&mut AurrObjectConfig) -> Result<Vec<String>,Box<dyn std::error::Error>>{

        let mut results_processed_tasks = self.process_all_tasks(cloud_manager, config).await?.unwrap();

        let upload_location = match config.get::<String>("CLOUD_DEFAULT_UPLOAD_LOCATION"){
            Some(s) => s,
            None => {
                match env::var(&"CLOUD_DEFAULT_UPLOAD_LOCATION".to_string()){
                    Ok(s) => s,
                    Err(_) => uuid::Uuid::new_v4().to_string()
                }
            }
        };

        let timeout = match config.get::<u8>("CLOUD_TOKEN_READ_TIMEOUT"){
            Some(s) => s,
            None => 12
        };

        let url = self.cloudify(cloud_manager, &upload_location, timeout).await?;

        let download_cmd = self.target_shell.get_download_template()?
            .replace("<URL>", &url)
            .replace("<REMOTE_TOOL_FILE_NAME>", &self.local_path.replace("\\", "/").split("/").last().unwrap());

        results_processed_tasks.push(download_cmd);

        match self.process_task(AurrObjectTaskList::AtTarget, cloud_manager, config).await{
            Ok(s) => {
                match s {
                    Some(ss) => {
                        results_processed_tasks.extend(ss);
                    },
                    None => ()
                }
            },
            Err(e) => return Err(e)
        };

        Ok(results_processed_tasks)
    }

    pub fn get_task_value(&self, task:AurrObjectTaskList) -> Option<Vec<String>>{

        match self.task_list.get(&task){
            Some(s) => s.clone(),
            None => None
        }

    }

    /// A function to process over all the inner tasks and return a vector of 
    pub async fn process_all_tasks(&mut self, cloud_manager:&CloudServiceManager, config:&mut AurrObjectConfig) -> Result<Option<Vec<String>>, Box<dyn std::error::Error>>{
        let mut super_r:Vec<String> = Vec::new();
        
        for (i,_) in self.task_list.iter(){

            if i.eq(&AurrObjectTaskList::AtTarget){
                continue;
            }
            
            let mut a = match self.process_task(*i, cloud_manager, config).await{

                Err(e) => return Err(e),
                Ok(s) => {
                    match s{
                        Some(value) => value,
                        None => {
                            continue;
                        }
                    }
                }
            };

            super_r.append(&mut a);

        }

        Ok(Some(super_r))
    }

    /// A function to produce a set of variables for a given tool. 
    /// This can interact with the CloudManager 
    pub async fn gen_var(&self, var:&str, cloud_manager:&CloudServiceManager, config:&AurrObjectConfig) -> Result<String, Box<dyn std::error::Error>>{

        let upload_loc = match env::var("CLOUD_DEFAULT_UPLOAD_LOCATION"){
            Ok(s) => s,
            Err(_) => {
                match config.get::<String>("CLOUD_DEFAULT_UPLOAD_LOCATION"){
                    None => return Err("Missing valid upload location in env or config".into()),
                    Some(t) => t
                }
            }
        };

        let cr = match CloudResource::from_path(&upload_loc, &cloud_manager.get_type()){
                Ok(a) => a,
                Err(_) => return Err("Need to implment CloudResource::from_path for the specified CloudResourceManager".into())
        };
        
        let token_timeout = config.get::<u8>("CLOUD_TOKEN_UPLOAD_TIMEOUT").unwrap();

        let s:String = if var.ends_with("UPLOAD_TOKEN"){
            
            cloud_manager.grant_upload_token(cr, token_timeout).await?

        }else if var.ends_with("UPLOAD_URL") {
 
            cloud_manager.grant_upload_url(cr, token_timeout).await?

        }else {
            return Err("The provided variable generation is not supported yet".into())
        };

        Ok(s)

    }

    pub async fn task_gen_env_var(&self, var:&Vec<String>, cloud_manager:&CloudServiceManager, config:&AurrObjectConfig) -> Result<(), Box<dyn std::error::Error>>{

        for v in var.iter(){

            let var_value = self.gen_var(v, cloud_manager, config).await?;

            unsafe {
                env::set_var(v, var_value);
            }
        }

        info!("Completed subtask_gen_env_var for {}",self.name);

        Ok(())

    }

    pub async fn task_gen_conf_var(&self, var:&Vec<String>, cloud_manager:&CloudServiceManager, config:&mut AurrObjectConfig) -> Result<(), Box<dyn std::error::Error>>{
        for v in var.iter(){

            let value = self.gen_var(&v, cloud_manager, config).await?;
            config.add(v.to_string(), value.to_string());
        }
        info!("Completed subtask_gen_conf_var for {}",self.name);

        Ok(())

    }

    pub async fn task_build(&self,var:&Vec<String>,config:&mut AurrObjectConfig) -> Result<(),Box<dyn std::error::Error>>{
        
        for v in var.iter(){

            let output = std::process::Command::new(v)
                .output()?;
            
            let stdout = String::from_utf8_lossy(&output.stdout);

            for l in stdout.lines(){
                let re = Regex::new(r#"\$([A-Za-z_][A-Za-z0-9_]*)="([^"]*)""#).unwrap();

                if let Some(caps) = re.captures(l){
                    let k = &caps[1];
                    let val = &caps[2];
                    config.add(k.to_string(), val.to_string());
                }
            }


        }

        info!("Completed subtask_build for {}",self.name);

        Ok(())
    }

    pub async fn task_alter(&mut self, var:&Vec<String>,config:&mut AurrObjectConfig) ->  Result<(),Box<dyn std::error::Error>>{
        
        todo!("Implement Task_alter for AurrObject under new tools");
        Ok(())
    }

    /// Function to pass on whatever object that is required at the target system. 
    /// It will check if it is a file or an entry in any of the AURRObject files.
    pub async fn task_req_obj(&self, var:&Vec<String>, cloud_manager:&CloudServiceManager, config:&mut AurrObjectConfig) -> Result<Vec<String>, Box<dyn std::error::Error>>{
        let mut r:Vec<String> = Vec::new();

        let mut req_obj :Vec<AurrObject>= Vec::new();

        // Finding the object eighter from a predefined list or filepath
        for v in var.iter(){

            let object = match fs::File::open(v){
                Ok(s) => AurrObject::new_from_path(v,self.target_shell.clone())?,
                Err(_) => {

                    let objcts:HashMap<String,AurrObject> = AurrObject::load_from_conf_env(config.config.clone())?;
                    
                    match objcts.get(v){

                        Some(vv) => vv.clone(),
                        None => return Err("Passed invalid reqobject to some object".into())
                    }
                }
            };

            req_obj.push(object);
        }
        
        // For each of the objects. Run the process all tasks.
        for o in req_obj.iter_mut(){

            let mut object_config = config.clone();

            let mut some_r:Vec<String> = Box::pin(async move {o.process_all_tasks_cloudify(cloud_manager, &mut object_config).await
            }).await?;

            r.append(&mut some_r);

        }

        info!("Completed subtask_reqobj for {}",self.name);

        // Cloudify the selfe object. 
        Ok(r)
    } 

    /// A wrapper function to process a target task.
    pub async fn process_task(&self, task:AurrObjectTaskList, cloud_manager:&CloudServiceManager, config:&mut AurrObjectConfig) -> Result<Option<Vec<String>>, Box<dyn std::error::Error>>{

        match task{

            // Some insane error handling here :()
            AurrObjectTaskList::GenEnvVar => {
               match self.task_list.get(&AurrObjectTaskList::GenEnvVar){
                    Some(n) => {
                        match n {
                            Some(v) => {
                                self.task_gen_env_var(v, cloud_manager, config).await?
                            },
                            None => ()
                        }
                    },
                    None => ()
                }
                ()
            },

            AurrObjectTaskList::GenConfVar => match self.get_task_value(AurrObjectTaskList::GenConfVar){
                Some(s) => {
                    self.task_gen_conf_var(&s, cloud_manager, config).await?;
                }
                None => ()
            },

            AurrObjectTaskList::Alter => match self.get_task_value(AurrObjectTaskList::Alter) {
                Some(val) => {
                    todo!("Fix Alter function! or remove it!")
                },
                None => ()
            },

            //Build or run. This function calls a script
            AurrObjectTaskList::Build => match self.get_task_value(AurrObjectTaskList::Build) {
                Some(val) => self.task_build(&val, config).await?,
                None => ()
            },

            // This will return a vector of downloadable urls. 
            AurrObjectTaskList::ReqObj => match self.get_task_value(AurrObjectTaskList::ReqObj){
                Some(val) => { 
                    return Ok(Some(self.task_req_obj(&val, cloud_manager, config).await?));
                },

                None => ()  
            },

            // This will return a vector of cmdline that needs to be run at the target before the main call
            AurrObjectTaskList::AtTarget => match self.get_task_value(AurrObjectTaskList::AtTarget){
                Some(val) => {
                    return Ok(Some(val))
                },
                None => ()
            },
            
            _ => () 
        }

        Ok(None)
    }

}

