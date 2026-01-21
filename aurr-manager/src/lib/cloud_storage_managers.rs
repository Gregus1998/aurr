use crate::lib::{
    aurr_core::{LocalResource,GetName},
    azure::{AzureCloudResource, AzureStorageMgmt}, 
    tools::Tool};

use config::Config;
use futures::future::ok;
use serde::de::DeserializeOwned;
use tracing::error;
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
impl CloudResource {

    pub fn as_azure(&self) -> Option<&AzureCloudResource>{
        match self {
            CloudResource::AZURE(azure) => Some(azure),
            _ => None
            
        }
    }
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
    async fn upload(&self, resource:LocalResource, some_cloud_storage_path:&str) -> Result<CloudResource, Box<dyn std::error::Error>>;
    async fn grant_read_access(&self, cloud_resource:CloudResource, timeout:u8) -> Result<String, Box<dyn std::error::Error>>;
}


///
/// Implementation of CloudServiceManager
/// Mainly this will be a wrapper and forking all the different calls for different cloud managers. 
///     -> Looks unnecessary since there are only support of azure atm
/// 
impl CloudServiceManagerTrait for CloudServiceManager{

    async fn upload(&self, resource:LocalResource, some_cloud_storage_path:&str) -> Result<CloudResource, Box<dyn std::error::Error>> {
        match self{
            CloudServiceManager::Azure(asm) => Ok(
                CloudResource::AZURE(
                    asm.upload_resource(&resource, &resource.get_name(),some_cloud_storage_path, true).await.unwrap()))
        }
    }

    async fn grant_read_access(&self, cloud_resource:CloudResource, timeout:u8) -> Result<String, Box<dyn std::error::Error>> {
        match self{
            CloudServiceManager::Azure(asm) => asm.grant_read_access(cloud_resource,timeout).await
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


///This implement should be moved to azure.rs
impl CloudServiceManagerTrait for AzureStorageMgmt {

    async fn upload(&self, resource:LocalResource, some_cloud_storage_path:&str) -> Result<CloudResource, Box<dyn std::error::Error>> {
        Ok(
            CloudResource::AZURE(
                self.upload_resource(&resource, &resource.get_name(),some_cloud_storage_path, true).await.unwrap()
        ))
    }

    async fn grant_read_access(&self, cloud_resource:CloudResource, timeout:u8) -> Result<String, Box<dyn std::error::Error>>{
        
        match cloud_resource{
            CloudResource::AZURE(acr) => {
                self.get_blob_download_url(None, acr, timeout).await
            },
            _ => {
                error!("Passed wrong cloud resource type to azure");
                return Err("Wrong cloud resource type".into());
            }
        }
    }
}
