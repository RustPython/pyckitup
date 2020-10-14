use super::Size;
use anyhow::Context;
use rustpython_bytecode::bytecode::FrozenModule;
use rustpython_compiler::compile;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub fn pyckitup_build(file: PathBuf, size: Size) -> anyhow::Result<()> {
    eprintln!("Deploying to `./build`");
    anyhow::ensure!(
        file.exists(),
        "Input file {:?} doesn't exist. Doing nothing.",
        file
    );
    let mut options = fs_extra::dir::CopyOptions::new();
    options.copy_inside = true;
    options.overwrite = true;
    fs_extra::dir::copy("./static", "./build", &options).context("Cannot copy folder")?;
    let dist: include_dir::Dir = include_dir::include_dir!("../wasm/dist/");
    let build_path = Path::new("build");
    for f in dist.files() {
        std::fs::write(build_path.join(f.path()), f.contents())?;
    }
    std::fs::write(
        "./build/server.py",
        include_bytes!("../../include/server.py"),
    )?;

    let template = include_str!("../../include/template.html");
    let rendered = render(template, &file, size)?;
    std::fs::write("./build/index.html", rendered)?;
    println!("Deployed!");

    Ok(())
}

fn render(tmpl: &str, entry: &Path, size: Size) -> anyhow::Result<String> {
    let modules = compile_dir(entry.parent().unwrap(), String::new(), compile::Mode::Exec)?;
    let encoded_modules = bincode::serialize(&modules)?;

    let Size(w, h) = size;

    let code = format!(
        "\
window.pyckitupData = {{
    entryModule: {entry:?},
    frozenModules: new Uint8Array({modules:?}),
    width: {w},
    height: {h},
}};
",
        entry = entry
            .file_stem()
            .unwrap()
            .to_str()
            .context("file path is not utf8")?,
        modules = encoded_modules,
        w = w,
        h = h,
    );
    Ok(tmpl.replacen("INSERTCODEHERE", &code, 1))
}

// from rustpython-derive
fn compile_dir(
    // &self,
    path: &Path,
    parent: String,
    mode: compile::Mode,
) -> anyhow::Result<HashMap<String, FrozenModule>> {
    let mut code_map = HashMap::new();
    let paths =
        std::fs::read_dir(&path).with_context(|| format!("Error listing dir {:?}", path))?;
    for path in paths {
        let path = path.context("failed to list file")?;
        let path = path.path();
        let file_name = path
            .file_name()
            .unwrap()
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Invalid UTF-8 in file name {:?}", path))?;
        if path.is_dir() {
            code_map.extend(compile_dir(
                &path,
                format!("{}{}", parent, file_name),
                mode,
            )?);
        } else if file_name.ends_with(".py") {
            let source = fs::read_to_string(&path)
                .with_context(|| format!("Error reading file {:?}", path))?;
            let stem = path.file_stem().unwrap().to_str().unwrap();
            let is_init = stem == "__init__";
            let module_name = if is_init {
                parent.clone()
            } else if parent.is_empty() {
                stem.to_owned()
            } else {
                format!("{}.{}", parent, stem)
            };
            code_map.insert(
                module_name.clone(),
                FrozenModule {
                    code: compile::compile(&source, mode, module_name, Default::default())
                        .with_context(|| format!("Python compile error from {}", path.display()))?,
                    package: is_init,
                },
            );
        }
    }
    Ok(code_map)
}
