use roxmltree::Document;
use std::collections::HashMap;

fn parse_config_xml(path: &str) -> HashMap<String, Vec<String>> {
    let mut config = HashMap::new();
    let content = std::fs::read_to_string(path).unwrap_or_default();
    if content.is_empty() { return config; }
    
    let doc = Document::parse(&content).unwrap();
    for node in doc.descendants() {
        if node.has_tag_name("param") {
            let name = node.attribute("name").unwrap();
            let value = node.attribute("value").unwrap();
            config.insert(name.to_string(), vec![value.to_string()]);
        } else if node.has_tag_name("paramlist") {
            let name = node.attribute("name").unwrap();
            let mut values = Vec::new();
            for item in node.children() {
                if item.has_tag_name("item") {
                    values.push(item.attribute("value").unwrap().to_string());
                }
            }
            config.insert(name.to_string(), values);
        }
    }
    config
}

fn main() {
    println!("{:?}", parse_config_xml("src/models/mac/bypass/bypassnem.xml"));
}
