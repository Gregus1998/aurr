use std::{collections::HashMap, hash::Hash, io::Error};

/*  A set of functions to parse and process the task template. 
    This will be used to make it more user friendly for anyone to use this.
    The following section should explain and document some of the features and rules. 

    The goal is to load a template, and then use this to create a list of tasks that needs to be done for everything to work.
*/
use crate::{error, impl_has_name, lib::{aurr_core::{
        HasName, load_json, load_json_hashmap, load_json_vec, load_manyjson_hashmap_by_name
    }, tools::{Tool, ToolConfig}}};
    
use config::{Config, Value};
use serde::de::DeserializeOwned;



/// 
/// A case structure for the case template. 
/// The case template will mainly be the one template that needs to be edited for each individual case. 
///  
#[derive(serde::Deserialize, Debug, Clone)]
pub struct CaseTemplate{

    name : String,
    hostname : String,
    pub task_template : TaskTemplate
}

#[derive(serde::Deserialize, Debug, Clone)]
pub struct TaskTemplate {
    name : String,
    os : String,
    pub shell :String,
    tasks : HashMap<String, Option<HashMap<String,Vec<String>>>> //Cursed nested structure >:()
}

impl_has_name!(TaskTemplate);
impl_has_name!(CaseTemplate);

impl CaseTemplate{

    /// 
    /// Function to load a nested json structure. a Case template need a path to a task template. 
    /// Made AI do some work here. It seems to work
    ///  
    pub fn load_from_json(path:&str) -> Result<CaseTemplate,Box<dyn std::error::Error>>
    {
        let data = std::fs::read_to_string(path)?;
        let mut case_data: serde_json::Value = serde_json::from_str(&data)?;
        
        let task_template_path = case_data["task_template"].as_str()
            .ok_or("task_template path not found")?;
        
        let task_template: TaskTemplate = load_json(task_template_path)?;

        let ct = CaseTemplate {
                name : case_data["name"].as_str().unwrap_or("").to_string(),
                hostname : case_data["hostname"].as_str().unwrap_or("").to_string(),
                task_template : task_template
            };
        
        Ok(ct)
    }

    pub fn build_task_list(&self, tools:HashMap<String,Tool>, config:&Config) -> Vec<String>{
        self.task_template.build_remote_system_task_list(tools, config)
    }


}

impl TaskTemplate {
    pub fn load_from_json<T>(path:&str) -> Result<HashMap<String, T>,Box<dyn std::error::Error>>
    where
        T: DeserializeOwned + Clone + Hash + Eq,
    {
        load_json_hashmap(path)
    }

    pub fn list_tasks(&self) -> Vec<String> {
        self.tasks.keys().cloned().collect()
    }

    ///
    /// Function to craft the cmdlines of the tools mandarory steps and call steps.
    ///

    pub fn build_remote_system_task_list(&self, tools:HashMap<String,Tool>, config:&Config) -> Vec<String>{

        let mut res:Vec<String> = Vec::new();

        //Some stuff to structure the tasks in alphabetic order
        let mut v:Vec<_> = self.tasks.iter().collect();
        v.sort_by(|(a,_), (b,_)| a.cmp(b));

        for (i,v) in v.iter(){
            
            match v {

                Some(map) => {

                    for (tool, callkeyss) in map.iter(){
                        
                        //Get the tool from the tool-map
                        let t = match tools.get(tool){
                            Some(t) => t,
                            None => {
                                error!("Tool: {} not available in the tool list", tool);
                                continue;
                            }
                        };
                        
                        //Extracting toolconfig from another config. Probably bad since I clone config here.
                        let toolconfig = ToolConfig::from_config_by_tag(config.clone(), &t.config_tag).unwrap();
                        
                        //For each of the mandaroty steps. If they are present. append them to the task list prior to the cmd.
                        match t.get_mandatory_step(&toolconfig){
                            None => (),
                            Some(steps) => res.extend(steps),
                        }

                        for key in callkeyss.iter(){
                            let cmd = t.get_cmdline(key, &toolconfig).unwrap();
                            res.push(cmd);
                        } 
                    } 

                }
                None => continue
            }
        }

        res
    }

}