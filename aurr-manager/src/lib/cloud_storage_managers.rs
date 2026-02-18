use config::Config;
use tracing::info;

use crate::lib::{
    aurr_core::{LocalResource,GetName},
    azure::{AzureCloudResource, AzureStorageMgmt}
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
            CloudResource::AZURE(azure) => Some(azure)
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
                None => "N/A".to_string()
            },
            acr.get_name()).into()
        }
    }

    pub fn from_path(path:&str, cloud_type:&str) -> Result<CloudResource, Box<dyn std::error::Error>>{

        match cloud_type {

            "AZURE-CLOUD" => {
                match AzureCloudResource::from_path(path){
                    Ok(a) => return Ok(CloudResource::AZURE(a)),
                    Err(e) => return Err(e.to_string().into())
                }
            },
            _ => Err("CloudResource::from_path - cloudtype not supported".into())

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
///
/// TODO: Should implement the following trait: 
///    - New(type, config) -> see how this is called in aurr_core
///    - Download 
///    - Monitor a cloud resource
///    - Add a trigger (when a cloud resource is done upload. Start to download the resource)
/// 
pub trait CloudServiceManagerTrait {
    async fn new(cloud_service_manager_type:CloudServiceManager, config:&Config) -> Result<CloudServiceManager, Box<dyn std::error::Error>>;
    async fn test_connection(&self) -> Results<bool, Box<dyn std::error::Error>>
    async fn upload(&self, resource:LocalResource, some_cloud_storage_path:&str) -> Result<CloudResource, Box<dyn std::error::Error>>;
    async fn grant_read_access(&self, cloud_resource:CloudResource, timeout:u8) -> Result<String, Box<dyn std::error::Error>>; 
    async fn grant_upload_token(&self, cloud_resource:CloudResource, timeout:u8) -> Result<String, Box<dyn std::error::Error>>;
    async fn grant_upload_url(&self, cloud_resource:CloudResource, timeout:u8) -> Result<String, Box<dyn std::error::Error>>;
    fn get_name(&self) -> String;
    fn get_type(&self) -> String;

    async fn list_containers(&self) -> Result<Vec<String>, Box<dyn std::error::Error>>;
    async fn list_blobs_container(&self, container_name:&str) -> Result<Vec<String>, Box<dyn std::error::Error>>;
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

    async fn grant_upload_url(&self, cloud_resource:CloudResource, timeout:u8) -> Result<String, Box<dyn std::error::Error>> {

        match self{
            CloudServiceManager::Azure(asm) => asm.grant_upload_url(cloud_resource, timeout).await
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

    async fn list_containers(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        match self{
            CloudServiceManager::Azure(acm) => {
                match acm.list_containers().await{
                    Ok(vec) => Ok(vec.iter().map(|c| c.name.clone()).collect::<Vec<String>>()),
                    Err(e) => {
                        Err(e.to_string().into())
                    }
                }
            },
            
        }
    }

    async fn list_blobs_container(&self, container_name:&str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        match self{
            CloudServiceManager::Azure(acm) => acm.list_blobs_container(container_name).await
        }
    }



}

impl CloudServiceManager {

    //Function to convert self as Option<&AzureStorageMgmt>
    pub fn as_azure(&self) -> Option<&AzureStorageMgmt>{
        match &self{
            CloudServiceManager::Azure(a) => Some(a)
        }
    }

    ///Function to convert self as Option<&mut AzureStorageMgmt>
    pub fn as_mut_azure(&mut self) -> Option<&mut AzureStorageMgmt>{
        match self{
            CloudServiceManager::Azure(a) => Some(a)
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
                info!("{:?}",acr);
                self.get_blob_download_url(None, acr, timeout).await
            }
        }
    }

    async fn list_containers(&self) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        match self.list_containers().await{
            Ok(vec) => Ok(vec.iter().map(|c| c.name.clone()).collect::<Vec<String>>()),
            Err(e) => {
                Err(e.to_string().into())
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
    /// Trait Function to grant a upload url to any type of azure cloud resource
    /// 
    async fn grant_upload_url(&self, cloud_resource:CloudResource, timeout:u8) -> Result<String, Box<dyn std::error::Error>> {
        match cloud_resource{
            CloudResource::AZURE(cr) => {
                let t:String = match cr{
                    AzureCloudResource::Container(con) => {
                        self.gem_upload_container_url(&con, timeout).await?
                    }
                    _ => return Err("Generation for AzureCloudResource not implemented YET -> Add entry in AzureStorageMgmt::CloudServiceManagerTrait::grant_upload_url".into())
                };
                Ok(t)
            }

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

    ///
    /// A trait function to list all blobs in a specific container. 
    /// Returns only the blob name for each blob.
    /// 
    async fn list_blobs_container(&self, container_name:&str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        match self.list_blobs(container_name).await{
            Ok(blobs) => {
                Ok(
                    blobs.iter().map(|b| b.name.to_string()).collect()
                )
            },
            Err(e) => {
                Err(e.to_string().into())
            }
        }
    }
}
