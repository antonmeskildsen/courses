//! Then what is this??

use std::fs::create_dir;
use std::path::PathBuf;
use std::time::Duration;
use std::{env, fs};

use anyhow::Context;
use clap::{Parser, Subcommand};
use console::style;
use inquire::{InquireError, Select};
use notify::{RecommendedWatcher, RecursiveMode};
use notify_debouncer_mini::{
    new_debouncer_opt, DebounceEventResult, DebouncedEventKind, Debouncer,
};
use penguin::Server;

use courses::pipeline::Pipeline;
use courses::project::config::ProjectConfig;
use courses::project::Project;

mod setup;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Serve {
        #[arg(short, long)]
        path: Option<PathBuf>,
        #[arg(short, long, default_value = "dev")]
        mode: String,
    },
    Build {
        #[arg(short, long)]
        path: Option<PathBuf>,
        #[arg(short, long, default_value = "release")]
        mode: String,
    },
    Init {
        name: Option<String>,
        #[arg(short, long)]
        repository: Option<String>,
    },
    Test {},
    Publish {},
}

async fn cli_run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Build { path, mode } => {
            let path = path.unwrap_or(env::current_dir()?);

            print!("Configuring project...");
            let proj = Project::generate_from_directory(path.as_path())?;
            println!(" {}", style("done").green());

            let config_path = path.join("config.yml");
            let config_input = fs::read_to_string(config_path)?;
            let config: ProjectConfig = serde_yaml::from_str(&config_input)
                .context("Could not load project configuration")?;

            let mut pipeline = Pipeline::new(path.as_path(), mode, config, proj)?;
            pipeline.build_all(true)?;

            println!("🌟 Done.");
            Ok(())
        }
        Commands::Serve { path, mode } => {
            let path = path.unwrap_or(env::current_dir()?);

            print!("Configuring project...");
            let proj = Project::generate_from_directory(path.as_path())?;
            println!(" {}", style("done").green());

            let config_path = path.join("config.yml");
            let config_input = fs::read_to_string(config_path)?;
            let config: ProjectConfig = serde_yaml::from_str(&config_input)
                .context("Could not load project configuration")?;

            let mut pipeline =
                Pipeline::new(path.as_path(), mode.clone(), config.clone(), proj.clone())?;

            let res = pipeline.build_all(true).context("Build error:");
            err_print(res);

            let p2 = path.as_path().join("content");
            let tp = path.as_path().join("templates");
            let p_build = path.as_path().join("build/html");

            let (server, controller) = Server::bind(([127, 0, 0, 1], 8000).into())
                .add_mount(config.url_prefix.clone(), p_build)?
                .build()?;

            print!("\n\n");

            println!("Server open at: http://localhost:8000{}", config.url_prefix);

            let notify_config = notify::Config::default();
            let mut debouncer: Debouncer<RecommendedWatcher> = new_debouncer_opt(
                Duration::from_millis(20),
                None,
                move |res: DebounceEventResult| match res {
                    Ok(events) => events.iter().for_each(|event| {
                        if let DebouncedEventKind::Any = &event.kind {
                            let p = &event.path;
                            println!();

                            if p.starts_with(path.as_path().join("content")) {
                                // pipeline.build_file(p, &c2, &cf);
                                let res = pipeline.build_single(p.to_path_buf());
                                err_print(res);
                            } else {
                                if p.starts_with(
                                    path.as_path().join("templates").join("shortcodes"),
                                ) {
                                    let res = pipeline.reload_shortcode_tera();
                                    err_print(res);
                                    println!("{}", style("reloaded shortcode templates").green());
                                }
                                let res = pipeline.reload_base_tera();
                                println!("{}", style("Reloaded page templates").green());
                                err_print(res);

                                let res = pipeline.build_all(false);
                                err_print(res);
                            }

                            controller.reload();
                            println!();
                            println!("Page reloaded");
                            println!("Server open at: http://localhost:8000{}", config.url_prefix);
                        }
                    }),
                    Err(errs) => errs.iter().for_each(|e| println!("Error {:?}", e)),
                },
                notify_config,
            )?;

            // Add a path to be watched. All files and directories at that path and
            // below will be monitored for changes.
            debouncer
                .watcher()
                .watch(p2.as_path(), RecursiveMode::Recursive)?;
            debouncer
                .watcher()
                .watch(tp.as_path(), RecursiveMode::Recursive)?;

            server.await?;

            Ok(())
        }
        Commands::Init { name, repository } => {
            println!("{}", style("Project initialisation").bold().blue());

            let repository = repository.map(Ok::<String, InquireError>).unwrap_or_else(|| {
                let options = vec!["Default", "Empty"];
                let values = vec!["https://github.com/coursesproject/courses-template-default/archive/main.zip", ""];

                let mut s: Select<&str> = Select::new("Select a project template:", options);

                s.starting_cursor = 0;
                Ok(values[s.raw_prompt()?.index].to_string())
            })?;

            let dir = name
                .map(|n| {
                    let dir = env::current_dir()?.join(n);
                    create_dir(dir.as_path())?;
                    Ok(dir)
                })
                .unwrap_or_else(env::current_dir)?;

            print!("Template downloading ...");

            setup::setup(dir, repository)?;

            println!("{}", style("Success").green().bold());

            Ok(())
        }
        _ => Ok(()),
    }
}

fn err_print(res: anyhow::Result<()>) {
    match res {
        Ok(_) => {}
        Err(e) => {
            eprintln!("{} {}", style("Error:").red().bold(), e);
            e.chain()
                .skip(1)
                .for_each(|cause| eprintln!(" {} {}", style("caused by:").bold(), cause));
        }
    }
}

#[tokio::main]
async fn main() {
    err_print(cli_run().await)
}
