use std::{path::PathBuf, process};

use clap::Parser;
use tracing::error;

use crate::{ENV_KEY, ENV_VALUE};

#[derive(Parser, Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
#[command(version, about)]
pub struct Args {
    /// Docker update interval in ms, minimum effectively 1000
    #[clap(short = 'd', value_name = "ms", default_value_t = 1000)]
    pub docker_interval: u32,

    /// Remove timestamps from Docker logs
    #[clap(short = 't')]
    pub timestamp: bool,

    /// Attempt to colorize the logs, conflicts with "-r"
    #[clap(short = 'c', conflicts_with = "raw")]
    pub color: bool,

    /// Show raw logs, default is to remove ansi formatting, conflicts with "-c"
    #[clap(short = 'r', conflicts_with = "color")]
    pub raw: bool,

    /// Show self when running as a docker container
    #[clap(short = 's')]
    pub show_self: bool,

    /// Don't draw gui - for debugging - mostly pointless
    #[clap(short = 'g')]
    pub gui: bool,

    /// Docker host, defaults to `/var/run/docker.sock`
    #[clap(long, short = None)]
    pub host: Option<String>,

    /// Force use of docker cli when execing into containers
    #[clap(long="use-cli", short = None)]
    pub use_cli: bool,

    /// Directory for saving exported logs, defaults to `$HOME`
    #[clap(long="save-dir", short = None)]
    pub save_dir: Option<String>,

    /// Base URL for opening the container in a browser
    #[clap(long = "base-url-map", short = 'm', value_delimiter = ' ', num_args = 1..)]
    pub base_url_map: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct BaseUrlMap {
    pub name: Option<String>,
    pub image: Option<String>,
    pub label: Option<String>,
    pub base_url: String,
}

#[derive(Debug, Clone)]
#[allow(clippy::struct_excessive_bools)]
pub struct CliArgs {
    pub color: bool,
    pub docker_interval: u32,
    pub gui: bool,
    pub host: Option<String>,
    pub in_container: bool,
    pub save_dir: Option<PathBuf>,
    pub raw: bool,
    pub show_self: bool,
    pub timestamp: bool,
    pub use_cli: bool,
    pub base_url_map: Option<Vec<BaseUrlMap>>,
}

impl CliArgs {
    /// An ENV is set in the ./containerised/Dockerfile, if this is ENV found, then sleep for 250ms, else the container, for as yet unknown reasons, will close immediately
    /// returns a bool, so that the `update_all_containers()` won't bother to check the entry point unless running via a container
    fn check_if_in_container() -> bool {
        if let Ok(value) = std::env::var(ENV_KEY) {
            if value == ENV_VALUE {
                return true;
            }
        }
        false
    }

    fn parse_base_url_map_input(input: &str) -> BaseUrlMap {
        let mut name = None;
        let mut image = None;
        let mut label = None;

        let mut split = input.splitn(3, ';');
        let value_type = split.next().map(std::string::ToString::to_string);

        if value_type == Some("name".to_string()) {
            name = split.next().map(std::string::ToString::to_string);
        }
        if value_type == Some("image".to_string()) {
            image = split.next().map(std::string::ToString::to_string);
        }
        if value_type == Some("label".to_string()) {
            label = split.next().map(std::string::ToString::to_string);
        }

        if name.is_none() && image.is_none() && label.is_none() {
            error!("Couldn't parse type, \"-m\" argument needs to be in the format \"name|image|label;value;base_url\"");
            process::exit(1)
        }

        let Some(base_url) = split.next().map(std::string::ToString::to_string) else {
            error!(
                "Couldn't parse url, \"-m\" argument needs to be in the format \"name|image|label;value;input_url\""
            );
            process::exit(1)
        };

        BaseUrlMap {
            name,
            image,
            label,
            base_url,
        }
    }

    /// Parse cli arguments
    pub fn new() -> Self {
        let args = Args::parse();

        let logs_dir = args.save_dir.map_or_else(
            || directories::BaseDirs::new().map(|base_dirs| base_dirs.home_dir().to_owned()),
            |logs_dir| Some(std::path::Path::new(&logs_dir).to_owned()),
        );

        let base_url_map = args.base_url_map.map(|b| {
            b.iter()
                .map(|s| Self::parse_base_url_map_input(s))
                .collect()
        });

        // Quit the program if the docker update argument is 0
        // Should maybe change it to check if less than 100
        if args.docker_interval == 0 {
            error!("\"-d\" argument needs to be greater than 0");
            process::exit(1)
        }

        Self {
            color: args.color,
            docker_interval: args.docker_interval,
            use_cli: args.use_cli,
            gui: !args.gui,
            host: args.host,
            in_container: Self::check_if_in_container(),
            save_dir: logs_dir,
            raw: args.raw,
            show_self: !args.show_self,
            timestamp: !args.timestamp,
            base_url_map,
        }
    }
}
