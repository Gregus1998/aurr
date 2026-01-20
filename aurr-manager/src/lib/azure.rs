// Library to store all azure structs and functions. 
use crate::{error, lib::{
    error::CustomError, tools::Tool}};
use std::{fmt::Debug, fs::{self, DirEntry, File, create_dir_all}, io::{self, Read}, str::FromStr, vec};
use azure_core::{http::Url, time::OffsetDateTime};
use azure_storage_blobs::
    {blob::{Blob, operations::PutBlobResponse}, 
        container::Container, 
        prelude::*
    };

use azure_storage::prelude::*;
use tokio::runtime::{self, Runtime};
use futures::{future, stream::StreamExt};
use serde::{Deserialize};



//enum to structure local resources
pub enum LocalResource {
    Text(String),
    Entry(DirEntry),
    Tool(Tool)
}

pub enum CloudResource{
    Text(String),
    Blob(Blob),
    BlobClient(azure_storage_blobs::prelude::BlobClient)
}

impl CloudResource{

    pub fn get_blobclient(&self, cc:ContainerClient) -> BlobClient{
        //Function to get a blobclient for a CloudResource. 
        //Need ContainerClient

        match self{
            CloudResource::Blob(blob) => cc.blob_client(blob.name.clone()),
            CloudResource::Text(s) => cc.blob_client(s),
            CloudResource::BlobClient(_bc) => _bc.clone(),
        }
    }

}

#[derive(Deserialize, Debug)]
pub struct Config{
    account_name:String,
    sas_token:String,
    tools_dir_local:String,
    tools_dir_cloud:String,
    upload_dir:String,
}

 ///Wrapper Structure for the azure mgmt and core features     
 pub struct AzureStorageMgmt{
    ///Structure for the azure 
    account_name:String, 
    creds: StorageCredentials,
    bsc: BlobServiceClient
}

impl AzureStorageMgmt {

    ///Function to create a new Azure_storage_Mgmt object
    pub fn new(account_storage_name:&str, sas_token:&str) -> Result<AzureStorageMgmt, CustomError>{

        match StorageCredentials::sas_token(sas_token) {

            Err(e) => {
                error!("Could not create storage credentials due to: {}",e );
            },
            Ok(s) => {
                return Ok(AzureStorageMgmt { 
                    account_name : account_storage_name.to_string(), 
                    creds : s.clone(),
                    bsc : BlobServiceClient::new(account_storage_name.to_string(), s)
                })}
        }
    }

    pub async fn list_containers(&self) -> Result<Vec<Container>,CustomError>{

        ///Function to return a vector of asure storage account containers.

        let blob_service_client =  BlobServiceClient::new(self.account_name.clone(),self.creds.clone()); 

        let mut response = blob_service_client.list_containers().into_stream();

        let mut containers:Vec<Container> = Vec::new();

        while let Some(r) = response.next().await{
            containers = r.unwrap().containers;
        }

        Ok(containers)
    } 

    pub async fn create_container(&self, container_name:&str) -> Result<ContainerClient, CustomError>{

        //Function to create a new container with the given provided container_name:&str

        let container_client = self.bsc.container_client(container_name);

        match container_client.create().await{
            Ok(s)     => {return Ok(container_client)},
            Err(e) => {return Err(CustomError::AzureStorageError(e))}
        }
    }

    pub async fn delete_container(&self, container_name:&str) -> Result<(), CustomError>{

        //Function to delete a container by name. If wrong name -> error

        match self.get_container_client(container_name, false).await{
            Ok(cc) => {
                match cc.delete().await{
                    Ok(s) => {return Ok(())}
                    Err(e) => {return Err(CustomError::AzureStorageError(e))}

                }
            },
            Err(e) => {return Err(e)}

        }

    }

    pub async fn get_container_client(&self, container_name:&str, create_container:bool) -> Result<ContainerClient,CustomError>{

        let container_client = self.bsc.container_client(container_name);


        match container_client.exists().await{

            Ok(bool) => {
                
                if bool{
                    return Ok(container_client);
                }else {

                    //If create_container flag is set -> try to create container
                    if create_container{
                        return(self.create_container(container_name).await)
                    }

                    return Err(CustomError::GenericError(format!("Container does not exist | containername: {:?}", container_name)))
                }

            },
            Err(e) => {return Err(CustomError::AzureStorageError(e))}
            
        }
    
    }

    pub async fn list_blobs(&self, container_name:&str) -> Result<Vec<Blob>,CustomError>{

        //Function to get a Result<Vec<Blob>,CustomError> of all blob-objects in a provided container_name:&str

        let container_client = self.get_container_client(container_name, false).await;

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
                return Err(e)
            }
        }

    }

    pub async fn upload(&self, container_name:&str, blob_name:&str, blob_data:Vec<u8>, overwrite:bool) -> Result<(), CustomError>{

        //Function to upload any data:Vec<u8> to a blob in a container

        let cc = self.get_container_client(container_name, true).await.unwrap();

        let bc= cc.blob_client(blob_name);
        
        let exists = bc.exists().await.unwrap();

        if exists{
            if overwrite == false{
                return Err(CustomError::GenericError("Blob exists - overwrite set til 'false'".to_string()));
            }
        }

        let a = bc.put_block_blob(blob_data).await.unwrap();

        Ok(())
    }

    pub async fn gen_resource_token(&self, container:&str, t_resource:CloudResource, perm:&str) {
        //Function to provide a acess token to some cloud resource

        //Get the container client
        let cc = self.get_container_client(container, true).await.unwrap();

        //Getting the blob_client for the target resource in this containere
        let bc = t_resource.get_blobclient(cc);

        let sas = bc.shared_access_signature(
            BlobSasPermissions { 
                    read: true, 
                    add: false, 
                    create: false, 
                    write: false, 
                    delete: false, 
                    delete_version: false, 
                    permanent_delete: false, 
                    list: true, 
                    tags: false, 
                    move_: false, 
                    execute: false, 
                    ownership: false, 
                    permissions: false }, 
                OffsetDateTime::now_utc().replace_hour(12).unwrap()).await.unwrap();

        println!("{:?}",sas.token());
    }

}

pub struct CloudBasedFetchExecuteMngr{
    pub azure_mgmt: AzureStorageMgmt,
    config_file_path: String,
    config:Config,
    tools_local:Option<Vec<fs::DirEntry>>,
    tools_cloud:Option<Vec<String>>
}

impl CloudBasedFetchExecuteMngr {

    pub fn new(config_path:String) -> CloudBasedFetchExecuteMngr{

        let mut file = File::open(&config_path).unwrap();
        let mut content = String::new();
        file.read_to_string(&mut content).unwrap();
        let config:Config = serde_json::from_str(&content).unwrap();
        let azmgmt = AzureStorageMgmt::new(&config.account_name, &config.sas_token).unwrap();

        CloudBasedFetchExecuteMngr { azure_mgmt: azmgmt, config_file_path: config_path, config, tools_local:None, tools_cloud:None}
    }

    pub fn print_config(&self){
        println!("{:#?}",self.config);
    }

    pub fn update_local_tools_index(&mut self){
        //Function to update local tool index

        let mut tools:Vec<fs::DirEntry> = Vec::new();
        let t = fs::read_dir(&self.config.tools_dir_local).unwrap();
        
        for tool in t{
            tools.push(tool.unwrap());
        }
        self.tools_local = Some(tools);
    }

    pub async fn update_cloud_tools_index(&mut self){
        
        let tools = self.azure_mgmt.list_blobs(&self.config.tools_dir_cloud).await.unwrap();
        let mut vec:Vec<String> = Vec::new();
        for blob in tools.iter(){
            vec.push(blob.name.clone());
        }
        self.tools_cloud = Some(vec);
    }

    pub fn list_tools_local(&mut self){
        self.update_local_tools_index();

        for t in self.tools_local.as_ref().unwrap(){
            println!("{:?}",t.file_name());
        }
    }

    pub async fn list_tools_cloud(&mut self){
        self.update_cloud_tools_index().await;

        for t in self.tools_cloud.as_ref().unwrap().iter(){
            println!("{:?}",t);
        }
    }

    pub async fn list_containers(&self){

        match self.azure_mgmt.list_containers().await {
            Ok(a) => {
                for i in a.iter(){
                    println!("{:?}",i.name);
                }
            },
            Err(e) => println!("{:#?}", e),
        }
    }

    pub async fn upload_resource(&self, localresource:LocalResource, blob_name:&str, container:&str, overwrite:bool){
        //Function to upload a filesystem resource to a container. 
        match localresource{
            LocalResource::Entry(resource) => {
                let blob_name = resource.file_name().into_string().unwrap();

                let mut data_vec:Vec<u8> = Vec::new();

                let mut f = File::open(resource.path()).unwrap();

                f.read_to_end(&mut data_vec).unwrap();

                self.azure_mgmt
                    .upload(container, &blob_name, data_vec, overwrite)
                    .await
                    .expect("could not upload blob");
             },
            
            LocalResource::Text(s) => {

                let mut f = File::open(s).unwrap();
                let mut content:Vec<u8> = Vec::new();
                f.read(&mut content).unwrap();

                self.azure_mgmt
                    .upload(container, blob_name, content, overwrite)
                    .await
                    .expect("could not upload blob");

            }

            LocalResource::Tool(tool) => {todo!()}

        }

    }

    pub async fn create_resource_access(&self, t_resource:CloudResource){
        

    }     

}


