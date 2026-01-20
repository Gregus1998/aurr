use crate::lib::{
    aurr_core::{LocalResource,GetName},
    azure::{AzureCloudResource, AzureStorageMgmt}, 
    tools::Tool};

use config::Config;
use futures::future::ok;
use serde::de::DeserializeOwned;
use std::{
    collections::HashMap,
    fs
};


///
/// Enum to store and structure all the different types of cloud resources
///     -> Guess this can become handy if we add support or change to another cloud provider in the future
/// 
pub enum CloudResource {
    AZURE(AzureCloudResource)   
}





///
/// Enum to store all possible cloud storage managers. 
/// Currently only azure is supported, but this aims to make it easy to replace a given provided
///     -> All CloudServiceManagers should have a set of features/traits. 
///         - upload a local resource
///         - grant_access_to_cloud_resource
/// 
pub enum CloudServiceManager{
    Azure(AzureStorageMgmt)
}

pub trait CloudServiceManagerTrait {
    async fn upload(&self, resource:LocalResource, some_cloud_storage_path:&str) -> Result<(), Box<dyn std::error::Error>>;
    async fn grant_access(&self, cloud_resource:CloudResource) -> Result<(), Box<dyn std::error::Error>>;
}


///This implement should be moved to azure.rs
impl CloudServiceManagerTrait for AzureStorageMgmt {

    async fn upload(&self, resource:LocalResource, some_cloud_storage_path:&str) -> Result<(), Box<dyn std::error::Error>> {
        self.upload_resource(&resource, &resource.get_name(),some_cloud_storage_path, true).await
    }

    async fn grant_access(&self, cloud_resource:CloudResource) -> Result<(), Box<dyn std::error::Error>>{
        //self.gen_resource_token(cloud_resource, t_resource, perm);
        todo!("Do something here ");
        Ok(())
    }
}

impl CloudServiceManagerTrait for CloudServiceManager{

    async fn upload(&self, resource:LocalResource, some_cloud_storage_path:&str) -> Result<(), Box<dyn std::error::Error>> {
        match self{
            CloudServiceManager::Azure(asm) => asm.upload_resource(&resource, &resource.get_name(),some_cloud_storage_path, true).await
        }
    }

    async fn grant_access(&self, cloud_resource:CloudResource) -> Result<(), Box<dyn std::error::Error>> {
        
        match self {
            CloudServiceManager::Azure(asm) => Ok(())
            
        }
    }
}

impl CloudServiceManager {

    //Function to convert self as Option<&AzureStorageMgmt>
    pub fn as_azure(&self) -> Option<&AzureStorageMgmt>{
        match &self{
            CloudServiceManager::Azure(a) => Some(a),
            _ => None
        }
    }

    ///Function to convert self as Option<&mut AzureStorageMgmt>
    pub fn as_mut_azure(&mut self) -> Option<&mut AzureStorageMgmt>{
        match self{
            CloudServiceManager::Azure(a) => Some(a),
            _ => None
        }
    }
}
