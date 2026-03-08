// Library to store all azure structs and functions. 
use crate::{error, info, lib::{
    aurr_core::LocalResource}};

use async_recursion::async_recursion;
use colored::Colorize;
use std::{collections::{BTreeMap,}, error::Error, fmt::Debug, fs::{self, File}, io::{Read, Write}, process::exit, thread::sleep};
use azure_core::{time::{Duration,OffsetDateTime}
};
use azure_storage_blobs::
    {blob::{Blob, BlobProperties}, 
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
    /// A function to take a AzureCloudResource::text and split it into container and blob based on a separator
    /// 
    pub fn get_container_blob_by_pathsep(&self) -> Result<(Option<&str>,Option<&str>), Box<dyn std::error::Error>>{


        match &self{
            AzureCloudResource::Text(s) => {
                let sep = match s.contains("::"){
                    true => "::",
                    false => "/"
                };

                match s.split_once(sep){
                    None => return Err("Using wrong separator in container/blob path. Use \"container/blob\" OR \"container::blob\"".into()),
                    Some((a,b)) => Ok((Some(a),Some(b)))
                }
            },
            AzureCloudResource::Container(con) => Ok((Some(&con.name), None)),

            AzureCloudResource::Blob(blob) => Ok((None, Some(&blob.name))),

            _ => {
                return Err("Only supported for AzureCloudResource::text".into())}
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


    /// 
    /// A wrapper function to return itself as a string. 
    /// Could probably have done this via the fmt trait
    /// 
    pub fn as_string(&self) -> String{
        match self{
            AzureCloudResource::Blob(b) => b.name.to_string(),
            AzureCloudResource::BlobClient(bc) => bc.blob_name().to_string(),
            AzureCloudResource::Text(t) => t.to_string(),
            AzureCloudResource::Container(con) => con.name.to_string()
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
            containers = match r{
                Ok(con) => con.containers,
                Err(e) => {
                    return Err(format!("{}",e.into_inner().unwrap()).into());
                }
            };
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
            },

            LocalResource::AurrObject(ao) => {
                self.upload_resource(&LocalResource::Text(ao.local_path.to_string()), &ao.name, container, overwrite).await
            }

        }

    }


    /// 
    /// A function to download a specific cloud resource
    /// 
    pub async fn download_resource(&self, cloud_resource:AzureCloudResource, download_dir:&str) -> Result<(), Box<dyn std::error::Error>>{
        let (con_name,blob_name) = cloud_resource.get_container_blob_by_pathsep().unwrap();

        let cc = self.get_container_client(con_name.unwrap()).await?;

        let bc = cc.blob_client(blob_name.unwrap());

        let mut blob_stream = bc.get().into_stream();

        let path = format!("{}/{}",download_dir,bc.blob_name());
        
        //Checks if the file exists -> if it exists, create a new file with UTC timetamp at the end. 
        let mut file = match fs::File::open(&path){
            Err(_) => fs::File::create(&path)?,
            Ok(_) => {
                let new_path = format!("{}<{}>",path, OffsetDateTime::now_utc());
                fs::File::create(new_path)?
            }
        };

        while let Some(val) = blob_stream.next().await{
            let bytes =val.unwrap().data.collect().await?;
            file.write_all(&bytes).unwrap();
        }

        file.flush()?;

        info!("Download of blob <{}> Complete ",cloud_resource.get_name());

        Ok(())
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


    /// 
    /// A function to get some basic information of a cloud resource 
    /// Aims to use this to check if a cloud resource is done uploaded
    /// 
    pub async fn get_status_acr(&self, cloud_resource:AzureCloudResource) -> Result<(), Box<dyn std::error::Error>>{

        //Getting the container name/blob name
        let (cont_name, blob_name) = cloud_resource.get_container_blob_by_pathsep().unwrap();

        let cc = self.get_container_client(cont_name.unwrap()).await.unwrap();

        let bc = cc.blob_client(blob_name.unwrap());

        let prop = bc.get_properties().await.unwrap();

        let meta = bc.get_metadata().await.unwrap();

        let properties = prop.blob.properties.clone();

        let uploaded = if AzureStorageMgmt::check_uploaded(&properties, &Duration::minutes(2)){
            format!("{}", "[UPLOADED]".green())
        }else {
            format!("{}","[UNSURE]".red())
        };

        let s = format!(
        "
    Blob: 
        Name: {}
        Path: {}
    Properties:
        Creation_Time(UTC): {}
        Last_Modified(UTC): {}
        Last_Accessed{}
        ETag: {}
        Content_Len: {}
        md5: {}
    Metadata:
        Meta_Date: {}
        Meta_ETag: {}
        Request_ID: {}
        Server: {}
    Uploaded status:
        {}
",
        prop.blob.name,
        cloud_resource.as_string(),
        prop.blob.properties.creation_time,
        prop.blob.properties.last_modified,
        match prop.blob.properties.last_access_time{
            Some(s) => s.to_string(),
            None => "N/A".to_string()
        },
        prop.blob.properties.etag,
        prop.blob.properties.content_length,
        match prop.blob.properties.content_md5{
            Some(md5) => format!("{}",hex::encode(md5.bytes())),
            None => "N/A".to_string()
        },

        meta.date,
        meta.etag,
        meta.request_id,
        meta.server,
        uploaded

    );

        println!("{}",s);

        Ok(())
    }


    fn check_uploaded(blob_prop:&BlobProperties, last_mod_since:&Duration) -> bool{

        // produces a 5 min old timestamp
        let now = OffsetDateTime::now_utc() - *last_mod_since;


        // Checks if etag and md5 of content exists. + If the last modified timestamp is older than 5 min.
        !blob_prop.etag.to_string().is_empty() && blob_prop.last_modified < now


    }



    async fn pull_sync_blob(&self, blob_name:&str, container_name:&str, download_dir:&str, timeout:u8, check_period:u8) -> Result<(),Box<dyn Error>>{

        Ok(())
    }

    async fn pull_sync_container(&self, container_name:&str, download_dir:&str, timeout:i64, check_period:i64) -> Result<(),Box<dyn Error>> {

        info!("Running Pull_Sync on container: <{}> Timout: <{}Min> Interval: <{}Min> ",container_name, timeout,check_period);

        struct BlobStatus{
            uploaded:bool,
            downloaded:bool, 
            download_timestamp:OffsetDateTime // Should be the last_updated timestamp of that blob that is downloaded.
        }

        //Creating map to store everything.
        let mut blob_map:BTreeMap<String, BlobStatus> = BTreeMap::new();

        // Creating the download directory
        std::fs::create_dir_all(download_dir)?;

        // Creating container client
        let cc = self.bsc.container_client(container_name);

        let timeouttime = OffsetDateTime::now_utc() + Duration::minutes(timeout.into());

        // As long as not_utc is less than The previously created timeouttime do the loop
        while OffsetDateTime::now_utc() < timeouttime  {

            let cloud_files = self.list_blobs(container_name).await.unwrap();
            let d = Duration::minutes(check_period);

            for cf in cloud_files.iter(){

                let c_path = format!("{}/{}",container_name,cf.name).replace("//", "/");

                match blob_map.get_mut(&cf.name.to_string()){
                    
                    None => {
                        // Check if uploaded is good
                        let up = AzureStorageMgmt::check_uploaded(&cf.properties, &d);

                        // if uploaded is good -> download and set download flag true
                        let down = if up{
                            self.download_resource(AzureCloudResource::Text(c_path), download_dir).await?;
                            true

                        }else{
                            false
                        };

                        blob_map.insert(cf.name.to_string(), BlobStatus { 
                            uploaded: up,
                            downloaded: down,
                            download_timestamp: cf.properties.last_modified
                        });
                    },

                    // If 
                    Some(bs) => {
                        // If not uploaded, check if uploaded
                        if !bs.uploaded {
                            bs.uploaded = AzureStorageMgmt::check_uploaded(&cf.properties, &d)
                        }

                        // If not download and uploaded -> download
                        if !bs.downloaded && bs.uploaded{
                            self.download_resource(AzureCloudResource::Blob(cf.clone()), download_dir).await?;
                            bs.download_timestamp = cf.properties.last_modified; 
                            bs.downloaded = true

                        // if blob uploaded, downloaded and older timestamp then last dowloaded file -> download new copy. 
                        }else if bs.downloaded && bs.uploaded && bs.download_timestamp < cf.properties.last_modified{
                            self.download_resource(AzureCloudResource::Blob(cf.clone()), download_dir).await?;
                            bs.download_timestamp = cf.properties.last_modified; 

                        }

                    }
                }

            }

            let mut s = format!("Container: {} Time(UTC): <{}> \n",container_name, OffsetDateTime::now_utc());


            for (i,v) in blob_map.iter(){
                let s1 = if v.uploaded{&"[UPLAODED]".green()
                }else {
                    &"[NOT UPLOADED]".red()
                };

                let s2 = if v.downloaded {&"[DOWNLOADED]".green()}else {
                    &"[NOT_DOWNLOADED]".red()
                };

                s.push_str(&format!("\t{:<50} {:<} {:<} Time(UTC):<{}>\n", i,s1,s2, v.download_timestamp));
  
            } 

            println!("{}",s);

            //Sleeping for the check period. 
            sleep(std::time::Duration::from_mins(check_period.try_into().unwrap()));

            
        }

        Ok(())
    }

    /// 
    /// A function to monitor and download all new content of a azure cloud resource for a specific timeout. 
    /// 
    pub async fn pull_sync_acr(&self, acr:AzureCloudResource, download_dir:&str, timeout:i64, check_period:i64) -> Result<(),Box<dyn Error>>{

        match acr{

            AzureCloudResource::Container(con) => {
                self.pull_sync_container(&con.name, download_dir, timeout, check_period).await
            }

            // If anyone is using the "from_path" function to create Azure Cloud resources, this will give a container or text type. this means that Text will only be single blobs.  
            AzureCloudResource::Text(path ) => {
                todo!("Need to find out how to do stuff here.")
            }

            _ => return Err("The provided AzoureCloudResource is not supported yet".into())
        }

    }

}