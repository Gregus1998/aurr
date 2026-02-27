use clap::{Parser, Subcommand};


#[derive(Parser)]
#[command(name = "Aurr")]
#[command(about = "Test Aurr clap cli")]
struct Cli{

    // Cmdlines variables
    #[arg(long, global = true, default_value = "./Config.toml")]
    config: Option<String>,

    #[arg(long, global = true)]
    key: Option<String>,

    #[arg(long, global = true, default_value = "./log/")]
    log_dir:Option<String>,

    #[command(subcommand)]
    switch: Switch,

}


#[derive(Subcommand)]
enum Switch {

    LocalSetup,

    Upload {

        #[arg()]
        local_path:String,
        
        #[arg()]
        remote_path:String
    },

    Download {
        #[arg()]
        remote_path:String,
        
        #[arg()]
        local_path:String
    },

    Cloudify,

    Sync {
        #[arg()]
        remote_path:String,
        
        #[arg()]
        local_path:String
    },

    Run {
        #[command(subcommand)]
        obj:AurrObjects
    },
     


    #[command(about = "A switch to list information about different objects")]
    Ls {
        #[command(subcommand)]
        category:AurrObjects,
    }
}

#[derive(Subcommand)]
enum AurrObjects{
    Container {
        #[arg()]
        con:String
    },

    Config,

    Tool {
        #[arg()]
        filter:Option<String>
    },

    Case {
        #[arg()]
        filter:String
    },

    Csm {
        filter:Option<String>
    },
}


impl Cli{

    pub async fn init() -> Result<(), Box<dyn std::error::Error>>{
        let cli = Cli::parse();

        if let Some(config) = &cli.config {
        println!("Using config file: {}", config);
        }

        match cli.switch{

        Switch::Ls { category} => todo!(),

        Switch::LocalSetup => todo!(),

        Switch::Upload {local_path, remote_path} => todo!(),
        
        Switch::Download { remote_path, local_path } => todo!("Add support for download"),

        Switch::Sync { remote_path, local_path } => todo!("add support for sync"),

        Switch::Cloudify => todo!(),

        Switch::Run { obj } => todo!(),

    }

        Ok(())

    }
}




pub async fn main(){

    Cli::init().await.unwrap();

}