use std::fs::{File, OpenOptions};
use std::io::{self, Read, Write};
use crate::InputOutput;
use anyhow;

const INPUT_STATE: &str = "/tmp/pctrl-input";
const OUTPUT_STATE: &str = "/tmp/pctrl-output";

pub (crate) fn read_device_index(input_output: InputOutput) -> anyhow::Result<Option<u32>>{
    let file_path = match input_output{
        InputOutput::Input => INPUT_STATE,
        InputOutput::Output => OUTPUT_STATE,
    };

    // Try to open the file with read and write permissions, creating it if it doesn't exist
    let mut file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(file_path)?;

    // Read the file content
    let mut content = String::new();
    file.read_to_string(&mut content)?;

    Ok(content.parse::<u32>().ok())
}

pub (crate) fn write_device_index(input_output: InputOutput, index: u32) -> anyhow::Result<()> {
    let file_path = match input_output{
        InputOutput::Input => INPUT_STATE,
        InputOutput::Output => OUTPUT_STATE,
    };
    // Try to open the file with read and write permissions, creating it if it doesn't exist
    let mut file = File::create(file_path)?;

    file.write_all(index.to_string().as_bytes())?;

    Ok(())
}
