use std::env;
use clap::Parser;
use roxmltree::Document;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(help = "URL of XML configuration file.")]
    config_url: String,

    #[arg(short = 'd', long = "daemonize", help = "Run in the background.")]
    daemonize: bool,
    
    #[arg(short = 'l', long = "loglevel", default_value_t = 2, help = "Set initial log level [0,4].")]
    loglevel: u8,
}

fn main() {
    let args = Args::parse();
    
    println!("Starting emane-rs with config: {}", args.config_url);
    
    // Parse the platform.xml
    let xml_content = std::fs::read_to_string(&args.config_url).expect("Failed to read config file");
    let doc = Document::parse(&xml_content).expect("Failed to parse XML");
    
    // TODO: build the orchestrator and NEMs
    for node in doc.descendants().filter(|n| n.has_tag_name("nem")) {
        let id = node.attribute("id").unwrap_or("0").parse::<u16>().unwrap_or(0);
        let name = node.attribute("name").unwrap_or("unknown");
        println!("Found NEM id: {}, name: {}", id, name);
    }
}
