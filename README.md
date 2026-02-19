# AURR - A Yggdrasil soil project

To get started:
./aurr-manager --help

or 

## Core Idea: 
This is a project that aims to automate remote tasks via the use of cloud resources. 
I have put together a set if core functionality to achieve this: 

- Upload Files
- Download Files
- Cloudify (Upload and produce a download URL)
- Case Running (Setup to automate the process of running remote jobs. One line to run em all) 

## Nameing and important stuff: 

- CaseTemplate: Used for a specifig Case or remote Job. Can leverage several TaskTemplates to automate remote tasks.

- TaskTemplate: Used to specify what tools with what configuration that should be used at different steps for a remote job.

- Tools: A List of local/(or CLOUD:TODO! ) resources that can be used in a remote job. A tool can be a binary that should be run with a set of arguments. 

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
The tools config file(json) is by default located at "project_root/data/templates/tools.json". This file conists of a list of different tools. For each tools there should be some metadata and call options. Additionally it is support for mandatory steps anything that needs to be done before the execution of the actual tool. 

An element in the toolconfig can look something like this: 

{
    "name" : "Surge-Collect-Windows",
    "task" : "1 memory",
    "author" : "Jonas Sørensen",
    "config_tag" : "SURGE",
    "localpath" : "PATH/YOU/EXECUTABLE",
    "mandatory_steps" : {
        "Generate" : ["SURGE_SAS-UPLOAD-TOKEN"]
    },
    "call" : {
        "Default_Windows_Upload_Azure" : [".\\Surge-Collect.exe","SURGE_COLLECT_PASSWORD","AZBLOB://AZURE_ACCOUNT_STORAGE_NAME/CLOUD_DEFAULT_UPLOAD_LOCATION","--azblob-sas-token='SURGE_SAS-UPLOAD-TOKEN'"],
        "Default_Windows_Test" : [".\\Surge-Collect.exe", "SURGE_COLLECT_PASSWORD", "--help"],
    }
},

A given tool entry needs to be provided for each of the tools to be used. Here are some important notes: 

- Name: Can be anything, but whenever something is pushed out in the cloud, this will be the name of that CloudReasource.

- config_tage: Can be anything, but a config_tag is used to extract relevant config entries from the master config. This makes it possible to pass tool-specific config from anywhere.

- local_path: The path of the binary/executable/file on the local filesystem.

- mandatory_steps: A set of steps of tasks that needs to be done before the execution fo the actual tool. This can be production of environment variables, compilation of code, unzip of archive. 

- Call: A dict of different options to call/execute the tool. For each call, there will be extracted fields from the config. So if the string "SUPER_TOKEN" is present in the call option and is an entry in the config file, then this will be replace in the output. This should have been done differently by whatever. Currently there is a 1-1 string match. So use unique variable names to prevent coallition. And this is probalby vulnerable to some sort of config tempering / poisoning. Blobably not on the local filesystem, but at the place where you run remote code. 

-> Bad stuff: SOME_VALID_CONFIG_VARIABLE = $(Command for reverse shell >:( ) -> IF this is passed to the remote target, the tool will probably crash, but a remote shell will be spawned. 


 
## Tips and Triks: .devcontainer/devcontainer.json

Dont know if we need: .devcontainer/devcontainer.json 
    -> but it makes it possible to run the code on any system with docker. Sounds nice. 
    -> Need to create a a run script based on the: .devcontainer/devcontainer.

Source: https://bkedwards.github.io/comp423-course-notes/tutorials/rust-setup/
name: A descriptive name for your dev container.

image: The Docker image to use, in this case, the latest version of a Rust environment. Microsoft maintains a collection of base images for many programming language environments, but you can also create your own!

customizations: Adds useful configurations to VS Code, like installing the Rust extension. When you search for VSCode extensions on the marketplace, you will find the string identifier of each extension in its sidebar. Adding extensions here ensures other developers on your project have them installed in their dev containers automatically.

postCreateCommand: Commands to be executed after the container is created. In our case, there is nothing to be run after creation.