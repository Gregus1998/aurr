use crate::lib::{
    azure::{AzureStorageMgmt},
    cloud_storage_managers::{CloudServiceManager, CloudServiceManagerTrait}, 
    tools::Tool};

use config::Config;
use futures::future::ok;
use serde::de::DeserializeOwned;
use std::{
    collections::HashMap,
    fs::{self,DirEntry}
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

pub fn load_json_vec<T>(path: &str) -> Result<Vec<T>, Box<dyn std::error::Error>>
    where
        T: DeserializeOwned,
    {
        let data = fs::read_to_string(path)?;
        let values: Vec<T> = serde_json::from_str(&data)?;
        Ok(values)
    }

pub fn load_json_hashmap<T>(path:&str) -> Result<HashMap<String, T>,Box<dyn std::error::Error>>
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
                    config.get::<String>("ACCOUNT_STORAGE_NAME").unwrap().as_str(),
                     config.get::<String>("SAS_TOKEN").unwrap().as_str()
                    ).unwrap()}
            ),
            config: config
        }
    }

    pub fn new_from_ac(config:&Config) -> AurrCore{

        AurrCore{
            cloudservicemanager : CloudServiceManager::Azure({
                AzureStorageMgmt::from_access_key(
                    config.get::<String>("ACCOUNT_STORAGE_NAME").unwrap().as_str(),
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


    pub async fn upload_tool(&self, tool:Tool) -> Result<(), Box<dyn std::error::Error>>{

        self.cloudservicemanager.upload(LocalResource::Tool(tool), "tools").await
        

    }
}


