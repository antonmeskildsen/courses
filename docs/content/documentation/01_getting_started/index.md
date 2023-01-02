---
title: Getting started
notebook_output: true
---

# Getting started
This page explains the basics of how `courses` is used. It covers installation, creating a new project, serving the project locally, and building the project.

## Installation
For now, you have to compile Courses yourself. However, it only requires a few steps and should work without any tweaks on most setups.



1. You need to install rust and cargo to compile the application. Doing so is most easily accomplished by installing *rustup*. Simply follow the instructions on the [installation page](https://rustup.rs/).
2. Install `courses` by opening a terminal (or command promt/powershell) and running the command `cargo install courses`.


## Usage
Courses is a single *cli* program which provides facilities for creating new projects, serving projects locally, and building projects for distribution. Get an overview of the interface, by running `courses -h`. You should see something similar to the folowing:

```text
Usage: courses [OPTIONS] <COMMAND>

Commands:
  serve    
  build    
  test     
  publish  
  help     Print this message or the help of the given subcommand(s)

Options:
  -p, --path <PATH>  
  -h, --help         Print help information
  -V, --version      Print version information
```

## Creating a new project
To create a new project, use the command `courses init <NAME>` where *NAME* is the name of a new folder the project will be created in. The tool will ask you whether you want to start from a minimal setup (no default theme and templates) or a batteries-included setup similar to this documentation page. The latter is much better if you want to get up and running quickly.

### Local development
Courses includes a dev-server (bsed on [Penguin](https://crates.io/crates/penguin/0.1.7)) and automatically rebuilds files when changes are detected. Simply run `courses serve` in the project directory to start.

### Build for deployment
When you want to build the static site for deployment, run `courses build` to build the project with the *release* configuration. The output is placed in the `build/` folder and is ready for use. Read more about configurations [here](/courses/documentation/02_project_organisation).