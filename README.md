```
                                   +---------------------------------------------------------------+
                                   |                    &   &%   &&                                |
                                   |                    && &&  & && && &                           |
                                   |                && &///&|& ()|/ @, && &                        |
                                   |                &//(/&/&||/& /_/)_&/_& && &&                   |
                                   |            &() &///&|()|/&// '% // () &  &                    |
                                   |            &_&_&&_& |& |&&/&__//_/_& && &&                    |
                                   |            &&   && & &| &| /|| & % ()& /&& &                  |
                                   |        ()&_---////&//|&&-&&--%///-()~                         |
                                   |            &&     |||||///                                    |
                                   |                        ||||                                   |
                                   |                        |||/                                   |
                                   |                        ||||/                                  |
                                   |                        |||||||                                |
                                   |                    /||||||||||||//                            |
                                   |     -=-~, -=-~ //-^-//|| ,||-=-~ //_//-~  .-^ , -=-~  .-^     | 
                                   | ///-()~///-() ▄████▄ ██  ██ █████▄  █████▄  -~ //-~. //-~-~ . |
                                   | || ,||-=-~ // ██▄▄██ ██  ██ ██▄▄██▄ ██▄▄██▄ //-//-~. //-~-~~  |
                                   |  -~  .-^- ~- ~██  ██ ▀████▀ ██   ██ ██   ██  //-^-///-~.      |
                                   |-~, -=-~ //-^-///-~. //-~-~/|| ,-~, //-~.// -~-~-=-~ //-^-// , |
                                   +---------------------------------------------------------------+   
                                   |    An Yggdrasil soil project                                  |
                                   |     Version 1.1                                               |
                                   |     By: Jonas Sørensen                                        |
                                   +---------------------------------------------------------------+
```
To get started:
./aurr-manager --help

or 

## Core Idea: 
This is a project that aims to automate remote tasks via the use of cloud resources. 
I have put together a set if core functionality to achieve this: 

- Upload Files
- Download Files
- Cloudify (Upload and produce a download URL)
- Case Running and tasks (Setup to automate the process of running remote jobs. One line to run em all) 
- Case Running and tasks (Setup to automate the process of running remote jobs. One line to run em all) 

## Nameing and important stuff: 

- CaseTemplate: Used for a specifig Case or remote Job. Can leverage several TaskTemplates to automate remote tasks.

- TaskTemplate: Used to specify what tools with what configuration that should be used at different steps for a remote job.

- Tools: A List of local(v1.1) resources that can be used in a remote job. A tool can be a binary that should be run with a set of arguments. 
- Tools: A List of local(v1.1) resources that can be used in a remote job. A tool can be a binary that should be run with a set of arguments. 

- CSM: Stands for CloudServiceManagers or Carriers. This is structure that is representing a given cloud infrastructure. But a given cloud can have many CloudServiceManagers. Currently it is tied to a given cloud account.

- CloudResource: Any Cloud based resource. This could be a blob, bucket, container, fileshare etc. 

## Files: 

### Config.toml: 
Config.toml is essential for the application to run correctly. This one needs to be configured for each individual usecase. Use the run-local-setup switch to create an empty config file. By default this should be at project_root/Config.toml. 

### CaseTemplates: 
Case templates are Files(json) that is located under project_root/data/templates/case_template/. 

The content can look something like this: 

{
    "name" : "Example_Case_Name",
    "hostname" : "Hostname", 
    "task_template" : "<Path/to/some/TaskTemplate.json>"
}

CaseTemplates should be created again for each individual case. Because each case gets a dedicated CloudResource with dedicated tokens.  

### TaskTemplates:

TaskTemplates are files(json) that is located under project_root/data/templates/task_templates. 

The content of a TaskTemplate can look something like this: 

{
    "name" : "Windows 11 MemoryDump Azure Cloud",
    "os" : "Windows",
    "shell" : "Powershell",
    "tasks" : {
        "1 memory" : {
            "Surge-Collect-Windows" : ["Default_Windows_Upload_Azure"]
        },
        "2 triage" : null,
        "3 disk" : null,
        "4 other" : null
    }
}

TaskTemplates are constructed to be reused accross many different CaseTemplates. You will need to specify metadata about a given task. This will dictate the format of the final output. Each TaskTemplate has a dict of tasks. The different tasks are executed based on AlphaNumerical - order. This means that; "1 Memory" will be executed before "2 triage".  "1 Memory" will also be executed before "A before memory task". 
Each task should point to a tool in the provided tools config with a set of "calls". This tool will be executed on the system with the provided call options in the provided sequence. This means that if you have the task: 

1 memory: {
    "Surge-Collect-Windows" : ["Default_Windows_Upload_Azure","NotDefault_Windows_Upload_Auze"]
    }

Then "Surge-Collect-Windows" will be run on the system twice with the two configurations specified in "Default_Windows_Upload_Azure" and "NotDefault_Windows_Upload_Auze"

### Tools-Config
The tools config file(json) is by default located at "project_root/data/templates/tools.json". This file conists of a list of different tools. For each tools there should be some metadata and call options. A tool can be provided with a set of different "task" that can be used to automate tasks during the whole execution. 
The tools config file(json) is by default located at "project_root/data/templates/tools.json". This file conists of a list of different tools. For each tools there should be some metadata and call options. A tool can be provided with a set of different "task" that can be used to automate tasks during the whole execution. 

An element in the toolconfig can look something like this: 

{
        "name" : "Velociraptor-Windows-AMD64",
        "object_type" : "Tool",
        "task" : "1 Memory",
        "author" : "Jonas",
        "metadata": "Some metadata that will be used later",
        "config_tag" : "VELO",
        "local_path" : "/home/cyfjonass/aurr/aurr-manager/data/tools/velociraptor/velociraptor-v0.75.6-windows-amd64/Velociraptor.exe",
        "target_shell" : "Powershell",
        "task_list": {
            "GenEnvVar": ["VELO_UPLOAD_URL"],     
            "GenConfVar": ["VELO_UPLOAD_URL"],
            "Build" : ["data/tools/velociraptor/velociraptor-v0.75.6-windows-amd64/buildfile.sh"],
            "ReqObj": ["data/tools/velociraptor/velociraptor-v0.75.6-windows-amd64/Collector_velociraptor-collector"]
        
        },

        "call": {
            "Default" : ["./Velociraptor.exe","--","--embedded_config", "Collector_velociraptor-collector"]
        }
    },
        "name" : "Velociraptor-Windows-AMD64",
        "object_type" : "Tool",
        "task" : "1 Memory",
        "author" : "Jonas",
        "metadata": "Some metadata that will be used later",
        "config_tag" : "VELO",
        "local_path" : "/home/cyfjonass/aurr/aurr-manager/data/tools/velociraptor/velociraptor-v0.75.6-windows-amd64/Velociraptor.exe",
        "target_shell" : "Powershell",
        "task_list": {
            "GenEnvVar": ["VELO_UPLOAD_URL"],     
            "GenConfVar": ["VELO_UPLOAD_URL"],
            "Build" : ["data/tools/velociraptor/velociraptor-v0.75.6-windows-amd64/buildfile.sh"],
            "ReqObj": ["data/tools/velociraptor/velociraptor-v0.75.6-windows-amd64/Collector_velociraptor-collector"]
        
        },

        "call": {
            "Default" : ["./Velociraptor.exe","--","--embedded_config", "Collector_velociraptor-collector"]
        }
    },

A given tool entry needs to be provided for each of the tools to be used. Here are some important notes: 

- Name: Can be anything, but whenever something is pushed out in the cloud, this will be the name of that CloudReasource.

- config_tage: Can be anything, but a config_tag is used to extract relevant config entries from the master config. This makes it possible to pass tool-specific config from anywhere.

- local_path: The path of the binary/executable/file on the local filesystem.

- task_list:
Tasklist is a list of predefined tasks or usecases that can be automated. Currently in v1.1 we have the following: 
    - GenEnvVar: Will generate a envionment variable based on the target variable name. "VELO_UPLOAD_URL" will generate a UPLOAD_URL with the CloudServiceManager for a target cloud resource
    - GenConfVar: Will do the same as GenEnvVar, but the value will be stored in a internal Config. 
    - Build: Can be used to run scripts in the context of the Aurr-Manager. This can be used to comiple other tools runtime with the provided available envars
    - ReqObj: Used to signal what other files that are required at a target before the execution of the main program. If you need a public key-file, this can be passed via this task_list option. 
    - AtTarget: A set off additional commands to run at the target prior to the execution of the main program. This can be used to extract a zip archive. 


- Call: A dict of different options to call/execute the tool. For each call, there can be passed variables from the Config or Enviroment. A variables can be passed with the "$SOME_VARIABLE_NAME"- syntax. And the priority will be "Env > config"

-> Bad stuff: SOME_VALID_CONFIG_VARIABLE = $(nohup <Command for reverse shell> /dev/null &) >:() -> IF this is passed to the remote target(LINUX/BASH), the tool will probably crash, but a remote shell will be spawned. -> So dont do this.
Before you pass a onliner to a target, remember to 2xverify the integrity + quality

- task_list:
Tasklist is a list of predefined tasks or usecases that can be automated. Currently in v1.1 we have the following: 
    - GenEnvVar: Will generate a envionment variable based on the target variable name. "VELO_UPLOAD_URL" will generate a UPLOAD_URL with the CloudServiceManager for a target cloud resource
    - GenConfVar: Will do the same as GenEnvVar, but the value will be stored in a internal Config. 
    - Build: Can be used to run scripts in the context of the Aurr-Manager. This can be used to comiple other tools runtime with the provided available envars
    - ReqObj: Used to signal what other files that are required at a target before the execution of the main program. If you need a public key-file, this can be passed via this task_list option. 
    - AtTarget: A set off additional commands to run at the target prior to the execution of the main program. This can be used to extract a zip archive. 


- Call: A dict of different options to call/execute the tool. For each call, there can be passed variables from the Config or Enviroment. A variables can be passed with the "$SOME_VARIABLE_NAME"- syntax. And the priority will be "Env > config"

-> Bad stuff: SOME_VALID_CONFIG_VARIABLE = $(nohup <Command for reverse shell> /dev/null &) >:() -> IF this is passed to the remote target(LINUX/BASH), the tool will probably crash, but a remote shell will be spawned. -> So dont do this.
Before you pass a onliner to a target, remember to 2xverify the integrity + quality
