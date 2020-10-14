use std::path::PathBuf;

pub fn pyckitup_init(project_name: PathBuf) -> anyhow::Result<()> {
    anyhow::ensure!(
        !project_name.exists(),
        "Path {:?} already exists. Doing nothing.",
        project_name
    );

    println!(
        "Initializing pyckitup project in directory {:?}",
        project_name.display()
    );
    std::fs::create_dir_all(project_name.join("static"))?;
    std::fs::write(
        project_name.join("static/click.wav"),
        include_bytes!("../static/click.wav"),
    )?;
    std::fs::write(
        project_name.join("run.py"),
        include_bytes!("../examples/clock.py"),
    )?;
    std::fs::write(
        project_name.join("common.py"),
        include_bytes!("../examples/common.py"),
    )?;
    std::fs::write(
        project_name.join(".gitignore"),
        include_bytes!("../include/gitignore"),
    )?;
    println!("Initialized. To run: `pyckitup run`");

    Ok(())
}
