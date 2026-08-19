use roxmltree::{Document, Node};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;

#[derive(Debug, Clone)]
pub struct ParamValues {
    pub values: Vec<String>,
}

pub type ParamMap = HashMap<String, ParamValues>;

#[derive(Debug, Clone)]
pub struct LayerConfig {
    pub layer_type: String,
    pub definition_file: Option<String>,
    pub params: ParamMap,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NemType {
    Structured,
    Unstructured,
}

#[derive(Debug, Clone)]
pub struct NemConfig {
    pub id: u16,
    pub definition_file: Option<String>,
    pub external_transport: bool,
    pub nem_type: NemType,
    pub params: ParamMap,
    pub layers: Vec<LayerConfig>,
}

#[derive(Debug, Clone)]
pub struct PlatformConfig {
    pub name: Option<String>,
    pub nems: Vec<NemConfig>,
    pub params: ParamMap,
}

#[derive(Debug, Clone)]
pub enum ConfigurationError {
    ParseError(String),
    ValidationError(String),
    FileError(String),
}

impl std::fmt::Display for ConfigurationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ParseError(e) => write!(f, "Parse Error: {}", e),
            Self::ValidationError(e) => write!(f, "Validation Error: {}", e),
            Self::FileError(e) => write!(f, "File Error: {}", e),
        }
    }
}
impl std::error::Error for ConfigurationError {}

fn parse_params(node: Node<'_, '_>) -> Result<ParamMap, ConfigurationError> {
    let mut params = ParamMap::new();
    for child in node.children().filter(|c| c.is_element()) {
        let name = child.tag_name().name();
        if name == "param" {
            let param_name = child.attribute("name").ok_or_else(|| ConfigurationError::ParseError("param missing name".to_string()))?;
            let param_value = child.attribute("value").unwrap_or("");
            if params.contains_key(param_name) {
                return Err(ConfigurationError::ParseError(format!("Duplicate configuration item '{}'", param_name)));
            }
            params.insert(param_name.to_string(), ParamValues { values: vec![param_value.to_string()] });
        } else if name == "paramlist" {
            let param_name = child.attribute("name").ok_or_else(|| ConfigurationError::ParseError("paramlist missing name".to_string()))?;
            if params.contains_key(param_name) {
                return Err(ConfigurationError::ParseError(format!("Duplicate configuration item '{}'", param_name)));
            }
            let mut values = Vec::new();
            for item in child.children().filter(|c| c.is_element() && c.tag_name().name() == "item") {
                values.push(item.attribute("value").unwrap_or("").to_string());
            }
            params.insert(param_name.to_string(), ParamValues { values });
        }
    }
    Ok(params)
}

fn overlay_params(base: &mut ParamMap, new_params: ParamMap) {
    for (k, v) in new_params {
        base.insert(k, v);
    }
}

pub fn parse_layer(file_path: &Path, node: Node<'_, '_>, expected_type: &str) -> Result<LayerConfig, ConfigurationError> {
    let mut config = LayerConfig {
        layer_type: expected_type.to_string(),
        definition_file: None,
        params: ParamMap::new(),
    };

    let definition = node.attribute("definition");
    
    if let Some(def) = definition {
        let def_path = file_path.parent().unwrap_or(Path::new("")).join(def);
        let content = fs::read_to_string(&def_path).map_err(|e| ConfigurationError::FileError(format!("Failed to read {}: {}", def_path.display(), e)))?;
        let doc = Document::parse(&content).map_err(|e| ConfigurationError::ParseError(e.to_string()))?;
        let root = doc.root_element();
        
        if root.tag_name().name() != expected_type {
             return Err(ConfigurationError::ParseError(format!("Document root node is not '{}'", expected_type)));
        }
        
        config.definition_file = Some(def.to_string());
        config.params = parse_params(root)?;
    }
    
    let inline_params = parse_params(node)?;
    overlay_params(&mut config.params, inline_params);
    
    Ok(config)
}

pub fn parse_nem(file_path: &Path, node: Node<'_, '_>) -> Result<NemConfig, ConfigurationError> {
    let id_str = node.attribute("id").ok_or_else(|| ConfigurationError::ParseError("nem missing id".to_string()))?;
    let id: u16 = id_str.parse().map_err(|_| ConfigurationError::ParseError(format!("invalid nem id: {}", id_str)))?;
    
    let mut config = NemConfig {
        id,
        definition_file: None,
        external_transport: node.attribute("transport") == Some("external"),
        nem_type: NemType::Structured,
        params: ParamMap::new(),
        layers: Vec::new(),
    };
    
    if node.attribute("type") == Some("unstructured") {
        config.nem_type = NemType::Unstructured;
    }

    let definition = node.attribute("definition");
    
    if let Some(def) = definition {
        let def_path = file_path.parent().unwrap_or(Path::new("")).join(def);
        let content = fs::read_to_string(&def_path).map_err(|e| ConfigurationError::FileError(format!("Failed to read {}: {}", def_path.display(), e)))?;
        let doc = Document::parse(&content).map_err(|e| ConfigurationError::ParseError(e.to_string()))?;
        let root = doc.root_element();
        
        if root.tag_name().name() != "nem" {
             return Err(ConfigurationError::ParseError("Document root node is not 'nem'".to_string()));
        }
        
        if root.attribute("type") == Some("unstructured") {
            config.nem_type = NemType::Unstructured;
        }
        
        config.definition_file = Some(def.to_string());
        config.params = parse_params(root)?;
        
        for child in root.children().filter(|c| c.is_element()) {
            let name = child.tag_name().name();
            if name != "param" && name != "paramlist" {
                 config.layers.push(parse_layer(&def_path, child, name)?);
            }
        }
    }
    
    let inline_params = parse_params(node)?;
    overlay_params(&mut config.params, inline_params);
    
    for child in node.children().filter(|c| c.is_element()) {
        let name = child.tag_name().name();
        if name != "param" && name != "paramlist" {
             let child_def = child.attribute("definition");
             
             if name == "shim" {
                 config.layers.push(parse_layer(file_path, child, name)?);
             } else {
                 let mut found_idx = None;
                 for (i, layer) in config.layers.iter().enumerate() {
                     if layer.layer_type == name {
                         found_idx = Some(i);
                         break;
                     }
                 }
                 
                 if let Some(idx) = found_idx {
                     let layer = &mut config.layers[idx];
                     if layer.definition_file.as_deref() == child_def {
                         let inline_layer_params = parse_params(child)?;
                         overlay_params(&mut layer.params, inline_layer_params);
                     } else {
                         return Err(ConfigurationError::ValidationError(format!("Unexpected definition for '{}'", name)));
                     }
                 } else {
                     config.layers.push(parse_layer(file_path, child, name)?);
                 }
             }
        }
    }
    
    if config.nem_type == NemType::Structured {
        let has_phy = config.layers.iter().any(|l| l.layer_type == "phy");
        let has_mac = config.layers.iter().any(|l| l.layer_type == "mac");
        let has_transport = config.layers.iter().any(|l| l.layer_type == "transport");
        
        if !has_phy || !has_mac || !has_transport {
             return Err(ConfigurationError::ValidationError(format!("NEM id {} is NOT properly configured. Missing phy|mac|transport", id)));
        }
    }
    
    Ok(config)
}

pub fn parse_platform(file_path: &Path) -> Result<PlatformConfig, ConfigurationError> {
    let content = fs::read_to_string(file_path).map_err(|e| ConfigurationError::FileError(format!("Failed to read {}: {}", file_path.display(), e)))?;
    let doc = Document::parse(&content).map_err(|e| ConfigurationError::ParseError(e.to_string()))?;
    let root = doc.root_element();
    
    if root.tag_name().name() != "platform" {
        return Err(ConfigurationError::ParseError("Document root node is not 'platform'".to_string()));
    }
    
    let mut config = PlatformConfig {
        name: root.attribute("name").map(|s| s.to_string()),
        nems: Vec::new(),
        params: parse_params(root)?,
    };
    
    for child in root.children().filter(|c| c.is_element()) {
        let name = child.tag_name().name();
        if name == "nem" {
            config.nems.push(parse_nem(file_path, child)?);
        }
    }
    
    Ok(config)
}
