use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};
use std::process::{Command, ExitCode};

use serde::Deserialize;
use serde_json::Value;

const SELECTION: &str = "aphel.modules.toml";
const APP_MANIFEST: &str = "app/Cargo.toml";
const APP_REGISTRY: &str = "app/src/modules.rs";
const APP_PACKAGE: &str = "aphel";
const WEB_DIR: &str = "../web";
const WEB_MODULES_DIR: &str = "src/modules";
const WEB_REGISTRY: &str = "src/modules.generated.ts";
const WEB_ALIASES: &str = "modules.aliases.json";
const WEB_TSCONFIG: &str = "tsconfig.modules.json";
const DEPENDENCY_PREFIX: &str = "module-";
const KERNEL_MODULE: &str = "kernel";
const PLUGIN_WIT: &str = "modules/wasm_runtime/wit/plugin.wit";
const SDK_PLUGIN_WITS: [&str; 2] = [
    "../sdk/rust/aphel-plugin-sdk/wit/plugin.wit",
    "../sdk/js/aphel-plugin-sdk/wit/plugin.wit",
];

mod arch;
mod themes;

type Failure = Box<dyn std::error::Error>;

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ModuleSpec {
    path: Option<String>,
    git: Option<String>,
    tag: Option<String>,
    branch: Option<String>,
    rev: Option<String>,
    version: Option<String>,
    package: Option<String>,
    #[serde(default)]
    required: bool,
}

struct SelectedModule {
    key: String,
    spec: ModuleSpec,
}

struct ResolvedModule {
    key: String,
    required: bool,
    module_type: String,
    web: Option<PathBuf>,
}

struct Generated {
    path: PathBuf,
    content: String,
}

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let words: Vec<&str> = arguments.iter().map(String::as_str).collect();
    let result = match words.as_slice() {
        ["modules", "sync"] => sync(false),
        ["modules", "sync", "--check"] => sync(true),
        ["themes", "sync"] => themes::sync(false),
        ["themes", "sync", "--check"] => themes::sync(true),
        ["arch", "check"] => arch::run(arch::Mode::Check, false),
        ["arch", "check", "--details"] => arch::run(arch::Mode::Check, true),
        ["arch", "check", "--report-only"] => arch::run(arch::Mode::ReportOnly, false),
        ["arch", "check", "--report-only", "--details"] => arch::run(arch::Mode::ReportOnly, true),
        ["arch", "baseline"] => arch::run(arch::Mode::WriteBaseline, false),
        _ => {
            eprintln!(
                "usage: cargo run -p xtask -- modules|themes sync [--check] | arch check [--report-only] [--details] | arch baseline"
            );
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn back_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("xtask lives inside back/")
        .to_path_buf()
}

fn sync(check: bool) -> Result<bool, Failure> {
    let back = back_dir();
    let selection = read_selection(&back.join(SELECTION))?;
    let profiles: Vec<Value> = serde_json::from_str(&fs::read_to_string(
        back.join("../blueprints/profiles.json"),
    )?)?;
    for module in selection.iter().filter(|module| !module.spec.required) {
        if !profiles.iter().any(|profile| {
            profile["modules"].as_array().is_some_and(|modules| {
                modules
                    .iter()
                    .any(|key| key.as_str() == Some(module.key.as_str()))
            })
        }) {
            return Err(format!("optional module {} has no installer profile", module.key).into());
        }
    }
    let manifest_path = back.join(APP_MANIFEST);
    let current_manifest = fs::read_to_string(&manifest_path)?;
    let manifest = app_manifest(&current_manifest, &dependency_lines(&back, &selection)?)?;

    if check {
        let mut outdated = Vec::new();
        if manifest != current_manifest {
            outdated.push(manifest_path.clone());
        }
        let resolved = resolve(&back, &selection, Some(&manifest))?;
        for generated in generated_files(&back, &resolved)? {
            if fs::read_to_string(&generated.path).ok().as_deref() != Some(&generated.content) {
                outdated.push(generated.path);
            }
        }
        if outdated.is_empty() {
            println!("the module registries match {SELECTION}");
            return Ok(true);
        }
        for path in outdated {
            eprintln!("outdated: {}", path.display());
        }
        eprintln!("run `make modules` (cargo run -p xtask -- modules sync)");
        return Ok(false);
    }

    write_if_changed(&manifest_path, &manifest)?;
    let resolved = resolve(&back, &selection, None)?;
    for generated in generated_files(&back, &resolved)? {
        write_if_changed(&generated.path, &generated.content)?;
    }
    println!(
        "{} modules selected in {SELECTION} ({} with a web package)",
        resolved.len(),
        resolved
            .iter()
            .filter(|module| module.web.is_some())
            .count()
    );
    Ok(true)
}

fn read_selection(path: &Path) -> Result<Vec<SelectedModule>, Failure> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let document: toml::Table = text.parse()?;
    let modules = document
        .get("modules")
        .and_then(toml::Value::as_table)
        .ok_or_else(|| format!("{} has no [modules] table", path.display()))?;
    let mut selected = Vec::new();
    for (key, value) in modules {
        if !valid_key(key) {
            return Err(format!("`{key}` is not a valid module key (use a-z, 0-9 and _)").into());
        }
        if key == KERNEL_MODULE {
            return Err("the kernel is always compiled; remove it from [modules]".into());
        }
        let spec: ModuleSpec = value
            .clone()
            .try_into()
            .map_err(|error| format!("module `{key}`: {error}"))?;
        let sources = [
            spec.path.is_some(),
            spec.git.is_some(),
            spec.version.is_some(),
        ]
        .iter()
        .filter(|present| **present)
        .count();
        if sources != 1 {
            return Err(
                format!("module `{key}` needs exactly one of `path`, `git` or `version`").into(),
            );
        }
        if spec.git.is_none() && (spec.tag.is_some() || spec.branch.is_some() || spec.rev.is_some())
        {
            return Err(format!("module `{key}`: `tag`, `branch` and `rev` need `git`").into());
        }
        selected.push(SelectedModule {
            key: key.clone(),
            spec,
        });
    }
    Ok(selected)
}

fn valid_key(key: &str) -> bool {
    let mut characters = key.chars();
    characters
        .next()
        .is_some_and(|first| first.is_ascii_lowercase())
        && characters.all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        })
}

fn package_name(back: &Path, module: &SelectedModule) -> Result<String, Failure> {
    if let Some(package) = &module.spec.package {
        return Ok(package.clone());
    }
    if let Some(path) = &module.spec.path {
        let manifest = back.join(path).join("Cargo.toml");
        let text = fs::read_to_string(&manifest)
            .map_err(|error| format!("module `{}`: {}: {error}", module.key, manifest.display()))?;
        let document: toml::Table = text.parse()?;
        return document
            .get("package")
            .and_then(|package| package.get("name"))
            .and_then(toml::Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| format!("{} has no package name", manifest.display()).into());
    }
    Ok(format!("aphel-mod-{}", module.key.replace('_', "-")))
}

fn dependency_lines(back: &Path, selection: &[SelectedModule]) -> Result<Vec<String>, Failure> {
    let mut lines = Vec::new();
    for module in selection {
        let mut fields = vec![
            "optional = true".to_owned(),
            format!("package = {}", quoted(&package_name(back, module)?)),
        ];
        let spec = &module.spec;
        if let Some(path) = &spec.path {
            fields.push(format!("path = {}", quoted(&format!("../{path}"))));
        }
        if let Some(git) = &spec.git {
            fields.push(format!("git = {}", quoted(git)));
        }
        for (name, value) in [
            ("tag", &spec.tag),
            ("branch", &spec.branch),
            ("rev", &spec.rev),
        ] {
            if let Some(value) = value {
                fields.push(format!("{name} = {}", quoted(value)));
            }
        }
        if let Some(version) = &spec.version {
            fields.push(format!("version = {}", quoted(version)));
        }
        lines.push(format!(
            "{DEPENDENCY_PREFIX}{} = {{ {} }}",
            module.key.replace('_', "-"),
            fields.join(", ")
        ));
    }
    Ok(lines)
}

fn quoted(value: &str) -> String {
    toml::Value::String(value.to_owned()).to_string()
}

fn app_manifest(current: &str, generated: &[String]) -> Result<String, Failure> {
    let lines: Vec<&str> = current
        .lines()
        .filter(|line| !line.starts_with(DEPENDENCY_PREFIX))
        .collect();
    let start = lines
        .iter()
        .position(|line| line.trim() == "[dependencies]")
        .ok_or("app/Cargo.toml has no [dependencies] table")?;
    let mut end = lines[start + 1..]
        .iter()
        .position(|line| line.trim_start().starts_with('['))
        .map_or(lines.len(), |offset| start + 1 + offset);
    while end > start + 1 && lines[end - 1].trim().is_empty() {
        end -= 1;
    }
    let mut output: Vec<String> = lines[..end].iter().map(|line| (*line).to_owned()).collect();
    if !generated.is_empty() {
        output.push(String::new());
        output.extend(generated.iter().cloned());
    }
    let rest: Vec<String> = lines[end..]
        .iter()
        .skip_while(|line| line.trim().is_empty())
        .map(|line| (*line).to_owned())
        .collect();
    if !rest.is_empty() {
        output.push(String::new());
        output.extend(rest);
    }
    let mut text = output.join("\n") + "\n";
    let features = format!(
        "[features]\ndefault = [\"plugins\"]\nplugins = [{}]\n\n",
        generated
            .iter()
            .filter_map(|line| line
                .split_once(" = ")
                .map(|(key, _)| quoted(&format!("dep:{key}"))))
            .collect::<Vec<_>>()
            .join(", ")
    );
    if let Some(start) = text.find("[features]\n") {
        let end = text[start + 11..]
            .find("\n[")
            .map_or(text.len(), |offset| start + 12 + offset);
        text.replace_range(start..end, &features);
    } else {
        let start = text.find("[dependencies]").ok_or("missing dependencies")?;
        text.insert_str(start, &features);
    }
    Ok(text)
}

fn resolve(
    back: &Path,
    selection: &[SelectedModule],
    manifest_override: Option<&str>,
) -> Result<Vec<ResolvedModule>, Failure> {
    let metadata = cargo_metadata(back, manifest_override)?;
    let packages = metadata["packages"]
        .as_array()
        .ok_or("cargo metadata returned no packages")?;
    let app = packages
        .iter()
        .find(|package| package["name"] == APP_PACKAGE)
        .ok_or("the app package was not found by cargo metadata")?;
    let dependencies = app["dependencies"].as_array().cloned().unwrap_or_default();
    let mut resolved = Vec::new();
    for module in selection {
        let rename = format!("{DEPENDENCY_PREFIX}{}", module.key.replace('_', "-"));
        let dependency = dependencies
            .iter()
            .find(|dependency| dependency["rename"] == rename.as_str())
            .ok_or_else(|| format!("module `{}` is not a dependency of the app", module.key))?;
        let package = find_package(packages, dependency).ok_or_else(|| {
            format!(
                "module `{}`: package {} was not resolved",
                module.key, dependency["name"]
            )
        })?;
        let manifest_dir = Path::new(package["manifest_path"].as_str().unwrap_or_default())
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default();
        let aphel = &package["metadata"]["aphel"];
        let module_type = aphel["module"]
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| format!("{}Module", pascal_case(&module.key)));
        let web = match aphel["web"].as_str() {
            Some(relative) => {
                let directory = normalize(&manifest_dir.join(relative));
                if !directory.is_dir() {
                    return Err(format!(
                        "module `{}`: the web package {} does not exist",
                        module.key,
                        directory.display()
                    )
                    .into());
                }
                Some(directory)
            }
            None => {
                let directory =
                    normalize(&back.join(WEB_DIR).join(WEB_MODULES_DIR).join(&module.key));
                directory.is_dir().then_some(directory)
            }
        };
        resolved.push(ResolvedModule {
            key: module.key.clone(),
            required: module.spec.required,
            module_type,
            web,
        });
    }
    Ok(resolved)
}

fn cargo_metadata(back: &Path, manifest_override: Option<&str>) -> Result<Value, Failure> {
    let manifest_path = back.join(APP_MANIFEST);
    let original = fs::read_to_string(&manifest_path)?;
    let swapped = manifest_override.is_some_and(|manifest| manifest != original);
    if let Some(manifest) = manifest_override.filter(|_| swapped) {
        fs::write(&manifest_path, manifest)?;
    }
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let output = Command::new(cargo)
        .current_dir(back)
        .args(["metadata", "--format-version", "1", "--manifest-path"])
        .arg(&manifest_path)
        .output();
    if swapped {
        fs::write(&manifest_path, &original)?;
    }
    let output = output?;
    if !output.status.success() {
        return Err(format!(
            "cargo metadata failed:\n{}",
            String::from_utf8_lossy(&output.stderr)
        )
        .into());
    }
    Ok(serde_json::from_slice(&output.stdout)?)
}

fn find_package<'a>(packages: &'a [Value], dependency: &Value) -> Option<&'a Value> {
    let name = dependency["name"].as_str()?;
    let candidates: Vec<&Value> = packages
        .iter()
        .filter(|package| package["name"] == name)
        .collect();
    if candidates.len() <= 1 {
        return candidates.first().copied();
    }
    if let Some(path) = dependency["path"].as_str() {
        let directory = normalize(Path::new(path));
        return candidates.into_iter().find(|package| {
            Path::new(package["manifest_path"].as_str().unwrap_or_default())
                .parent()
                .is_some_and(|parent| normalize(parent) == directory)
        });
    }
    let source = dependency["source"].as_str().unwrap_or_default();
    candidates.into_iter().find(|package| {
        package["source"].as_str().is_some_and(|candidate| {
            candidate.starts_with(source.split('#').next().unwrap_or(source))
        })
    })
}

fn normalize(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                normalized.pop();
            }
            Component::CurDir => {}
            other => normalized.push(other),
        }
    }
    normalized
}

fn relative_to(path: &Path, base: &Path) -> String {
    let path: Vec<Component> = path.components().collect();
    let base: Vec<Component> = base.components().collect();
    let shared = path
        .iter()
        .zip(base.iter())
        .take_while(|(left, right)| left == right)
        .count();
    let mut parts: Vec<String> = base[shared..].iter().map(|_| "..".to_owned()).collect();
    parts.extend(
        path[shared..]
            .iter()
            .map(|component| component.as_os_str().to_string_lossy().into_owned()),
    );
    let joined = parts.join("/");
    if joined.starts_with("..") {
        joined
    } else {
        format!("./{joined}")
    }
}

fn pascal_case(key: &str) -> String {
    key.split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut characters = part.chars();
            characters
                .next()
                .map(|first| first.to_ascii_uppercase().to_string() + characters.as_str())
                .unwrap_or_default()
        })
        .collect()
}

fn camel_case(key: &str) -> String {
    let pascal = pascal_case(key);
    let mut characters = pascal.chars();
    characters
        .next()
        .map(|first| first.to_ascii_lowercase().to_string() + characters.as_str())
        .unwrap_or_default()
}

fn generated_files(back: &Path, modules: &[ResolvedModule]) -> Result<Vec<Generated>, Failure> {
    let web = normalize(&back.join(WEB_DIR));
    let mut generated = vec![
        Generated {
            path: back.join(APP_REGISTRY),
            content: rust_registry(modules),
        },
        Generated {
            path: web.join(WEB_REGISTRY),
            content: web_registry(modules),
        },
        Generated {
            path: web.join(WEB_ALIASES),
            content: web_aliases(&web, modules)?,
        },
        Generated {
            path: web.join(WEB_TSCONFIG),
            content: web_tsconfig(&web, modules)?,
        },
    ];
    for sdk_wit in SDK_PLUGIN_WITS {
        let sdk = normalize(&back.join(sdk_wit));
        if sdk
            .parent()
            .and_then(Path::parent)
            .is_some_and(Path::exists)
        {
            generated.push(Generated {
                path: sdk,
                content: fs::read_to_string(back.join(PLUGIN_WIT))?,
            });
        }
    }
    Ok(generated)
}

fn rust_registry(modules: &[ResolvedModule]) -> String {
    let required: Vec<String> = modules
        .iter()
        .filter(|module| module.required)
        .map(|module| format!("    \"{}\",\n", module.key))
        .collect();
    let compiled: String = modules
        .iter()
        .map(|module| {
            format!(
                "        #[cfg(feature = \"plugins\")]\n        Box::new({}{}::{}),\n",
                DEPENDENCY_PREFIX.replace('-', "_"),
                module.key,
                module.module_type
            )
        })
        .collect();
    format!(
        "use aphel_kernel::errors::AppError;\n\
         use aphel_kernel::modules::{{App, AppBuilder, Module}};\n\
         \n\
         pub const REQUIRED: &[&str] = &[\n{}];\n\
         \n\
         pub fn compiled() -> Vec<Box<dyn Module>> {{\n    vec![\n        Box::new(aphel_kernel::KernelModule),\n{}    ]\n}}\n\
         \n\
         pub fn build() -> Result<App, AppError> {{\n    let builder = AppBuilder::new();\n    #[cfg(feature = \"plugins\")]\n    let builder = {{\n        let mut builder = builder;\n        crate::blueprints::register(&mut builder);\n        builder\n    }};\n    builder\n        .modules(compiled())\n        .require(REQUIRED.iter().copied())\n        .build()\n}}\n",
        required.concat(),
        compiled
    )
}

fn web_registry(modules: &[ResolvedModule]) -> String {
    let with_web: Vec<&ResolvedModule> = modules
        .iter()
        .filter(|module| module.web.is_some())
        .collect();
    let mut imports = vec![
        "import type { WebModule } from '@/kernel/modules/define'".to_owned(),
        "import kernelModule from '@modules/kernel'".to_owned(),
    ];
    imports.extend(with_web.iter().map(|module| {
        format!(
            "import {}Module from '@modules/{}'",
            camel_case(&module.key),
            module.key
        )
    }));
    let mut entries = vec!["  kernelModule,".to_owned()];
    entries.extend(
        with_web
            .iter()
            .map(|module| format!("  {}Module,", camel_case(&module.key))),
    );
    format!(
        "{}\n\nexport const MODULES: WebModule[] = [\n{}\n]\n",
        imports.join("\n"),
        entries.join("\n")
    )
}

fn external_web(web: &Path, modules: &[ResolvedModule]) -> BTreeMap<String, String> {
    let modules_dir = web.join(WEB_MODULES_DIR);
    modules
        .iter()
        .filter_map(|module| {
            let directory = module.web.as_ref()?;
            (directory != &modules_dir.join(&module.key))
                .then(|| (module.key.clone(), relative_to(directory, web)))
        })
        .collect()
}

fn web_aliases(web: &Path, modules: &[ResolvedModule]) -> Result<String, Failure> {
    let aliases = external_web(web, modules);
    Ok(serde_json::to_string_pretty(&aliases)? + "\n")
}

fn web_tsconfig(web: &Path, modules: &[ResolvedModule]) -> Result<String, Failure> {
    let mut paths = serde_json::Map::new();
    paths.insert("@/*".to_owned(), serde_json::json!(["./src/*"]));
    for (key, directory) in external_web(web, modules) {
        paths.insert(format!("@modules/{key}"), serde_json::json!([directory]));
        paths.insert(
            format!("@modules/{key}/*"),
            serde_json::json!([format!("{directory}/*")]),
        );
    }
    paths.insert(
        "@modules/*".to_owned(),
        serde_json::json!(["./src/modules/*"]),
    );
    let document = serde_json::json!({ "compilerOptions": { "paths": paths } });
    Ok(serde_json::to_string_pretty(&document)? + "\n")
}

fn write_if_changed(path: &Path, content: &str) -> Result<(), Failure> {
    if fs::read_to_string(path).ok().as_deref() == Some(content) {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)?;
    println!("wrote {}", path.display());
    Ok(())
}
