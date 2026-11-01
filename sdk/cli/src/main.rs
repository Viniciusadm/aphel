use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use zip::write::SimpleFileOptions;

const WASM_TARGET: &str = "wasm32-wasip2";
const JS_BUILD_SCRIPT: &str = "build:backend";
const USAGE: &str = "usage:
  aphel-plugin new <key> [--js] [--sdk <path to aphel-plugin-sdk>]
  aphel-plugin build [dir]
  aphel-plugin pack [dir] [-o file.aphelplugin]";

type Failure = Box<dyn std::error::Error>;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.first().map(String::as_str) {
        Some("new") => new(&args[1..]),
        Some("build") => build(&dir(&args[1..])),
        Some("pack") => {
            build(&dir(&args[1..])).and_then(|()| pack(&dir(&args[1..]), output(&args[1..])))
        }
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn dir(args: &[String]) -> PathBuf {
    args.iter()
        .take_while(|arg| !arg.starts_with('-'))
        .next()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn output(args: &[String]) -> Option<PathBuf> {
    args.windows(2)
        .find(|pair| pair[0] == "-o")
        .map(|pair| PathBuf::from(&pair[1]))
}

fn run(command: &mut Command) -> Result<(), Failure> {
    let status = command.status()?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{command:?} failed with {status}").into())
    }
}

fn manifest(dir: &Path) -> Result<toml::Table, Failure> {
    Ok(fs::read_to_string(dir.join("plugin.toml"))?.parse()?)
}

fn crate_name(dir: &Path) -> Result<Option<String>, Failure> {
    let path = dir.join("Cargo.toml");
    if !path.exists() {
        return Ok(None);
    }
    let cargo: toml::Table = fs::read_to_string(path)?.parse()?;
    Ok(cargo
        .get("package")
        .and_then(|package| package.get("name"))
        .and_then(toml::Value::as_str)
        .map(|name| name.replace('-', "_")))
}

fn npm_script(dir: &Path, script: &str) -> Result<bool, Failure> {
    let path = dir.join("package.json");
    if !path.exists() {
        return Ok(false);
    }
    Ok(fs::read_to_string(path)?.contains(&format!("\"{script}\"")))
}

fn npm(dir: &Path, script: &str) -> Result<(), Failure> {
    if !dir.join("node_modules").exists() {
        run(Command::new("npm").current_dir(dir).arg("install"))?;
    }
    run(Command::new("npm").current_dir(dir).args(["run", script]))
}

fn build(dir: &Path) -> Result<(), Failure> {
    manifest(dir)?;
    if npm_script(dir, JS_BUILD_SCRIPT)? {
        npm(dir, JS_BUILD_SCRIPT)?;
    } else if let Some(name) = crate_name(dir)? {
        run(Command::new("cargo").current_dir(dir).args([
            "build",
            "--release",
            "--target",
            WASM_TARGET,
        ]))?;
        let wasm = dir
            .join("target")
            .join(WASM_TARGET)
            .join("release")
            .join(format!("{name}.wasm"));
        fs::copy(&wasm, dir.join("backend.wasm"))
            .map_err(|error| format!("could not copy {}: {error}", wasm.display()))?;
    }
    let web = dir.join("web");
    if npm_script(&web, "build")? {
        npm(&web, "build")?;
    }
    Ok(())
}

fn files(root: &Path, current: &Path, found: &mut Vec<PathBuf>) -> Result<(), Failure> {
    for entry in fs::read_dir(current)? {
        let path = entry?.path();
        if path.is_dir() {
            files(root, &path, found)?;
        } else {
            found.push(path.strip_prefix(root)?.to_path_buf());
        }
    }
    Ok(())
}

fn pack(dir: &Path, output: Option<PathBuf>) -> Result<(), Failure> {
    let manifest = manifest(dir)?;
    let text = |key: &str| {
        manifest
            .get(key)
            .and_then(toml::Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| format!("plugin.toml needs {key}"))
    };
    let key = text("key")?;
    let version = text("version")?;
    let output = output.unwrap_or_else(|| dir.join(format!("{key}-{version}.aphelplugin")));
    let mut archive = zip::ZipWriter::new(File::create(&output)?);
    let options = SimpleFileOptions::default();
    archive.start_file("plugin.toml", options)?;
    archive.write_all(&fs::read(dir.join("plugin.toml"))?)?;
    if dir.join("backend.wasm").exists() {
        archive.start_file("backend.wasm", options)?;
        archive.write_all(&fs::read(dir.join("backend.wasm"))?)?;
    }
    let dist = dir.join("web").join("dist");
    if dist.exists() {
        let mut found = Vec::new();
        files(&dist, &dist, &mut found)?;
        found.sort();
        for path in found {
            let name = format!("web/{}", path.to_string_lossy().replace('\\', "/"));
            archive.start_file(name, options)?;
            archive.write_all(&fs::read(dist.join(&path))?)?;
        }
    }
    archive.finish()?;
    println!("packed {}", output.display());
    Ok(())
}

fn new(args: &[String]) -> Result<(), Failure> {
    let key = args.first().ok_or(USAGE)?;
    let js = args.iter().any(|arg| arg == "--js");
    let sdk = args
        .windows(2)
        .find(|pair| pair[0] == "--sdk")
        .map(|pair| pair[1].clone())
        .unwrap_or_else(|| {
            if js {
                "../aphel/sdk/js/aphel-plugin-sdk"
            } else {
                "../aphel/sdk/rust/aphel-plugin-sdk"
            }
            .to_owned()
        });
    let dir = PathBuf::from(format!("aphel-{}", key.replace('_', "-")));
    if dir.exists() {
        return Err(format!("{} already exists", dir.display()).into());
    }
    fs::create_dir_all(dir.join("src"))?;
    fs::write(
        dir.join("plugin.toml"),
        format!(
            "key = \"{key}\"\nname = \"{key}\"\nversion = \"0.1.0\"\napi = 1\nrequires_aphel = \">=0.1.0\"\n\n[menu]\nflat = true\n\n[[routes]]\nmethod = \"GET\"\npath = \"/ping\"\n"
        ),
    )?;
    if js {
        fs::write(
            dir.join("package.json"),
            format!(
                "{{\n  \"name\": \"aphel-plugin-{}\",\n  \"private\": true,\n  \"type\": \"module\",\n  \"scripts\": {{\n    \"{JS_BUILD_SCRIPT}\": \"aphel-plugin-js src/plugin.js -o backend.wasm\"\n  }},\n  \"devDependencies\": {{\n    \"@aphel/plugin-sdk\": \"file:{sdk}\"\n  }}\n}}\n",
                key.replace('_', "-")
            ),
        )?;
        fs::write(
            dir.join(".gitignore"),
            "node_modules/\nbackend.wasm\n*.aphelplugin\n",
        )?;
        fs::write(
            dir.join("src").join("plugin.js"),
            "import { definePlugin, ok } from '@aphel/plugin-sdk'\n\nexport const { onHook, onEvent, onRoute, onSchedule, onAction, onWidget } = definePlugin({\n  onRoute(ctx, request) {\n    if (request.is('GET', '/ping')) return ok({ pong: request.path })\n  },\n})\n",
        )?;
    } else {
        fs::write(
            dir.join("Cargo.toml"),
            format!(
                "[package]\nname = \"aphel-plugin-{}\"\nversion = \"0.1.0\"\nedition = \"2024\"\npublish = false\n\n[lib]\ncrate-type = [\"cdylib\"]\n\n[dependencies]\naphel-plugin-sdk = {{ path = \"{sdk}\" }}\n\n[profile.release]\nopt-level = \"s\"\nlto = true\n\n[workspace]\n",
                key.replace('_', "-")
            ),
        )?;
        fs::write(
            dir.join("src").join("lib.rs"),
            "use aphel_plugin_sdk::{Ctx, Plugin, Request, Response, Result, export_plugin, json};\n\nstruct App;\n\nimpl Plugin for App {\n    fn on_route(&self, _ctx: &Ctx, request: &Request) -> Result<Response> {\n        Ok(Response::ok(json!({ \"pong\": request.path })))\n    }\n}\n\nexport_plugin!(App);\n",
        )?;
    }
    println!("created {}", dir.display());
    Ok(())
}
