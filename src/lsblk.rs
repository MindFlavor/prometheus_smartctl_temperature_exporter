use log::trace;
use serde::Deserialize;
use std::error::Error;
use std::process::Command;

#[derive(Debug, Clone, Deserialize)]
struct Lsblk {
    pub blockdevices: Vec<BlockDevice>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BlockDevice {
    pub name: String,
    #[serde(rename = "maj:min")]
    pub maj_min: String,
    pub rm: bool,
    pub size: String,
    pub ro: bool,
    #[serde(rename = "type")]
    pub block_device_type: String,
    pub model: Option<String>,
    pub serial: Option<String>,
    pub wwn: Option<String>,
    pub children: Option<Vec<BlockDevice>>,
}

impl BlockDevice {
    pub fn signature(&self) -> String {
        match (&self.model, &self.serial) {
            (Some(model), Some(serial)) => format!("{}_{}", model, serial),
            _ => self.name.to_owned(),
        }
    }
}

pub fn get_block_devices() -> Result<Vec<BlockDevice>, Box<dyn Error + Send + Sync>> {
    let output = Command::new("lsblk")
        .arg("-J")
        .arg("-o")
        .arg("NAME,MAJ:MIN,RM,SIZE,RO,TYPE,MODEL,SERIAL,WWN")
        .output()?;
    let output_stdout_str = String::from_utf8(output.stdout)?;
    trace!("output_stdout_str == {}", output_stdout_str,);
    let output_stderr_str = String::from_utf8(output.stderr)?;
    trace!("output_stderr_str == {}", output_stderr_str,);

    let lsblk: Lsblk = serde_json::from_str(&output_stdout_str)?;

    Ok(lsblk.blockdevices)
}
