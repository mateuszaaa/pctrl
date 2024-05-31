use anyhow;
use env_logger::{self, Builder};
use log::{debug, error, info, LevelFilter};
use pulsectl::controllers::{
    types::{ApplicationInfo, DeviceInfo},
    AppControl, DeviceControl, SinkController, SourceController,
};

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

    #[arg(long)]
    prev: Option<u32>,
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

trait GenericController<T, K>: DeviceControl<T> + AppControl<K> + SetDefault {}
impl<T, K, L> GenericController<K, L> for T
where
    T: DeviceControl<K>,
    T: AppControl<L>,
    T: SetDefault,
{
}
type Controller = dyn GenericController<DeviceInfo, ApplicationInfo>;

/// sets given index for all runntine applications
pub trait SetDefault {
    fn set_default(&mut self, index: u32) -> anyhow::Result<()>;
    fn get_default(&mut self) -> anyhow::Result<u32>;
}

impl<T> SetDefault for T
where
    T: AppControl<ApplicationInfo>,
{
    fn set_default(&mut self, index: u32) -> anyhow::Result<()> {
        for app in self.list_applications()? {
            self.move_app_by_index(app.index, index)?;
        }
        Ok(())
    }

    fn get_default(&mut self) -> anyhow::Result<u32> {
        todo!()
    }
}

fn next_dev(
    mut controller: Box<Controller>,
    direction: Direction,
    prev: Option<u32>,
) -> anyhow::Result<()> {
    let devices = controller.list_devices().unwrap_or_default();

    let filter_out_monitor_devs = |d: &&DeviceInfo| {
        !d.name
            .clone()
            .unwrap_or_default()
            .to_lowercase()
            .contains("monitor")
    };

    let default_device = prev
        .and_then(|index| devices.iter().cloned().find(|d| d.index == index))
        .or(controller.get_default_device().ok());

    if let Some(prev) = default_device {
        debug!("Default device found #{} {:?}", prev.index, prev.name);

        let next_device = match direction {
            Direction::Forward => devices
                .iter()
                .filter(filter_out_monitor_devs)
                .cycle()
                .skip_while(|d| d.index != prev.index)
                .skip(1)
                .next(),
            Direction::Backward => devices
                .iter()
                .filter(filter_out_monitor_devs)
                .rev()
                .cycle()
                .skip_while(|d| d.index != prev.index)
                .skip(1)
                .next(),
        };

        match next_device {
            Some(ref d) if d.index == prev.index => {
                info!("There is only one sink availble, doing nothing");
            }
            Some(ref d) => {
                info!("Setting default device to: {:?}", d.index);
                controller.set_default(d.index)?;
            }
            None => {}
        }
    } else {
        debug!("Default device not set");
        if let Some(ref d) = devices.iter().filter(filter_out_monitor_devs).next() {
            info!("Setting default device to: {:?}", d.index);
            controller.set_default(d.index)?;
        } else {
            info!("No available devices");
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

    let mut controller: Box<Controller> = match cli.target {
        InputOutput::Input => Box::new(SourceController::create()?),
        InputOutput::Output => Box::new(SinkController::create()?),
    };

    let devices = controller.list_devices().unwrap_or_default();
    if devices.is_empty() {
        error!("No devices found");
        return Ok(());
    } else {
        for d in devices.iter() {
            debug!("Found devices: {:?}", d.name);
        }
    }

    match cli.action {
        Action::Next => {
            next_dev(controller, Direction::Forward, cli.prev)?;
        }
        Action::Prev => {
            next_dev(controller, Direction::Backward, cli.prev)?;
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
