/*  A set of functions to parse and process the task template. 
    This will be used to make it more user friendly for anyone to use this.
    The following section should explain and document some of the features and rules. 

    The goal is to load a template, and then use this to create a list of tasks that needs to be done for everything to work.
*/
use crate::{error, impl_has_name, lib::{aurr_core::{
        HasName, load_json, load_json_btreemap, print_btmap
    }, tools::{Tool, ToolConfig}}};

use config::Config;
use serde::de::DeserializeOwned;
use std::{collections::{BTreeMap, HashMap}, hash::Hash, process::exit};

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
    pub shell :String,
    tasks : BTreeMap<String, Option<BTreeMap<String,Vec<String>>>> //Cursed nested structure >:()
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
    /// Function to build a remote task list of all tools in a tools-map
    /// 
    pub fn build_task_list(&self, tools:HashMap<String,Tool>, config:&Config) -> Vec<String>{
        self.task_template.build_remote_system_task_list(tools, config)
    }

    pub fn build_task(&self,tool:&Tool, tool_config:&ToolConfig) -> Vec<String>{
        self.task_template.build_remote_system_task(tool, tool_config)
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

    ///
    /// Function to craft the cmdlines of the tools mandarory steps and call steps.
    /// -> This needs to generate tokens that can be used for a specifig tool. 
    /// 

    pub fn build_remote_system_task_list(&self, tools:HashMap<String,Tool>, config:&Config) -> Vec<String>{

        let mut res:Vec<String> = Vec::new();

        //Some stuff to structure the tasks in alphabetic order
        let mut v:Vec<_> = self.tasks.iter().collect();
        v.sort_by(|(a,_), (b,_)| a.cmp(b));

        for (_i,v) in v.iter(){
            
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
                        match t.produce_mandator_steps_by_type(super::tools::MandatorySteps::Target, &toolconfig){
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


    ///
    /// Function to build a specific task list for a given tool given a provided config
    /// 
    pub fn build_remote_system_task(&self,tool:&Tool, tool_config:&ToolConfig) -> Vec<String>{
        let mut res:Vec<String> = Vec::new();

        //Sortin all the tasks in alphanumeric order.
        let mut v:Vec<_> = self.tasks.iter().collect();
        v.sort_by(|(a,_), (b,_)| a.cmp(b));


        for (_task,value) in v.iter(){

            match value{
                None => continue,
                Some(task_map) => {

                        //Fetching the desired tool.
                        let callkeyss = match task_map.get(&tool.name){
                            Some(c) => c,
                            None => {
                                continue;
                            }
                        };

                        //For each of the mandaroty steps. If they are present. append them to the task list prior to the cmd.
                        match tool.produce_mandator_steps_by_type(super::tools::MandatorySteps::Target, &tool_config){
                            None => (),
                            Some(steps) => res.extend(steps),
                        }

                        for key in callkeyss.iter(){
                            let cmd = match tool.get_cmdline(key, &tool_config){
                                Some(s) => s,
                                None => {
                                    error!("The provided Call Key: {} Does not exist in the tool index >:() ",key);
                                    exit(15)}
                            };
                            res.push(cmd);
                        } 
                }
            }

        }

        res

    }


    ///
    /// Function to filter a hashmap of tools only by relevant tools
    /// 
    pub fn get_relevant_tools(&self, tools:&mut HashMap<String,Tool>) -> BTreeMap<String,Tool>{

        let mut rt:Vec<String> = Vec::new();

        //Function to extract the tool name of those to be used.
        //Alot of nested stuf since task:hashmap points to a hashmap of other tools and config to use. 
        for (_i,v) in self.tasks.iter(){
            match v{
                Some(map) => {
                    for k in map.keys(){
                        rt.push(k.clone());
                    }
                },
                None => continue
            }
        }

        let mut new_map:BTreeMap<String,Tool> = BTreeMap::new();

        for keys in rt.iter(){

            match tools.get(keys) {
                None => {error!("Configured tool: {} does not exist. Add it with correct name in the 'tools file'", keys)},
                Some(tool) => {
                    new_map.insert(keys.clone(), tool.clone());
                }
            };
        }
        
        new_map

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
        Shell: {},
        Tasks: {}
        ",self.name.clone(),self.os,self.shell,print_btmap(&tasks));
        f.write_str(s.as_str())
    }
}