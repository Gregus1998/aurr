/*  A set of functions to parse and process the task template. 
    This will be used to make it more user friendly for anyone to use this.
    The following section should explain and document some of the features and rules. 

    The goal is to load a template, and then use this to create a list of tasks that needs to be done for everything to work.
*/
use crate::{error, impl_has_name, lib::{aurr_core::{
        HasName, load_json, load_json_btreemap, print_btmap, Shell
    }}};

use config::{Case, Config};
use serde::de::DeserializeOwned;
use std::{collections::{BTreeMap, HashMap}, fs, hash::Hash, process::exit};

/// 
/// A case structure for the case template. 
/// The case template will mainly be the one template that needs to be edited for each individual case. 
///  
#[derive(serde::Deserialize, Debug, Clone)]
pub struct CaseTemplate{

    pub name : String,
    hostname : String,
    pub task_template : TaskTemplate
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct TaskTemplate {
    name : String,
    pub os : String,
    pub shell :Shell,
    tasks : BTreeMap<String, Option<BTreeMap<String,Vec<String>>>> //Cursed nested structure >:()
}

impl_has_name!(TaskTemplate);
impl_has_name!(CaseTemplate);

impl CaseTemplate{

    pub fn new_from_task(name:Option<String>, hostname:Option<String>, tasktemplate:TaskTemplate) -> CaseTemplate{

        let n = match name{
            Some(nn) => nn,
            None => uuid::Uuid::new_v4().to_string()
        };

        let h = hostname.unwrap_or("N/A".to_string());

        CaseTemplate { name: n, hostname: h, task_template: tasktemplate }

    }

    /// 
    /// Function to load a nested json structure. a Case template need a path to a task template. 
    /// Made AI do some work here. It seems to work
    ///  
    pub fn load_from_json(path:&str) -> Result<CaseTemplate,Box<dyn std::error::Error>>
    {
        let data = std::fs::read_to_string(path).unwrap();
        let case_data: serde_json::Value = serde_json::from_str(&data).unwrap();
        
        let task_template_path = case_data["task_template"].as_str()
            .ok_or("task_template path not found")?;
        
        let task_template: TaskTemplate = match load_json(task_template_path){
            Ok(s) => s,
            Err(e) => return Err(format!("Could not load task tempalte due to: {}",e.to_string()).into())
        };

        let ct = CaseTemplate {
                name : case_data["name"].as_str().unwrap_or("").to_string(),
                hostname : case_data["hostname"].as_str().unwrap_or("").to_string(),
                task_template : task_template
            };
        
        Ok(ct)
    }


    /// 
    /// Function to load all case templates in a directory and load the first one with the desired name
    /// 
    pub fn load_from_path_name(name:&str, path:&str)-> Result<CaseTemplate,Box<dyn std::error::Error>>{

        let files = fs::read_dir(path)?;

        for f in files.into_iter(){

            let case = CaseTemplate::load_from_json(
                f.unwrap()
                .path()
                .as_mut_os_str()
                .to_str()
                .expect("The provided path cannot be converted to os path :("))?;

            if case.name.eq(name){
                return Ok(case);
            }
        }

        Err(format!("Provided case template name does not exist in direcory: {}", path).into())

    }

    ///
    /// Function to list a case template for the help meny
    /// 
    pub fn ls_case(&self) -> String{
        format!("
      name: {}
      hostname: {}
        task_template: {}
        ", self.name,self.hostname,self.task_template)
    }


}


impl TaskTemplate {


     /// 
    /// Function to load all task templates in a directory and load the first one with the desired name
    /// 
    pub fn load_from_path_name(name:&str, path:&str)-> Result<TaskTemplate,Box<dyn std::error::Error>>{

        let files = fs::read_dir(path)?;

        for f in files.into_iter(){

            let task: TaskTemplate = match load_json(f.unwrap().path().as_os_str().to_str().unwrap()){
                Ok(s) => s,
                Err(e) => return Err(format!("Could not load task tempalte due to: {}",e.to_string()).into())
            };

            if task.name.eq(name){
                return Ok(task);
            }
        }

        Err(format!("Provided case template name does not exist in direcory: {}", path).into())

    }

    pub fn load_from_json<T>(path:&str) -> Result<BTreeMap<String, T>,Box<dyn std::error::Error>>
    where
        T: DeserializeOwned + Clone + Hash + Eq,
    {
        load_json_btreemap(path)
    }

    pub fn list_tasks(&self) -> Vec<String> {
        self.tasks.keys().cloned().collect()
    }

    ///
    /// Return the inner BTreeMap
    /// 
    pub fn tasks(&self) -> BTreeMap<String, Option<BTreeMap<String,Vec<String>>>>{
        self.tasks.clone()
    }

    pub fn list_tasks_clean(&self) -> BTreeMap<String, String>{
        let mut map = BTreeMap::new();

        for (i,v) in &self.tasks{
            let val = match v{
                None => "None",
                Some(a) => &print_btmap(&a)
            };

            map.insert(i.clone(), val.to_string());
        }

        map
    }

}

/// 
/// Implementation of the display for TaskTemplate
/// 
impl std::fmt::Display for TaskTemplate {

    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {

        let tasks = self.list_tasks_clean();

        let s = format!("
        TaskName: {},
        OS: {},
        Shell: {:?},
        Tasks: {}
        ",self.name.clone(),self.os,self.shell,print_btmap(&tasks));
        f.write_str(s.as_str())
    }
}