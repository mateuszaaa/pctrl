use anyhow;
use env_logger::{self, Builder};
use log::{debug, error, info, LevelFilter};
use pulsectl::controllers::{types::DeviceInfo, DeviceControl, SinkController, SourceController};

use clap::{Parser, ValueEnum};

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(long, value_enum)]
    target: InputOutput,

    #[arg(long, value_enum)]
    action: Action,

    #[arg(long, action = clap::ArgAction::SetTrue)]
    verbose: bool,
}

enum Direction {
    Forward,
    Backward,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum InputOutput {
    Input,
    Output,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Action {
    Next,
    Prev,
    Mute,
    Inc,
    Dec,
}

fn next_dev(
    mut controller: Box<dyn DeviceControl<DeviceInfo>>,
    direction: Direction,
) -> anyhow::Result<()> {
    let devices = controller.list_devices().unwrap_or_default();
    if let Ok(default) = controller.get_default_device() {
        debug!("Default device found {:?}", default.name);

        let next_device = match direction {
            Direction::Forward => devices
                .iter()
                .cycle()
                .skip_while(|d| d.index != default.index)
                .skip(1)
                .next(),
            Direction::Backward => devices
                .iter()
                .rev()
                .cycle()
                .skip_while(|d| d.index != default.index)
                .skip(1)
                .next(),
        };

        match next_device {
            Some(ref d) if d.index == default.index => {
                info!("There is only one sink availble, doing nothing");
            }
            Some(ref d) => {
                info!("Setting default device to: {:?}", d.name);
                let name = d.name.clone().unwrap_or_default();
                controller.set_default_device(name.as_ref())?;
            }
            None => {
            }
        }
    } else {
        debug!("Default device not set");
        if let Some(ref d) = devices.iter().next() {
            info!("Setting default device to: {:?}", d.name);
            let name = d.name.clone().unwrap_or_default();
        }
    }
    Ok(())
}

fn prev_sink(mut controller: SinkController) -> anyhow::Result<()> {
    let devices = controller.list_devices().unwrap_or_default();
    if let Ok(default) = controller.get_default_device() {
        debug!("Default device found {:?}", default.name);
        let prev_device = devices
            .iter()
            .rev()
            .cycle()
            .skip_while(|d| d.index != default.index)
            .skip(1)
            .next();

        match prev_device {
            Some(ref d) if d.index == default.index => {
                info!("There is only one sink availble, doing nothing");
                // do nothing
            }
            Some(ref d) => {
                info!("Setting default device to: {:?}", d.name);
                let name = d.name.clone().unwrap_or_default();
                controller.set_default_device(name.as_ref())?;
            }
            None => {
                // do nothin
            }
        }
    } else {
        debug!("Default device not set");
        let next_device = devices.iter().next();

        if let Some(ref d) = next_device {
            info!("Setting default device to: {:?}", d.name);
            let name = d.name.clone().unwrap_or_default();
            controller.set_default_device(name.as_ref())?;
        }
    }
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let mut builder = Builder::from_default_env();
    let level = if cli.verbose {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };

    builder.filter(None, level).init();

    let mut controller: Box<dyn DeviceControl<DeviceInfo>> = match cli.target {
        InputOutput::Input => Box::new(SourceController::create()?),
        InputOutput::Output => Box::new(SinkController::create()?),
    };

    let devices = controller.list_devices().unwrap_or_default();
    if devices.is_empty() {
        error!("No devices found");
        return Ok(());
    }else{
        for d in devices.iter(){
            debug!("Found devices: {:?}", d.name);
        }
    }

    match cli.action {
        Action::Next => {
            next_dev(controller, Direction::Forward)?;
        }
        Action::Prev => {
            next_dev(controller, Direction::Backward)?;
        }
        Action::Mute => {
            if let Ok(default) = controller.get_default_device() {
                controller.set_device_mute_by_index(default.index, !default.mute);
            }
        }
        Action::Inc => {
            if let Ok(default) = controller.get_default_device() {
                controller.increase_device_volume_by_percent(default.index, 0.05);
            }
        }
        Action::Dec => {
            if let Ok(default) = controller.get_default_device() {
                controller.decrease_device_volume_by_percent(default.index, 0.05);
            }
        }
    };

    Ok(())
}
