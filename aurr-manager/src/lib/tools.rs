use crate::{impl_has_name, 
    lib::aurr_core::{
        HasName,
        load_json_hashmap
    }};
use azure_core::error;
use config::{Config, Value};
use serde::de::DeserializeOwned;

//Module to handle the setup of all tools. 
use std::collections::HashMap;

#[derive(serde::Deserialize, Debug, Clone)]
pub struct ToolConfig{
    config:HashMap<String,String>
}

impl ToolConfig {

    pub fn new() -> ToolConfig{
        ToolConfig { config: HashMap::new() }
    }

    ///Function to add config parameters based on another config.
    pub fn search_other_config(&mut self, other_config:Config, search:&str){
        for i in other_config.clone().cache.try_deserialize::<HashMap<String,Value>>().unwrap().keys().filter(|s| s.contains(search) ){
            let val = other_config.get::<String>(i).unwrap();
            self.add(i.to_string(), val);
        }
    }
    
    pub fn add(&mut self, key:String, val:String){
        self.config.insert(key, val);

    }
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct Tool{
    name:String,
    author: String,
    localpath:String,
    call:HashMap<String,Vec<String>>
}

impl_has_name!(Tool);

impl Tool {

    pub fn load_from_json<T>(path:&str) -> Result<HashMap<String, T>,Box<dyn std::error::Error>>
    where
        T: DeserializeOwned + HasName + Clone,
    {
        load_json_hashmap(path)
    }

    /// Function to get a specific command line for a tool based on a call_key. 
    /// To define call keys, edit the specific template. 
    pub fn get_cmdline(&self, call_key:&str, tool_config:ToolConfig) -> Option<String>{
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
}