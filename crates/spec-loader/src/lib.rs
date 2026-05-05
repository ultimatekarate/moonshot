use std::fs;
use std::path::Path;

pub type LoadError = Box<dyn std::error::Error>;

pub fn load(path: &Path) -> Result<spec::ModelSpec, LoadError> {
    let text = fs::read_to_string(path)?;
    let spec = ron::from_str(&text)?;
    Ok(spec)
}
