use clap::{App, Arg, SubCommand};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::{env, fs};

extern crate confignoble;

mod simulate;

use confignoble::compilation::{
    build_sel4, resolve_sel4_source, ResolvedSeL4Source, SeL4BuildMode, SeL4BuildOutcome,
};

/// Walk up the directory tree from `start_dir`, looking for "sel4.toml"
fn find_sel4_toml(start_dir: &Path) -> Option<PathBuf> {
    assert!(
        start_dir.is_dir(),
        "{} is not a directory",
        start_dir.display()
    );

    let toml = start_dir.join("sel4.toml");
    if toml.exists() {
        return Some(toml);
    } else {
        match start_dir.parent() {
            Some(d) => find_sel4_toml(d),
            None => None,
        }
    }
}

pub struct BuildParams {
    arch: String,
    platform: String,
    is_debug: bool,
    is_verbose: bool,
}

enum Execution {
    Build(BuildParams),
    Simulate(BuildParams),
}

trait AppExt {
    fn add_build_params(self) -> Self;
}

impl<'a, 'b> AppExt for App<'a, 'b> {
    fn add_build_params(self) -> Self {
        self.arg(
            Arg::with_name("arch")
                .short("a")
                .long("arch")
                .value_name("ARCH")
                .required(true)
                .help("seL4 architecture target, like x86_64 or arm"),
        )
        .arg(
            Arg::with_name("platform")
                .short("p")
                .long("platform")
                .value_name("PLATFORM")
                .required(true)
                .help("seL4 platform, like pc99 or imx6 or sabre"),
        )
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .takes_value(false)
                .help("verbose"),
        )
        .arg(
            Arg::with_name("debug")
                .long("debug")
                .takes_value(false)
                .conflicts_with("release")
                .help("build with debug configuration"),
        )
        .arg(
            Arg::with_name("release")
                .long("release")
                .takes_value(false)
                .conflicts_with("debug")
                .help("build with release configuration"),
        )
    }
}

impl Execution {
    fn get_or_run_help() -> Self {
        // TODO - naming / piping / phrasing
        let mut app = App::new("cotransport")
            .version("0.1.0")
            .about("builds and runs seL4 applications")
            .subcommand(SubCommand::with_name("build").add_build_params())
            .subcommand(SubCommand::with_name("simulate").add_build_params());
        let matches = app.clone().get_matches();

        fn parse_build_params(matches: &clap::ArgMatches<'_>) -> BuildParams {
            let is_verbose = matches.is_present("verbose");
            let is_debug = !matches.is_present("release");
            let arch = matches
                .value_of("arch")
                .expect("Missing required arch argument")
                .to_owned();
            let platform = matches
                .value_of("platform")
                .expect("Missing required platform argument")
                .to_owned();
            BuildParams {
                arch,
                platform,
                is_debug,
                is_verbose,
            }
        }

        if let Some(matches) = matches.subcommand_matches("build") {
            Execution::Build(parse_build_params(matches))
        } else if let Some(matches) = matches.subcommand_matches("simulate") {
            Execution::Simulate(parse_build_params(matches))
        } else {
            let _ = app.print_help();
            panic!()
        }
    }
}

fn main() {
    // TODO - Exit code management
    let e = Execution::get_or_run_help();
    match e {
        Execution::Build(b) => {
            let (outcome, _config) = &build_kernel(&b);
            print_kernel_paths(outcome);
        }
        Execution::Simulate(b) => {
            let (outcome, config) = build_kernel(&b);
            if let SeL4BuildOutcome::Kernel{
                kernel_path,
                root_image_path,
                ..
            }  = outcome {
                simulate::run_simulate(&b, &kernel_path, &root_image_path, &config).expect("Simulation failed");
            } else {
                panic!("Should not have built a static lib when a kernel is expected")
            }

            panic!("simulate subcommand not yet supported");
        }
    }
}

fn build_kernel(
    build_params: &BuildParams,
) -> (
    SeL4BuildOutcome,
    confignoble::model::contextualized::Contextualized,
) {
    let target_arch = build_params.arch.to_owned();
    let sel4_platform = build_params.platform.to_owned();
    let is_debug = build_params.is_debug;
    let pwd = &env::current_dir().unwrap();
    let config_file_path = find_sel4_toml(&pwd).unwrap_or_else(|| {
        let cfg = env::var("SEL4_CONFIG_PATH")
            .expect("sel4.toml was not found in the current tree, and SEL4_CONFIG was not set");
        PathBuf::from(&cfg)
    });
    let config_file_dir = config_file_path
        .parent()
        .expect("Can't get parent of config file path");

    let config_content = fs::read_to_string(&config_file_path).expect(&format!(
        "Can't read config file: {}",
        config_file_path.display()
    ));

    let config = confignoble::model::contextualized::Contextualized::from_str(
        &config_content,
        target_arch.to_owned(),
        is_debug,
        Some(sel4_platform.to_owned()),
        Some(config_file_dir),
    )
    .expect("Can't process config");

    let out_dir = config_file_dir.join("target").join("sel4");

    let ResolvedSeL4Source {
        kernel_dir,
        tools_dir,
        util_libs_dir,
    } = resolve_sel4_source(&config.sel4_source, &out_dir.join("source"))
        .expect("resolve sel4 source");

    let root_task = config.build.root_task.as_ref()
        .expect(&format!("root task information, particularly a root_task_image path must be supplied in [build.platform.profile], here [build.{}.{}]",
        config.context.platform, if config.context.is_debug { "debug"} else { "release"})).clone();

    if let Some(make_root_task_command) = root_task.make_command {
        // Build the root task
        let mut build_cmd = Command::new("sh");
        build_cmd
            .arg("-c")
            .arg(&make_root_task_command)
            .current_dir(&config_file_dir)
            .env("SEL4_CONFIG_PATH", &config_file_path)
            .env("SEL4_PLATFORM", &config.context.platform)
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());

        println!(
            "Running root task build command:\n    SEL4_CONFIG_PATH={} SEL4_PLATFORM={} {}",
            config_file_path.display(),
            &config.context.platform,
            &make_root_task_command
        );
        let output = build_cmd.output().expect("Failed to execute build command");
        assert!(output.status.success());
    } else {
        println!("No make_root_task command supplied, skipping an explicit build for it.")
    }

    // Build the kernel and output images
    (
        build_sel4(
            &out_dir.join("build"),
            &kernel_dir,
            &tools_dir,
            &util_libs_dir,
            &config,
            SeL4BuildMode::Kernel,
        ),
        config,
    )
}

/// Print out the kernel-build-variant paths,
/// panic if the wrong variant
fn print_kernel_paths(outcome: &SeL4BuildOutcome) {
    match outcome {
        SeL4BuildOutcome::StaticLib { .. } => {
            panic!("Should not be making a static lib when a kernel is expected")
        }
        SeL4BuildOutcome::Kernel {
            build_dir,
            kernel_path,
            root_image_path,
        } => {
            println!("{}", build_dir.display());
            println!("{}", kernel_path.display());
            if let Some(rip) = root_image_path {
                println!("{}", rip.display());
            }
        }
    }
}
