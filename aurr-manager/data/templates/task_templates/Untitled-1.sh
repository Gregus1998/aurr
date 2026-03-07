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
|     Version 1.0                                               |
|     By: Jonas Sørensen                                        |
+---------------------------------------------------------------+


+--------+
| Syntax |
+--------+
    ./aurr <Switch> <Optional Arguments> 


+-------------------------------+--------------------------------------------------+
|   SWITCH                      |  DESCRIPTION                                     |
+-------------------------------+--------------------------------------------------+
    run-local-setup             // Switch to run a local setup in the current folder. 
                                    Only do this if you export Aurr somewhere. 
                                    No Failchecks. Is called -> Does a jobb!

    Upload                      // Upload a local tool/resource to the cloud
                                    Requires: 
                                        --account-key
                                    
                                    Call Options:
                                        - upload tools::<tool_name>
                                        - upload file <filepath1> <filepath2> .. <filepath_N>

    Download                    // Download the content of a cloud resource to a local filepath
                                    Requires: 
                                        --account key
                                    
                                    Call Options: 
                                        - download <container::blob> <local_path>

    status                      // List detailed information about a target cloud resource
                                    Requires:
                                        --account-key

    sync                        // Pull Sync the content of a cloud storage to a local path.
                                    Requires: 
                                        --account-key
                                    
                                    Call Options:
                                        sync <some_cloud_resource> <local_dir>

    Cloudify                    // Upload a local tool / resource and return a download URL
                                    Requires: 
                                        --account-key

                                    Call Options:
                                        - upload tools::<tool_name>
                                        - upload file <filepath1> <filepath2> .. <filepath_N> 

    Grant-Access                // Provides access to a cloud resource already in cloud. 
                                    Requires: --account-key

    Run-Case                    // Process a case-template. 
                                    Requires: --account-key
                                    
                                    Call Options:
                                        - run-case <Path/To/CaseTemplate.json>
                                        - --case=<Path/To/CaseTemplate.json> run-case

                                    Can be used to full automate a wide set of remote tasks.
                                        - Collect Memory
                                        - Take traige
                                        - Image Disk
                                        - Run Custom tools
                                        - Run Scripts

                                    To set up a custom case-template. Read docs <insert path to guide>

    ls <ls-option>              // Switch to list information about different elements of the framework. 
                                    ls-options:
                                        - tools::<filter>        // List all available tools based on the provided config
                                        - case::<filter>         // List information from the provided case - This prints task tempalte aswell!
                                        - config                 // List current running config. Same as "print-config"
                                        - container::<filter>    // List available container for the specific azure storage account
                                        - blobs (TODO)           // TODO
                                        - csm                    // List the status of each of the applied CloudServiceManagers and check if it is reachable
    
    print-config                //prints the current running config.


+---------------------------+-------------------------------+-------------------------------------------------------------+
|   OPTIONAL-ARGUMENT       |   DEFAULT_VALUES              |   DESCRIPTION                                               |
+---------------------------+-------------------------------+-------------------------------------------------------------+
    --account-key=<Key>                                     // Needer for all interaction with the cloud. 
    --config=<path>         | ./Config.toml                 // Path to the Config.toml -> Default path is ./Config.toml
    --use-default=<bool>    | true                          // Use to run whatever switch with default parameters.  
    --case=<path>                                           // If you want to run a case template. Provide the path to the case template
    --tool-config=<path>    | ./data/templates/tools.json   // Path to tool configuration <INSERT DEFAULT PATH HERE>
    --entry=<VALUE>                                         // ENTRY in the tool-configuration to use. need to be passed together with '--tool-config'
    --full-info|list-all                                    // Used to list more information when ls is used.   

+----------+
| Examples |
+----------+

# Cmdline to run a local setup. This will create the needed folders and unpack some basic files: 
    -> ./aurr --run-local-setup                                                 //Runs a local setup. Should make it easy to pass the tool around

# Examples of Cloudify  
    -> ./aurr --account-key=<key> cloudify tools::<tool_name>                   // Upload a tool to the cloud by config and tool config.    
    -> ./aurr --account-key=<key> cloudify path/to/file1 path/to/file2          //Uploads the targeted files to the cloud.  

# List tools: 
    -> ./aurr ls tools                                                          // Lists all tools based on the provided Tools.json file
    -> ./aurr ls tools::<tool_name>                                             // Lists only information about the specified tool "Surge-Collect"
    -> ./aurr ls tools::<tool_name> --list-all                                  // List all available information.

# List blobs in a container (AZURE CLOUD): 
    -> ./aurr ls container                                                      // Lists all containers in the cloud-root 
    -> ./aurr ls container::upload                                              // Lists content of a specific container. "upload" can be changed to any container in the cloud-root 

# Example of run a case: 
    -> ./aurr --account-key=<key> run-case <case_path>                          // Runs a set of TaskTemplates based on a case_tempalte. 
