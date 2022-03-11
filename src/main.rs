mod parallelism;
use crate::parallelism::*;

mod jobbers;
use crate::jobbers::checkerboard::*;
use crate::jobbers::gol::*;

use serde::{Serialize, Deserialize};


#[derive(Deserialize)]
struct ConfigToml {
    parallel_execution: Option<bool>,
    thread_count: Option<usize>,
    work_slice_len: Option<usize>,

    iterations: Option<usize>,
    width: Option<usize>,
    height: Option<usize>,
}


#[derive(Serialize)]
struct Config {
    parallel_execution: bool,
    thread_count: usize,
    work_slice_len: usize,

    iterations: usize,
    width: usize,
    height: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            parallel_execution: true,
            thread_count: num_cpus::get(),
            work_slice_len: 128 * 128,
             
            iterations: 1024,
            width: 3840,
            height: 2160,
        }
    }
}

impl From<ConfigToml> for Config {
    fn from(toml: ConfigToml) -> Self {
        let default = Config::default();
        Self {
            parallel_execution: toml.parallel_execution.unwrap_or(default.parallel_execution),
            thread_count: toml.thread_count.unwrap_or(default.thread_count),
            work_slice_len: toml.work_slice_len.unwrap_or(default.work_slice_len),

            iterations: toml.iterations.unwrap_or(default.iterations),
            width: toml.width.unwrap_or(default.width),
            height: toml.height.unwrap_or(default.height),
        }
    }
}

fn format_file_name_to_toml(file_name: &str) -> String {
    format!("{}.toml", file_name)
}

const HELP_STRING: &'static str = "
A tool for benchmarking CPUs using Conway's Game of Life.
It will run for the specified iteration count, simulating Game of Life generations.

You can configure the run parameters using the --generate-config and --use-config flags.

flags:
    --generate-config <filename?>
        to generate a config file used for benchmarking
        each parameter is what the app would've used on this machine when launching without flags
        the filename is optional, the tool will generate bench_conf.toml by default
    --use-config <filename?>
        to use a config file instead of default parameters
        any parameter be omitted if the default is preferred
        the filename is optional, the tool will search for bench_conf.toml by default
";

fn run() -> Result<(), String> {
    const DEFAULT_CONF_FILE_NAME: &'static str = "bench_conf";
    let mut config = Config::default();

    let mut args_iter = std::env::args().skip(1);
    if let Some(arg) = args_iter.next() {
        match arg.as_str() {
            "--help" => {
                println!("{}", HELP_STRING.trim());
                return Ok(());
            },
            "--generate-config" => {
                let file_name = format_file_name_to_toml(&args_iter.next().unwrap_or(String::from(DEFAULT_CONF_FILE_NAME)));
                let conf_serialized = toml::to_string(&config).unwrap();
                std::fs::write(&file_name, conf_serialized).map_err(|_| "Unable to write to file, exiting.")?;
                println!("Generated config file '{}', exiting.", file_name);
                return Ok(());
            },
            "--use-config" => {
                let file_name = format_file_name_to_toml(&args_iter.next().unwrap_or(String::from(DEFAULT_CONF_FILE_NAME)));
                let conf_seriazlied = std::fs::read_to_string(&file_name).map_err(|_| format!("Unable to find or read file {}, exiting.", file_name))?;
                let conf_deserialized: ConfigToml = toml::from_str(&conf_seriazlied).map_err(|_| "Unable to parse file's values, generate one to see available fields.")?;
                config = conf_deserialized.into();
                println!("Using config file '{}'", file_name);
            },
            _ => {
                println!("Unknown argument '{}', run with --help for more info.", arg);
                return Ok(());
            }
        }
    }

    println!(
        "Launching benchmark for {} iterations of a {}x{} buffer with {} thread(s)",
        config.iterations,
        config.width,
        config.height,
        if config.parallel_execution { config.thread_count } else { 1 },
    );
    
    let in_buf = Buffer::from_value_2d((config.width, config.height), GolCell::Dead);
    let exec: ExecutorSingleThread<_, _, CheckerboardJobber> = ExecutorSingleThread::new();
    let mut init_buf = Buffer::from_value_2d((config.width, config.height), GolCell::Dead);
    exec.compute(in_buf, &mut init_buf.data, CheckerboardConf { color_a: GolCell::Dead, color_b: GolCell::Alive, width: config.width });

    let exec_gol: Box<dyn Executor<GolCell, ()>> = if config.parallel_execution == true {
        Box::new(ExecutorParallel::new::<GameOfLifeJobber>(config.thread_count, config.work_slice_len))
    } else {
        Box::new(ExecutorSingleThread::<GolCell, (), GameOfLifeJobber>::new())
    };

    exec_gol.compute_iterations(config.iterations, init_buf, ());

    return Ok(());
}

fn main() {
    if let Err(message) = run() {
        print!("Fatal Error: {}", message);
    }
}
