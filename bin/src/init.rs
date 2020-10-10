use std::path::PathBuf;

pub fn pyckitup_init(project_name: PathBuf) -> std::io::Result<()> {
    if project_name.exists() {
        println!(
            "Path ./{} already exists. Doing nothing.",
            project_name.display()
        );
        std::process::exit(1);
    }

    println!(
        "Initializing pyckitup project in directory `./{}`",
        project_name.display()
    );
    std::fs::create_dir(&project_name)?;
    std::fs::create_dir(project_name.join("static"))?;
    std::fs::write(
        project_name.join("static/click.wav"),
        include_bytes!("../../include/click.wav"),
    )?;
    std::fs::write(
        project_name.join("run.py"),
        include_bytes!("../../examples/clock.py"),
    )?;
    std::fs::write(
        project_name.join("common.py"),
        include_bytes!("../../examples/common.py"),
    )?;
    std::fs::write(
        project_name.join(".gitignore"),
        include_bytes!("../../include/gitignore"),
    )?;
    println!("Initialized. To run: `pyckitup run`");

    Ok(())
}
