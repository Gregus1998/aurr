// Library to store all azure structs and functions. 
use crate::{error, info, lib::{
    aurr_core::LocalResource}};

use async_recursion::async_recursion;
use std::{fmt::Debug, fs::File, io::Read, process::exit,};
use azure_core::{
    time::{Duration, OffsetDateTime}
};
use azure_storage_blobs::
    {blob::Blob, 
        container::Container, 
        prelude::*
    };
use azure_storage::{prelude::*, shared_access_signature::service_sas::BlobSharedAccessSignature};
use futures::{stream::StreamExt};
use reqwest;


/// Enum to store the different types of blobs 
#[derive(Debug)]
pub enum AzureCloudResource{
    Text(String),
    Blob(Blob),
    BlobClient(azure_storage_blobs::prelude::BlobClient),
    Container(Container)
}

impl AzureCloudResource{

    pub fn get_blobclient(&self, cc:ContainerClient) -> Option<BlobClient>{
        //Function to get a blobclient for a AzureCloudResource. 
        //Need ContainerClient
        match self{
            AzureCloudResource::Blob(blob) => Some(cc.blob_client(blob.name.clone())),
            AzureCloudResource::Text(s) => {
                
                //Some random logic to handle if a container is passed or not. 
                let blob = match s.split_once("/"){
                    Some(prod) => prod.1,
                    None => s
                };

                Some(cc.blob_client(blob))},
            AzureCloudResource::BlobClient(_bc) => Some(_bc.clone()),
            AzureCloudResource::Container(_) => None

        }
    }

    pub fn get_name(&self) -> &str{
        match self {
            AzureCloudResource::Text(s) => s,
            AzureCloudResource::Blob(b) => &b.name,
            AzureCloudResource::BlobClient(bc) => bc.blob_name(),
            AzureCloudResource::Container(con) => &con.name 
            
        }
    }


    ///
    /// Function to get the potensial name of a container for a random set of AzureCloudReseources
    /// 
    pub fn get_container_name(&self) -> Option<String>{
        match &self{
            AzureCloudResource::BlobClient(bc) => Some(bc.container_client().container_name().to_string()),
            AzureCloudResource::Container(con) => Some(con.clone().name),
            //For all cases of AzurCloudResource::Text. There will be a path passed.
            AzureCloudResource::Text(string) =>
            Some(string.replace("\\", "/").split("/").collect::<Vec<&str>>()[0].to_string()),
            _ => None
        }
    }

    ///
    /// Function to generate a Azure cloud resource from a path.
    /// Aims to parse <container>/<blob> into a resource. 
    ///  
    pub fn from_path(path:&str) -> Result<AzureCloudResource, Box<dyn std::error::Error>>{

        //Replaces the use of "\\" to "/"
        let mut s = path.to_string().replace("\\", "/");

        //removes last element if it is equal to "/"
        //should only handle the usecase where someone ask for access to <container>/
        if s.ends_with("/"){
            s.pop();
        }

        //Splitting based on "/""
        let v:Vec<&str> = path.split("/").collect();

        match v.len(){
            0 => Err("Tried to create Azure Cloud Resource from invalid Optional Argument".into()),
            1 => Ok(AzureCloudResource::Container(Container::new(v[0]))),
            _ => Ok(AzureCloudResource::Text(s)) // Not the best approach, but whenerver a path container/path/to/blob is passed. I just move the problem to another part of th code.
        }
    }

}

 ///Wrapper Structure for the azure mgmt and core features
 /// Should handle all the core features targeting azure-cloud. 
 pub struct AzureStorageMgmt{
    ///Structure for the azure 
    pub account_name:String, 
    creds: StorageCredentials,
    bsc: BlobServiceClient
}

impl AzureStorageMgmt {

    ///Function to create a new Azure_storage_Mgmt object
    pub fn new(account_storage_name:&str, sas_token:&str) -> Result<AzureStorageMgmt, Box<dyn std::error::Error>>{

        match StorageCredentials::sas_token(sas_token) {

            Err(e) => {
                error!("Could not create storage credentials due to: {}",e );
                Err(e.into())
            },
            Ok(s) => {
                return Ok(AzureStorageMgmt { 
                    account_name : account_storage_name.to_string(), 
                    creds : s.clone(),
                    bsc : BlobServiceClient::new(account_storage_name.to_string(), s)
                })}
        }
    }

    /// 
    /// Using the access key several times here -> not ideally
    /// But if it works it works -> fix later >:)
    /// 
     
    pub fn from_access_key(account_storage_name:&str, key:&str) -> Result<AzureStorageMgmt, Box<dyn std::error::Error>>{
        Ok(
            AzureStorageMgmt{
                account_name : account_storage_name.to_string(),
                creds : StorageCredentials::access_key(account_storage_name, key.to_string()),
                bsc : BlobServiceClient::new(account_storage_name.to_string(), StorageCredentials::access_key(account_storage_name, key.to_string()))
            }
        )
    }

    ///
    /// Function to return a vector of asure storage account containers.
    /// 
    pub async fn list_containers(&self) -> Result<Vec<Container>, Box<dyn std::error::Error>>{

    
        let blob_service_client =  BlobServiceClient::new(self.account_name.clone(),self.creds.clone()); 

        let mut response = blob_service_client.list_containers().into_stream();

        let mut containers:Vec<Container> = Vec::new();

        while let Some(r) = response.next().await{
            containers = r.unwrap().containers;
        }

        Ok(containers)
    } 

    ///
    /// Function to create a new container in the storage account
    /// 
    pub async fn create_container(&self, container_name:&str) -> Result<ContainerClient, Box<dyn std::error::Error>>{

        let container_client = self.bsc.container_client(container_name);

        match container_client.create().await{
            Ok(_)     => Ok(container_client),
            Err(e) => Err(e.into())
        }
    }

    ///
    /// Function to delete a container by name. If wrong name -> error
    /// 
    pub async fn delete_container(&self, container_name:&str) -> Result<(), Box<dyn std::error::Error>>{
        match self.get_container_client(container_name).await{
            Ok(cc) => match cc.delete().await{
                Ok(_) => Ok(()),
                Err(e) => Err(format!("Could not delete container due to {}",e).into())
            }
            Err(e) => Err(format!("Could not delete container due to {}",e).into())
        
        }
    }

    /// 
    /// Function to get the container client for a given container_name
    /// 
    pub async fn get_container_client(&self, container_name:&str) -> Result<ContainerClient,Box<dyn std::error::Error>>{

        let container_client = self.bsc.container_client(container_name);

        match container_client.exists().await{
            Ok(bool) => {
                if !bool{
                    match container_client.create().await{
                        Ok(_) => return {
                            info!("Container {} created sucessfully",container_name);
                            Ok(container_client)},
                        Err(e) => {
                            Err(e.into())
                        }
                    }

                }else {
                    Ok(container_client)
                }
            },
            Err(e) => {
                Err(e.into())
            }
        }
            
    }

    /// 
    /// A function to list the blobs within a target container_name
    /// 
    pub async fn list_blobs(&self, container_name:&str) -> Result<Vec<Blob>, Box<dyn std::error::Error>>{

        //Function to get a Result<Vec<Blob>, Box<dyn std::error::Error>> of all blob-objects in a provided container_name:&str

        let container_client = self.get_container_client(container_name).await;

        match container_client{

            Ok(cc) => {
                let mut blobs = cc.list_blobs().into_stream();
                let mut results = Vec::new(); 
                
                while let Some(r) = blobs.next().await{
                    for b in r.unwrap().blobs.blobs(){
                        results.push(b.clone());
                    }
                }

                Ok(results)

            },
            Err(e) => {
                return Err(e.into())
            }
        }

    }

    ///
    /// Function to upload binary data to a blob in a container.
    /// 
    pub async fn upload(&self, container_name:&str, blob_name:&str, blob_data:Vec<u8>, overwrite:bool) -> Result<AzureCloudResource, Box<dyn std::error::Error>>{

        //Function to upload any data:Vec<u8> to a blob in a container
        let cc = self.get_container_client(container_name).await?;

        let bc= cc.blob_client(blob_name);
        
        let exists = bc.exists().await.unwrap();

        if exists{
            if overwrite == false{
                return Err("Blob exists - overwrite set til 'false'".into());
            }
        }

        match bc.put_block_blob(blob_data).await{
            Ok(_) => (),
            Err(e) => {
                error!("Could not upload binary data to <{}> <{}> due to: {}",container_name,blob_name, e.to_string());
                return Err(e.into())}
        };

        Ok(AzureCloudResource::BlobClient(bc))
    }

    /// Function to upload a local resource to the cloud. 
    /// It is important that this is tracked.
    /// Returns a Result<AzoureCloudResource> (BC)
    /// 
    #[async_recursion]
    pub async fn upload_resource(&self, localresource:&LocalResource, blob_name:&str, container:&str, overwrite:bool) -> Result<AzureCloudResource, Box<dyn std::error::Error>>{
        //Function to upload a filesystem resource to a container. 
        match localresource{
            
            LocalResource::Entry(resource) => {
                let blob_name = resource.file_name().into_string().unwrap();

                let mut data_vec:Vec<u8> = Vec::new();

                let mut f: File = File::open(resource.path()).unwrap();

                f.read_to_end(&mut data_vec).unwrap();

                let bc = self
                    .upload(container, &blob_name, data_vec, overwrite)
                    .await
                    .unwrap();
                Ok(bc)
             },
            
            LocalResource::Text(s) => {
                let mut f = File::open(s).unwrap();

                let mut content:Vec<u8> = Vec::new();
                f.read_to_end(&mut content).unwrap();

                let bc = self
                    .upload(container, blob_name, content, overwrite)
                    .await
                    .unwrap();

                Ok(bc)

            },

            LocalResource::Tool(tool) => {
                self.upload_resource(&LocalResource::Text(tool.localpath.to_string()), blob_name, container , overwrite).await
            }

        }

    }

    ///Function to generate a sas-token for a specific cloud resource.
    ///     -> Very scary function. Use with care 
    /// 
    pub async fn gen_blob_sas_token(&self, container:&str, t_resource:&AzureCloudResource, perm:BlobSasPermissions, timeout:u8) -> Option<BlobSharedAccessSignature>{
        
        //Get the container client
        let cc = self.get_container_client(container).await.unwrap();

        //Getting the blob_client for the target resource in this containere
        let bc = t_resource.get_blobclient(cc).unwrap();

        match bc.shared_access_signature(perm,OffsetDateTime::now_utc() + Duration::hours(timeout.into())).await{
            Ok(sas) => {
                info!("Generated SAS-BLOB-TOKEN for AZURE_CLOUD <{}> <{}> <{}> Timeout: UTC+{}", self.account_name, container, bc.blob_name(),timeout);
                Some(sas)},
            Err(e) => {
                error!("Could not produce SAS token due to{}",e);
                None}
        }
    }

    ///
    /// Function to generate a sas-tokoen for a container
    /// If the container does not exist, it will be created. 
    /// 
    pub async fn gen_container_sas_token(&self, container:&str, perm:BlobSasPermissions, timeout:u8) -> Option<BlobSharedAccessSignature>{
        
        //Get the container client
        let cc = match self.get_container_client(container).await{
            Ok(c) => c,
            Err(e) => {
                error!("Cound not crate container clinent for contianer: {} due to : {}",container,e.to_string());
                exit(16)
            }
        };

        match cc.shared_access_signature(perm,OffsetDateTime::now_utc() + Duration::hours(timeout.into())).await{
            Ok(sas) => {
                info!("Generated SAS-CONTAINER-TOKEN for AZURE_CLOUD <{}> <{}> Timeout: UTC+{}", self.account_name, container, timeout);
                Some(sas)},
            Err(e) => {
                error!("Could not produce SAS token due to{}",e);
                None}
        }
    }

    ///
    /// Function to generae a sas token for a given container
    /// 
    pub async fn gen_upload_container_sas(&self, container:&Container, timeout:u8)  -> Result<String, Box<dyn std::error::Error>>{
        let perm = BlobSasPermissions {
                        read: true,
                        add: true,
                        create: true,
                        write: true,
                        delete: true,
                        delete_version: false,
                        permanent_delete: false,
                        list: true,
                        tags: true,
                        move_: true,
                        execute: false,
                        ownership: false,
                        permissions: false,
                        };
        
        let sas_token = self.gen_container_sas_token(&container.name,perm, timeout)
                    .await.unwrap();
    

        Ok(sas_token.token().unwrap())
    }

    pub async fn gem_upload_container_url(&self, container:&Container, timeout:u8)-> Result<String, Box<dyn std::error::Error>>{

        let token = match self.gen_upload_container_sas(container, timeout).await{
            Ok(r) => r,
            Err(e) => return Err(e)
        };

        Ok(format!("https://{}.blob.core.windows.net/{}?{}", self.account_name, container.name, token))

    }

    ///
    /// Function to get the download url for AZURE blobs given a AzureCloudResource
    /// \n Need to provide a combination of containeroption + t_resource where it is possible to extract the desired container.
    /// 
    pub async fn get_blob_download_url(&self, containeroption:Option<&str>, t_resource:AzureCloudResource, timeout:u8) -> Result<String, Box<dyn std::error::Error>>{

        //Som error handling to make sure that you provide a sufficient amount of information. 
        let container = match containeroption{
            Some(s) => s,
            None => {
                match t_resource.get_container_name(){
                    Some(con) => &con.clone(),
                    None => return Err("Error in AzureStorageMgmt::get_blob_download_url: the provided containeroption + t_resource creates error".into())
                }
            }
        };

       

        match &t_resource{
            //Found it to be much easier to grant access to a BlobClient. 
            //All other types of AzureCloudResources that should be able to grant a read access should point to the BlobClient Switch
            AzureCloudResource::BlobClient(_) => {
                 //Defining the sas token
                let sas_token = self.gen_blob_sas_token(container, &t_resource,BlobSasPermissions {
                                read: true,
                                add: false,
                                create: false,
                                write: false,
                                delete: false,
                                delete_version: false,
                                permanent_delete: false,
                                list: false,
                                tags: false,
                                move_: false,
                                execute: false,
                                ownership: false,
                                permissions: false,
                                }, timeout)
                            .await.unwrap();
                
                Ok(format!("https://{}.blob.core.windows.net/{}/{}?{}", self.account_name, container, t_resource.get_name(), sas_token.token().unwrap()))
            }

            //Could be buggy stuff here. Be carefull when maintaining >:() 
            AzureCloudResource::Text(blob_name) => {

                // Logic to extract the container and blob path from a URL 
                let container_blob = match blob_name.replace("\\", "/").split_once("/"){
                    None => return Err("Cant Grant Read Access to CONTAINER ONLY".into()),
                    Some((i,v)) => (i.to_string(),v.to_string())
                };

                //Creating a contianer client
                let cc = self.get_container_client(&container_blob.0).await.unwrap();

                let bc = AzureCloudResource::BlobClient(cc.blob_client(&container_blob.1));

                let val = Box::pin(async move {
                    self.get_blob_download_url(None, bc, timeout).await.unwrap()
                }).await;

                Ok(val)
                
            }
            
            _ => return Err("Provided AzureCloudResource: <{}> not supported yet - Check spelling".into())
        }

    }

    /// 
    /// A fucntion to check if it is possible to reach the azure cloud. 
    /// Checks if you can reach the Azure cloud + if the basic api works.
    /// Can update this to test for different types of permissions. 
    /// 
    pub async fn check_connection(&self) -> Result<(), Box<dyn std::error::Error>>{

        let url = format!("https://{}.blob.core.windows.net/", self.account_name);

        match reqwest::Client::new()
            .head(url)
            .send().await{
                Ok(s) => {()
                },
                Err(e) => return Err(format!("Network Error! {}",e.to_string()).into())
            }

        match self.list_containers().await{
            Ok(_) => {},
            Err(e) => return Err(format!("API issues - {}",e.to_string()).into())
        };

        Ok(())
    }


}