use clap::{App, Arg, SubCommand};

mod build;
mod init;

fn main() {
    let matches = App::new("pickitup")
        .version("0.1")
        .arg(
            Arg::with_name("size")
                .short("s")
                .long("size")
                .value_name("SIZE")
                .help("size, WxH, defaults to 480x270")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("filename")
                .value_name("FNAME")
                .help("filename, defaults to run.py")
                .takes_value(true),
        )
        .subcommand(
            SubCommand::with_name("init")
                .about("initialize a new pyckitup project")
                .arg(Arg::with_name("project").help("name of the project")),
        )
        .subcommand(SubCommand::with_name("build").about("deploy for web"))
        .get_matches();
    if let Some(matches) = matches.subcommand_matches("init") {
        init::pyckitup_init(&matches).expect("Failed to init");
    } else if let Some(_) = matches.subcommand_matches("build") {
        build::pyckitup_build().expect("Failed to build");
    } else {
        pyckitup_run(&matches);
    }
}

fn pyckitup_run(matches: &clap::ArgMatches) {
    let fname = matches.value_of("filename").unwrap_or("run.py");

    if !std::path::Path::new(fname).exists() {
        println!("File `./run.py` doesn't exist. Doing nothing.");
        std::process::exit(1);
    }

    let (w, h) = {
        let size = matches.value_of("size").unwrap_or("800x600");
        let ret: Vec<i32> = size.split("x").map(|i| i.parse().unwrap()).collect();
        (ret[0], ret[1])
    };

    let _ = pyckitup_core::FNAME.set(fname.to_owned());

    pyckitup_core::run(w, h);
}
