use anyhow::anyhow;
use either::Either;
use env_logger::{self, Builder};
use log::{debug, error, info, warn, LevelFilter};
use pulsectl::controllers::{
    types::{ApplicationInfo, DeviceInfo},
    AppControl, DeviceControl, SinkController, SourceController,
};

use clap::{Parser, ValueEnum};
mod fs_helpers;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(long, value_enum)]
    target: InputOutput,

    #[arg(long, value_enum)]
    action: Option<Action>,

    #[arg(long, action = clap::ArgAction::SetTrue)]
    verbose: bool,

    #[arg(long, value_enum)]
    status: Option<Status>,

    #[arg(long)]
    prev: Option<u32>,
}

fn get_default_device(
    controller: &mut Box<Controller>,
    input_output: InputOutput,
) -> anyhow::Result<DeviceInfo> {
    if let Some(idx) = fs_helpers::read_device_index(input_output)?{
        if let Ok(device) = controller.get_device_by_index(idx) {
            debug!("Device with index #{} found: {:?}", idx, device.name);
            Ok(device)
        }else{
            warn!("Device with index {} not found - figuring out new default device", idx);
            //TODO: try to fetch default device first from pulse audio
            let dev = controller
                .list_devices()?
                .iter()
                .filter(ignore_monitor_devs)
                .cloned()
                .next()
                .ok_or(anyhow!("No devices found"))?;
            fs_helpers::write_device_index(input_output, dev.index)?;
            Ok(dev.clone())
        }
    }else{
        debug!("No previous state stored");
        let dev = controller
            .list_devices()?
            .first()
            .cloned()
            .ok_or(anyhow!("No devices found"))?;
        fs_helpers::write_device_index(input_output, dev.index)?;
        Ok(dev.clone())
    }
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

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum Status {
    Muted,
    Volume,
    Name,
    Desc,
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
}

fn ignore_monitor_devs(d: &&DeviceInfo) -> bool {
    !d.name
        .clone()
        .unwrap_or_default()
        .to_lowercase()
        .contains("monitor")
}

fn next_dev(
    controller: &mut Box<Controller>,
    direction: Direction,
    prev: DeviceInfo,
    input_output: InputOutput,
) -> anyhow::Result<()> {
    let devices = controller.list_devices().unwrap_or_default();

    let iter: Either<_,_> = match direction {
        Direction::Forward => Either::Left(devices.iter()),
        Direction::Backward => Either::Right(devices.iter().rev()),
    };

    for d in devices.iter()
        .filter(ignore_monitor_devs)
    {
        debug!("Found devices: {:?}", d.index);
    }

    let next_device = iter
        .cycle()
        .take(devices.len()*2)
        .skip_while(|d| d.index != prev.index)
        .skip(1)
        .filter(ignore_monitor_devs)
        .next()
        .expect("At least one device should be available at this point");

    info!("Setting default device to: {:?}", next_device.index);
    controller.set_default(next_device.index)?;
    controller.set_default_device(next_device.name.clone().unwrap_or_default().as_ref())?;
    fs_helpers::write_device_index(input_output, next_device.index)?;
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

    let prev_device = get_default_device(&mut controller, cli.target)?;

    match cli.action {
        Some(Action::Next) => {
            next_dev(& mut controller, Direction::Forward, prev_device, cli.target)?;
        }
        Some(Action::Prev) => {
            next_dev(& mut controller, Direction::Backward, prev_device, cli.target)?;
        }
        Some(Action::Mute) => {
            controller.set_device_mute_by_index(prev_device.index, !prev_device.mute);
        }
        Some(Action::Inc) => {
            controller.increase_device_volume_by_percent(prev_device.index, 0.05);
        }
        Some(Action::Dec) => {
            controller.decrease_device_volume_by_percent(prev_device.index, 0.05);
        }
        None => {}
    };

    if let Some(status) = cli.status{
        let dev = fs_helpers::read_device_index(cli.target)?;
        let info = controller.get_device_by_index(dev.unwrap())?;
        match status{
            Status::Muted => print!("{}", info.mute),
            Status::Volume => print!("{}", info.volume),
            Status::Name => print!("{}", info.name.clone().unwrap_or_default()),
            Status::Desc => print!("{}", info.description.clone().unwrap_or_default()),
        }
    }

    Ok(())
}
