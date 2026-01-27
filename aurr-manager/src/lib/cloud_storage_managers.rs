use crate::lib::{
    aurr_core::{LocalResource,GetName},
    azure::{AzureCloudResource, AzureStorageMgmt}, 
    tools::Tool};

use azure_core::cloud;
use config::Config;
use crossterm::style::Stylize;
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
    pub fn get_type(&self) -> String{
        match self{
            CloudResource::AZURE(_) => "AZURE".to_string()
        }
    }

    /// 
    /// Function to get some random information for a cloud resource
    /// Used to display metadata when uploading
    /// 
    pub fn get_info(&self) -> Option<String>{
        match self{
            CloudResource::AZURE(acr) => format!("{}/{}", 
            match acr.get_container_name(){
                Some(cn) => cn,
                None => "N/A"
            },
            acr.get_name()).into(),
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
    async fn grant_upload_token(&self, cloud_resource:CloudResource, timeout:u8) -> Result<String, Box<dyn std::error::Error>>;
    fn get_name(&self) -> String;
    fn get_type(&self) -> String;
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

    async fn grant_upload_token(&self, cloud_resource:CloudResource, timeout:u8) -> Result<String, Box<dyn std::error::Error>> {
        match self{
            CloudServiceManager::Azure(asm) => asm.grant_upload_token(cloud_resource,timeout).await
        }
    }

    fn get_name(&self) -> String {
        match self{
            CloudServiceManager::Azure(acm) => acm.get_name()
        }
    }

    fn get_type(&self) -> String {

        match self{
            CloudServiceManager::Azure(acm) => acm.get_type()
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

    ///
    /// A function to grant a upload token to a type of azure cloud resource
    /// Currently this only supports generating container SAS-upload tokens. 
    /// The permissions for the upload need to be tuned. 
    /// 
    async fn grant_upload_token(&self, cloud_resource:CloudResource, timeout:u8) -> Result<String, Box<dyn std::error::Error>> {
        match cloud_resource.as_azure(){
            
            Some(acr) => {
                
                match acr{
                    AzureCloudResource::Container(con) => {
                        self.gen_upload_container_sas(con, timeout).await
                    }
                    _ => return Err(format!("Granting upload token for the provided AzureCloudresource is not supported.\n
                    To fix this, add a implementation in impl 'CloudServiceManagerTrait for AzureStorageMgmt::grant_upload_token()'").into())
                }
            },
            None => return Err(format!("Provided mismatch cloud resource {} for AZURE cloud",cloud_resource.get_type()).into())

        }
    }

    ///
    /// Trait function to get the name of azure storage account
    /// Used to display info runtime
    /// 
    fn get_name(&self) -> String {
        self.account_name.clone()
    }

    ///
    /// Trait function to return "AZURE-CLOUD"
    /// Used runtime to display information
    /// 
    fn get_type(&self) -> String {
        "AZURE-CLOUD".to_string()
    }
}
