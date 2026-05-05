use std::fs;
use std::path::Path;

pub type LoadError = Box<dyn std::error::Error>;

pub fn load(_path: &Path) -> Result<spec::ModelSpec, LoadError> {
    todo!("stub: read RON from path, deserialize to spec::ModelSpec — see docs/plan.md Lab 0b")
}

#[allow(dead_code)]
fn _read_file(path: &Path) -> Result<String, std::io::Error> {
    fs::read_to_string(path)
}
