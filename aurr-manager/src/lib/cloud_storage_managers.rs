use std::error::Error;

use azure_core::time;
use config::Config;

use crate::{debug, error, info, lib::{
    aurr_core::{GetName, LocalResource},
    azure::{AzureCloudResource, AzureStorageMgmt}
}, warning};

///
/// Just a enum to list all possible carriers / Cloud Managers.
/// 
pub enum CarrierTypes{
    Azure,
    S3,
    SMB
}

///
/// Enum to store and structure all the different types of cloud resources
///     -> Guess this can become handy if we add support or change to another cloud provider in the future
/// 
pub enum CloudResource {
    Azure(AzureCloudResource)   
}

impl CloudResource {


    pub fn as_azure(&self) -> Option<&AzureCloudResource>{
        match self {
            CloudResource::Azure(azure) => Some(azure)
        }
    }
    pub fn get_type(&self) -> String{
        match self{
            CloudResource::Azure(_) => "Azure".to_string()
        }
    }

    /// 
    /// Function to get some random information for a cloud resource
    /// Used to display metadata when uploading
    /// 
    pub fn get_info(&self) -> Option<String>{
        match self{
            CloudResource::Azure(acr) => format!("{}/{}", 
            match acr.get_container_name(){
                Some(cn) => cn,
                None => "N/A".to_string()
            },
            acr.get_name()).into()
        }
    }

    pub fn from_path(path:&str, cloud_type:&str) -> Result<CloudResource, Box<dyn std::error::Error>>{

        match cloud_type {

            "Azure" => {
                match AzureCloudResource::from_path(path){
                    Ok(a) => return Ok(CloudResource::Azure(a)),
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
    async fn test_connection(&self) -> Result<bool, Box<dyn std::error::Error>>;
    async fn get_status(&self, cloud_resource:CloudResource) -> Result<(), Box<dyn std::error::Error>>;
    
    async fn upload(&self, resource:LocalResource, some_cloud_storage_path:&str) -> Result<CloudResource, Box<dyn std::error::Error>>;
    async fn download(&self, resource:CloudResource, download_dir:&str) -> Result<(),Box<dyn Error>>;
    async fn pull_sync(&self, resource:CloudResource, download_dir:&str, timeout:i64, check_period:i64) -> Result<(),Box<dyn Error>>;

    async fn grant_read_access(&self, cloud_resource:CloudResource, timeout:u32) -> Result<String, Box<dyn std::error::Error>>; 
    async fn grant_upload_token(&self, cloud_resource:CloudResource, timeout:u32) -> Result<String, Box<dyn std::error::Error>>;
    async fn grant_upload_url(&self, cloud_resource:CloudResource, timeout:u32) -> Result<String, Box<dyn std::error::Error>>;
    
    fn get_name(&self) -> String;
    fn get_type(&self) -> String;
    fn get_info(&self) -> String;

    async fn list_containers(&self) -> Result<Vec<String>, Box<dyn std::error::Error>>;
    async fn list_blobs_container(&self, container_name:&str) -> Result<Vec<String>, Box<dyn std::error::Error>>;
} 

///
/// Implementation of CloudServiceManager
/// Mainly this will be a wrapper and forking all the different calls for different cloud managers. 
///     -> Looks unnecessary since there are only support of azure atm
/// 
impl CloudServiceManagerTrait for CloudServiceManager{

    async fn get_status(&self, cloud_resource:CloudResource) -> Result<(), Box<dyn std::error::Error>> {
        match self{
            CloudServiceManager::Azure(asm) => asm.get_status(cloud_resource).await
        }
    }
    
    async fn test_connection(&self) -> Result<bool, Box<dyn std::error::Error>> {
        match self{
            CloudServiceManager::Azure(asm) => asm.test_connection().await
        }
    }

    async fn upload(&self, resource:LocalResource, some_cloud_storage_path:&str) -> Result<CloudResource, Box<dyn std::error::Error>> {
        match self{
            CloudServiceManager::Azure(asm) => Ok(
                CloudResource::Azure(
                    asm.upload_resource(&resource, &resource.get_base_name(),some_cloud_storage_path, true).await?))
        }
    }

    async fn download(&self, resource:CloudResource, download_dir:&str) -> Result<(),Box<dyn Error>> {
        match self{
            CloudServiceManager::Azure(asm) => {
                asm.download(resource, download_dir).await
            }
        }
    }

    async fn pull_sync(&self, resource:CloudResource, download_dir:&str, timeout:i64, check_period:i64) -> Result<(),Box<dyn Error>> {
        match self{
            CloudServiceManager::Azure(acm) => acm.pull_sync(resource, download_dir,timeout, check_period).await
        }
    }

    async fn grant_read_access(&self, cloud_resource:CloudResource, timeout:u32) -> Result<String, Box<dyn std::error::Error>> {
        match self{
            CloudServiceManager::Azure(asm) => asm.grant_read_access(cloud_resource,timeout).await
        }
    }

    async fn grant_upload_token(&self, cloud_resource:CloudResource, timeout:u32) -> Result<String, Box<dyn std::error::Error>> {
        match self{
            CloudServiceManager::Azure(asm) => asm.grant_upload_token(cloud_resource,timeout).await
        }
    }

    async fn grant_upload_url(&self, cloud_resource:CloudResource, timeout:u32) -> Result<String, Box<dyn std::error::Error>> {

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

    fn get_info(&self) -> String {
        match self{
            CloudServiceManager::Azure(acm) => acm.get_info()
        }
    }

    /// A function to list containers at the root of the cloud managers. 
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

    pub fn new(cloud_type:CarrierTypes, config:&Config) -> Result<CloudServiceManager, Box<dyn std::error::Error>> {
        
        match cloud_type{
            
            CarrierTypes::Azure => {
                Ok(
                    CloudServiceManager::Azure({
                        AzureStorageMgmt::from_access_key(
                            config.get::<String>("AZURE_ACCOUNT_STORAGE_NAME").unwrap().as_str(),
                            config.get::<String>("AZURE_ACCESS_KEY").unwrap().as_str()
                            ).unwrap()}
                        )       
                )
            },
            _ => Err("The provided cloud type is not supported".into())
        }


    }

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


    async fn get_status(&self, cloud_resource:CloudResource) -> Result<(), Box<dyn std::error::Error>> {
        match cloud_resource{
            CloudResource::Azure(acr) => self.get_status_acr(acr).await,
            _ => return Err("Passed wrong CloudResource to AzureCloudManager".into())
        }
    }


    async fn test_connection(&self) -> Result<bool, Box<dyn std::error::Error>> {
        match self.check_connection().await{
            Ok(_) => Ok(true),
            Err(e) => {
                warning!("Connection Check to Azure Cloud Failed");
                debug!("do I se this");
                info!("Is the Azure Access Key present? Run switch <ls config> to verify. Account Key can be passed as env_var with: <export AURR_KEY=\"<some_account_key>\">");
                Ok(false)
            }
        }
    }

    async fn upload(&self, resource:LocalResource, some_cloud_storage_path:&str) -> Result<CloudResource, Box<dyn std::error::Error>> {
        Ok(
            CloudResource::Azure(
                self.upload_resource(&resource, &resource.get_name(),some_cloud_storage_path, true).await.unwrap()
        ))
    }

    async fn download(&self, resource:CloudResource, download_dir:&str) -> Result<(),Box<dyn Error>> {
        match resource{
            CloudResource::Azure(acr) => {
                self.download_resource(acr, download_dir).await
            }
            _ => Err("Passing wrong cloud resource to Azure".into())
        }
    }

    async fn pull_sync(&self, resource:CloudResource, download_dir:&str, timeout:i64, check_period:i64) -> Result<(),Box<dyn Error>> {
        match resource{
            CloudResource::Azure(acr) => {
                self.pull_sync_acr(acr, download_dir, timeout, check_period).await
            },
            _ => return Err("Provided invalid cloud resource for azure cloud manager".into())
        }
    }

    async fn grant_read_access(&self, cloud_resource:CloudResource, timeout:u32) -> Result<String, Box<dyn std::error::Error>>{
        
        match cloud_resource{
            CloudResource::Azure(acr) => {
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
    async fn grant_upload_token(&self, cloud_resource:CloudResource, timeout:u32) -> Result<String, Box<dyn std::error::Error>> {
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
            None => return Err(format!("Provided mismatch cloud resource {} for Azure cloud",cloud_resource.get_type()).into())

        }
    }

    ///
    /// Trait Function to grant a upload url to any type of azure cloud resource
    /// 
    async fn grant_upload_url(&self, cloud_resource:CloudResource, timeout:u32) -> Result<String, Box<dyn std::error::Error>> {
        match cloud_resource{
            CloudResource::Azure(cr) => {
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
    /// Trait function to return "Azure-CLOUD"
    /// Used runtime to display information
    /// 
    fn get_type(&self) -> String {
        "Azure".to_string()
    }


    /// 
    /// Trait Function to get metadata about the azure manager.
    /// 
    fn get_info(&self) -> String {

        format!("via Azure Storage Cloud: Account: <{}>", self.account_name)
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
