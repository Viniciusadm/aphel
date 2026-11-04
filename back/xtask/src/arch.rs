use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::{Failure, back_dir};

const BASELINE_FILE: &str = "xtask/arch-baseline.json";
const KERNEL_SOURCE: &str = "kernel/src";
const KERNEL_MIGRATIONS: &str = "kernel/migrations";
const PLUGIN_ROOTS: [&str; 2] = ["modules", "plugins"];
const PLUGIN_CRATE_PREFIX: &str = "aphel-mod-";
const RECORD_TYPE_FILE: &str = "kernel/src/application/shared/record_ref.rs";
const PERMISSION_FILE: &str = "kernel/src/domain/access/permission.rs";
const SELECTION_FILE: &str = "aphel.modules.toml";
const WEB_SOURCE: &str = "../web/src";
const WEB_PLUGIN_ROOT: &str = "modules";
const WEB_SHELL_ROOTS: [&str; 6] = ["app", "features", "kernel", "components", "lib", "pwa"];
const WEB_KERNEL_PLUGIN: &str = "kernel";

const GENERIC_RECORD_TYPES: [&str; 5] = ["FILE", "MEMBER", "ROLE", "TEAM", "ORGANIZATION"];
const CORE_PERMISSION_AREAS: [&str; 6] =
    ["settings", "users", "audit", "system", "records", "privacy"];
const SQL_TABLE_KEYWORDS: [&str; 6] = ["FROM", "JOIN", "INTO", "UPDATE", "TABLE", "EXISTS"];
const DOMAIN_WORDS: [&str; 15] = [
    "contact",
    "people",
    "company",
    "organizations",
    "deal",
    "deals",
    "lead",
    "leads",
    "quote",
    "quotes",
    "proposal",
    "proposals",
    "crm",
    "renewal",
    "workflow",
];
const FOREIGN_RECORD_LITERALS: [&str; 24] = [
    "\"people.person\"",
    "\"organizations.organization\"",
    "\"deals.deal\"",
    "\"leads.lead\"",
    "\"quotes.quote\"",
    "\"proposals.proposal\"",
    "\"tasks.task\"",
    "\"catalog_items.item\"",
    "\"notes.note\"",
    "\"workflow.pipeline\"",
    "\"workflow.stage\"",
    "\"workflow.lost_reason\"",
    "\"contact\"",
    "\"company\"",
    "\"deal\"",
    "\"lead\"",
    "\"quote\"",
    "\"proposal\"",
    "\"activity\"",
    "\"product\"",
    "\"note\"",
    "\"pipeline\"",
    "\"pipeline_stage\"",
    "\"lost_reason\"",
];

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Check,
    ReportOnly,
    WriteBaseline,
}

struct Metric {
    id: &'static str,
    description: &'static str,
    findings: Vec<String>,
}

impl Metric {
    fn new(id: &'static str, description: &'static str, findings: Vec<String>) -> Self {
        Self {
            id,
            description,
            findings,
        }
    }

    fn count(&self) -> u64 {
        self.findings.len() as u64
    }
}

struct Plugin {
    key: String,
    dir: PathBuf,
}

struct Tables {
    owner_by_table: BTreeMap<String, String>,
}

pub fn run(mode: Mode, details: bool) -> Result<bool, Failure> {
    let back = back_dir();
    let mut metrics = backend_metrics(&back)?;
    metrics.extend(web_metrics(&back)?);
    let baseline_path = back.join(BASELINE_FILE);

    if mode == Mode::WriteBaseline {
        let counts: BTreeMap<&str, u64> = metrics
            .iter()
            .map(|metric| (metric.id, metric.count()))
            .collect();
        fs::write(
            &baseline_path,
            format!("{}\n", serde_json::to_string_pretty(&counts)?),
        )?;
        println!(
            "baseline written to {} ({} checks)",
            baseline_path.display(),
            counts.len()
        );
        return Ok(true);
    }

    let baseline = read_baseline(&baseline_path)?;
    let mut regressions = Vec::new();
    let mut ratchets = Vec::new();
    println!("{:<46} {:>8} {:>9}", "check", "now", "baseline");
    for metric in &metrics {
        let now = metric.count();
        let recorded = baseline.get(metric.id).copied();
        let label = match recorded {
            Some(limit) if now > limit => {
                regressions.push(format!("{}: {now} > baseline {limit}", metric.id));
                "RISES"
            }
            Some(limit) if now < limit => {
                ratchets.push(format!("{}: {now} < baseline {limit}", metric.id));
                "falls"
            }
            Some(_) => "",
            None => {
                regressions.push(format!("{}: no baseline recorded", metric.id));
                "NEW"
            }
        };
        println!(
            "{:<46} {:>8} {:>9}  {label}",
            metric.id,
            now,
            recorded.map_or_else(|| "-".to_string(), |limit| limit.to_string()),
        );
        if details {
            println!("    {}", metric.description);
            for finding in &metric.findings {
                println!("      {finding}");
            }
        }
    }
    for ratchet in &ratchets {
        println!(
            "ratchet available: {ratchet} (run `cargo run -p xtask -- arch baseline` to lock it in)"
        );
    }
    if regressions.is_empty() {
        println!("architecture counts are within the baseline");
        return Ok(true);
    }
    for regression in &regressions {
        eprintln!("architecture regression: {regression}");
    }
    if mode == Mode::ReportOnly {
        eprintln!("report-only mode: not failing");
        return Ok(true);
    }
    Ok(false)
}

fn read_baseline(path: &Path) -> Result<BTreeMap<String, u64>, Failure> {
    let text = fs::read_to_string(path).map_err(|error| {
        format!(
            "cannot read {}: {error} (run `cargo run -p xtask -- arch baseline`)",
            path.display()
        )
    })?;
    Ok(serde_json::from_str(&text)?)
}

fn backend_metrics(back: &Path) -> Result<Vec<Metric>, Failure> {
    let plugins = discover_plugins(back)?;
    let tables = collect_tables(back, &plugins)?;
    let kernel_sources = rust_files(&back.join(KERNEL_SOURCE));
    let plugin_sources: Vec<(String, PathBuf)> = plugins
        .iter()
        .flat_map(|plugin| {
            rust_files(&plugin.dir.join("src"))
                .into_iter()
                .map(|file| (plugin.key.clone(), file))
        })
        .collect();

    let mut manifest_problems = Vec::new();
    let mut layers = BTreeMap::new();
    for plugin in &plugins {
        match read_layer(plugin) {
            Ok(layer) => {
                layers.insert(plugin.key.clone(), layer);
            }
            Err(problem) => manifest_problems.push(format!("{}: {problem}", plugin.key)),
        }
    }

    let mut crate_dependencies = Vec::new();
    let mut layer_violations = Vec::new();
    for plugin in &plugins {
        let manifest = fs::read_to_string(plugin.dir.join("Cargo.toml"))?;
        let dependencies = plugin_dependencies(&manifest);
        if !dependencies.is_empty() {
            crate_dependencies.push(format!("{} -> {}", plugin.key, dependencies.join(", ")));
        }
        for dependency in &dependencies {
            if let (Some(from), Some(to)) = (layers.get(&plugin.key), layers.get(dependency))
                && layer_rank(from) < layer_rank(to)
            {
                layer_violations.push(format!("{} ({from}) -> {dependency} ({to})", plugin.key));
            }
        }
    }

    let mut foreign_keys = Vec::new();
    let mut foreign_alters = Vec::new();
    let mut foreign_sql = Vec::new();
    for plugin in &plugins {
        for file in files_with_extension(&plugin.dir.join("migrations"), "sql") {
            let text = fs::read_to_string(&file)?;
            let tokens = tokenize(&text);
            for pair in tokens.windows(2) {
                let (keyword, name) = (pair[0].as_str(), pair[1].as_str());
                let foreign = tables.is_foreign_to(name, &plugin.key);
                if keyword.eq_ignore_ascii_case("REFERENCES") && foreign {
                    foreign_keys.push(format!("{}: references {name}", relative(back, &file)));
                }
            }
            for window in tokens.windows(3) {
                if window[0].eq_ignore_ascii_case("ALTER")
                    && window[1].eq_ignore_ascii_case("TABLE")
                    && tables.owner_by_table.get(&window[2]) != Some(&plugin.key)
                    && tables.owner_by_table.contains_key(&window[2])
                {
                    foreign_alters.push(format!(
                        "{}: ALTER TABLE {}",
                        relative(back, &file),
                        window[2]
                    ));
                }
            }
            foreign_sql.extend(
                sql_table_references(&tokens, &tables, &plugin.key)
                    .into_iter()
                    .map(|name| format!("{}: {name}", relative(back, &file))),
            );
        }
    }
    for (key, file) in &plugin_sources {
        let tokens = tokenize(&fs::read_to_string(file)?);
        foreign_sql.extend(
            sql_table_references(&tokens, &tables, key)
                .into_iter()
                .map(|name| format!("{}: {name}", relative(back, file))),
        );
    }

    let mut kernel_sql = Vec::new();
    for file in &kernel_sources {
        let tokens = tokenize(&fs::read_to_string(file)?);
        for pair in tokens.windows(2) {
            if SQL_TABLE_KEYWORDS.contains(&pair[0].as_str())
                && tables
                    .owner_by_table
                    .get(&pair[1])
                    .is_some_and(|owner| owner != "kernel")
            {
                kernel_sql.push(format!("{}: {}", relative(back, file), pair[1]));
            }
        }
    }

    let mut domain_words = Vec::new();
    for file in &kernel_sources {
        let text = fs::read_to_string(file)?;
        let in_api = relative(back, file).starts_with("kernel/src/api/");
        let counted: String = text
            .lines()
            .filter(|line| !(in_api && (line.contains("example") || line.contains("description"))))
            .collect::<Vec<_>>()
            .join("\n");
        let count = count_domain_words(&counted);
        if count > 0 {
            for _ in 0..count {
                domain_words.push(relative(back, file));
            }
        }
    }

    let record_type_constants = fs::read_to_string(back.join(RECORD_TYPE_FILE))?
        .lines()
        .filter_map(constant_name)
        .filter(|name| !GENERIC_RECORD_TYPES.contains(&name.as_str()))
        .collect::<Vec<_>>();

    let permission_constants = fs::read_to_string(back.join(PERMISSION_FILE))?
        .lines()
        .filter_map(permission_area)
        .filter(|(_, area)| !CORE_PERMISSION_AREAS.contains(&area.as_str()))
        .map(|(name, _)| name)
        .collect::<Vec<_>>();

    let mut kernel_paths = BTreeSet::new();
    for (_, file) in &plugin_sources {
        kernel_paths.extend(kernel_paths_in(&fs::read_to_string(file)?));
    }

    let bridge_files = plugin_sources
        .iter()
        .filter(|(_, file)| {
            file.file_name()
                .is_some_and(|name| name == "plugin_bridge.rs")
        })
        .map(|(key, _)| key.clone())
        .collect::<Vec<_>>();

    let mut literal_plugins = Vec::new();
    for plugin in &plugins {
        let mut found = false;
        for (key, file) in &plugin_sources {
            if key == &plugin.key {
                let text = fs::read_to_string(file)?;
                if FOREIGN_RECORD_LITERALS
                    .iter()
                    .any(|literal| text.contains(literal))
                {
                    found = true;
                    break;
                }
            }
        }
        if found {
            literal_plugins.push(plugin.key.clone());
        }
    }

    let required_plugins = fs::read_to_string(back.join(SELECTION_FILE))?
        .lines()
        .filter(|line| line.replace(' ', "").contains("required=true"))
        .map(|line| line.split('=').next().unwrap_or("").trim().to_string())
        .collect::<Vec<_>>();

    let mut public_modules = Vec::new();
    for file in &kernel_sources {
        for line in fs::read_to_string(file)?.lines() {
            if line.trim_start().starts_with("pub mod ") {
                public_modules.push(format!(
                    "{}: {}",
                    relative(back, file),
                    line.trim().trim_end_matches(';')
                ));
            }
        }
    }

    let mut state_escapes = Vec::new();
    for (_, file) in &plugin_sources {
        let text = fs::read_to_string(file)?;
        let count = text.matches("ctx.state()").count()
            + text.matches("runtime_state()").count()
            + text.matches("env.state()").count();
        for _ in 0..count {
            state_escapes.push(relative(back, file));
        }
    }

    let mut conflicts = Vec::new();
    for (_, file) in &plugin_sources {
        let text = fs::read_to_string(file)?;
        let count = text.matches("conflicts_with").count() + text.matches("MENU_ACTIONS").count();
        for _ in 0..count {
            conflicts.push(relative(back, file));
        }
    }

    Ok(vec![
        Metric::new(
            "plugin_crates_depending_on_plugin_crates",
            "plugin crates with a dependency on another plugin crate",
            crate_dependencies,
        ),
        Metric::new(
            "cross_plugin_foreign_keys",
            "foreign keys that point at a table owned by another plugin",
            foreign_keys,
        ),
        Metric::new(
            "cross_plugin_sql_references",
            "SQL in plugin code or migrations naming a table owned by another plugin",
            foreign_sql,
        ),
        Metric::new(
            "alter_table_on_foreign_tables",
            "ALTER TABLE on a table owned by another plugin",
            foreign_alters,
        ),
        Metric::new(
            "kernel_sql_on_plugin_tables",
            "kernel SQL naming a table owned by a plugin",
            kernel_sql,
        ),
        Metric::new(
            "kernel_domain_words",
            "domain words (contact, deal, lead...) in kernel source, one finding per occurrence",
            domain_words,
        ),
        Metric::new(
            "kernel_record_type_domain_constants",
            "RecordType constants for domain types in the kernel",
            record_type_constants,
        ),
        Metric::new(
            "kernel_permission_constants_for_plugins",
            "permission constants in the kernel that belong to plugins",
            permission_constants,
        ),
        Metric::new(
            "plugin_kernel_paths",
            "distinct aphel_kernel paths used by plugins",
            kernel_paths.into_iter().collect(),
        ),
        Metric::new(
            "plugin_bridge_files",
            "plugin_bridge.rs files",
            bridge_files,
        ),
        Metric::new(
            "plugins_with_literal_record_types",
            "plugins using literal record types of the CRM vocabulary",
            literal_plugins,
        ),
        Metric::new(
            "required_plugins",
            "modules marked required in aphel.modules.toml",
            required_plugins,
        ),
        Metric::new(
            "kernel_public_modules",
            "kernel modules declared pub instead of pub(crate)",
            public_modules,
        ),
        Metric::new(
            "plugin_state_escapes",
            "plugin code reaching the whole AppState through ctx.state(), runtime_state() or env.state()",
            state_escapes,
        ),
        Metric::new(
            "conflict_and_menu_rule_usages",
            "conflicts_with and menu rule usages in the backend",
            conflicts,
        ),
        Metric::new(
            "layer_direction_violations",
            "plugin in a lower layer depending on a plugin in a higher one",
            layer_violations,
        ),
        Metric::new(
            "plugin_manifest_problems",
            "plugins without a valid plugin.toml matching their directory",
            manifest_problems,
        ),
    ])
}

fn web_metrics(back: &Path) -> Result<Vec<Metric>, Failure> {
    let web = back.join(WEB_SOURCE);
    let plugin_root = web.join(WEB_PLUGIN_ROOT);
    let mut cross_imports = Vec::new();
    let mut feature_imports = Vec::new();
    let mut conflicts = Vec::new();
    for entry in fs::read_dir(&plugin_root)? {
        let entry = entry?;
        if !entry.path().is_dir() {
            continue;
        }
        let key = entry.file_name().to_string_lossy().to_string();
        for file in web_files(&entry.path()) {
            let text = fs::read_to_string(&file)?;
            for specifier in import_specifiers(&text) {
                if let Some(rest) = specifier.strip_prefix("@/modules/") {
                    let target = rest.split('/').next().unwrap_or("");
                    if target != key && target != WEB_KERNEL_PLUGIN {
                        cross_imports.push(format!("{}: {specifier}", relative(&web, &file)));
                    }
                }
                if specifier.starts_with("@/features") {
                    feature_imports.push(format!("{}: {specifier}", relative(&web, &file)));
                }
            }
            for _ in 0..text.matches("conflicts_with").count() {
                conflicts.push(relative(&web, &file));
            }
        }
    }

    let mut shell_words = Vec::new();
    for root in WEB_SHELL_ROOTS {
        for file in web_files(&web.join(root)) {
            let count = count_domain_words(&fs::read_to_string(&file)?);
            for _ in 0..count {
                shell_words.push(relative(&web, &file));
            }
        }
    }

    Ok(vec![
        Metric::new(
            "web_imports_across_plugins",
            "web plugin imports of another plugin's package",
            cross_imports,
        ),
        Metric::new(
            "web_feature_imports_from_plugins",
            "web plugin imports of @/features",
            feature_imports,
        ),
        Metric::new(
            "web_shell_domain_words",
            "domain words in the web shell, one finding per occurrence",
            shell_words,
        ),
        Metric::new(
            "web_conflict_usages",
            "conflicts_with usages in web plugins",
            conflicts,
        ),
    ])
}

fn discover_plugins(back: &Path) -> Result<Vec<Plugin>, Failure> {
    let mut plugins = Vec::new();
    for root in PLUGIN_ROOTS {
        let directory = back.join(root);
        if !directory.is_dir() {
            continue;
        }
        for entry in fs::read_dir(&directory)? {
            let entry = entry?;
            if entry.path().join("Cargo.toml").is_file() {
                plugins.push(Plugin {
                    key: entry.file_name().to_string_lossy().to_string(),
                    dir: entry.path(),
                });
            }
        }
    }
    plugins.sort_by(|left, right| left.key.cmp(&right.key));
    Ok(plugins)
}

fn layer_rank(layer: &str) -> u8 {
    match layer {
        "domain" => 1,
        _ => 0,
    }
}

fn read_layer(plugin: &Plugin) -> Result<String, String> {
    let text = fs::read_to_string(plugin.dir.join("plugin.toml"))
        .map_err(|_| "plugin.toml is missing".to_string())?;
    let document: toml::Table = text
        .parse()
        .map_err(|error| format!("plugin.toml is invalid: {error}"))?;
    let key = document.get("key").and_then(toml::Value::as_str);
    if key != Some(plugin.key.as_str()) {
        return Err(format!(
            "plugin.toml key {key:?} does not match the directory"
        ));
    }
    match document.get("layer").and_then(toml::Value::as_str) {
        Some(layer @ ("capability" | "domain")) => Ok(layer.to_string()),
        other => Err(format!(
            "plugin.toml layer {other:?} is not capability or domain"
        )),
    }
}

fn collect_tables(back: &Path, plugins: &[Plugin]) -> Result<Tables, Failure> {
    let mut owner_by_table = BTreeMap::new();
    for file in files_with_extension(&back.join(KERNEL_MIGRATIONS), "sql") {
        for table in created_tables(&fs::read_to_string(&file)?) {
            owner_by_table.insert(table, "kernel".to_string());
        }
    }
    for plugin in plugins {
        for file in files_with_extension(&plugin.dir.join("migrations"), "sql") {
            for table in created_tables(&fs::read_to_string(&file)?) {
                owner_by_table.entry(table).or_insert(plugin.key.clone());
            }
        }
    }
    Ok(Tables { owner_by_table })
}

impl Tables {
    fn is_foreign_to(&self, table: &str, plugin: &str) -> bool {
        self.owner_by_table
            .get(table)
            .is_some_and(|owner| owner != plugin && owner != "kernel")
    }
}

fn sql_table_references(tokens: &[String], tables: &Tables, plugin: &str) -> Vec<String> {
    tokens
        .windows(2)
        .filter(|pair| {
            SQL_TABLE_KEYWORDS.contains(&pair[0].as_str()) && tables.is_foreign_to(&pair[1], plugin)
        })
        .map(|pair| pair[1].clone())
        .collect()
}

fn created_tables(sql: &str) -> Vec<String> {
    let tokens = tokenize(sql);
    let mut tables = Vec::new();
    let mut index = 0;
    while index + 2 < tokens.len() {
        if tokens[index].eq_ignore_ascii_case("CREATE")
            && tokens[index + 1].eq_ignore_ascii_case("TABLE")
        {
            let mut cursor = index + 2;
            if tokens[cursor].eq_ignore_ascii_case("IF")
                && tokens
                    .get(cursor + 1)
                    .is_some_and(|word| word.eq_ignore_ascii_case("NOT"))
                && tokens
                    .get(cursor + 2)
                    .is_some_and(|word| word.eq_ignore_ascii_case("EXISTS"))
            {
                cursor += 3;
            }
            if let Some(name) = tokens.get(cursor) {
                tables.push(name.clone());
            }
            index = cursor;
        }
        index += 1;
    }
    tables
}

fn plugin_dependencies(manifest: &str) -> Vec<String> {
    let mut in_dependencies = false;
    let mut found = Vec::new();
    for line in manifest.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_dependencies = trimmed == "[dependencies]" || trimmed == "[dev-dependencies]";
            continue;
        }
        if in_dependencies && trimmed.contains(PLUGIN_CRATE_PREFIX) {
            found.push(trimmed.split('=').next().unwrap_or("").trim().to_string());
        }
    }
    found
}

fn constant_name(line: &str) -> Option<String> {
    let rest = line.trim().strip_prefix("pub const ")?;
    if rest.starts_with("fn ") {
        return None;
    }
    let name = rest.split(':').next()?.trim();
    rest.contains("from_static(").then(|| name.to_string())
}

fn permission_area(line: &str) -> Option<(String, String)> {
    let name = constant_name(line)?;
    let value = line.split("from_static(\"").nth(1)?;
    let area = value.split(['.', '"']).next()?.to_string();
    Some((name, area))
}

fn kernel_paths_in(text: &str) -> BTreeSet<String> {
    let mut paths = BTreeSet::new();
    let mut remaining = text;
    while let Some(position) = remaining.find("aphel_kernel::") {
        let after = &remaining[position + "aphel_kernel::".len()..];
        let path: Vec<&str> = after
            .split(|character: char| {
                !(character.is_ascii_alphanumeric() || character == '_' || character == ':')
            })
            .next()
            .unwrap_or("")
            .split("::")
            .filter(|segment| !segment.is_empty())
            .take(2)
            .collect();
        if path.len() == 2
            && path
                .iter()
                .all(|segment| segment.chars().all(|c| c.is_ascii_lowercase() || c == '_'))
        {
            paths.insert(path.join("::"));
        }
        remaining = after;
    }
    paths
}

fn count_domain_words(text: &str) -> usize {
    word_parts(text)
        .iter()
        .filter(|word| DOMAIN_WORDS.contains(&word.as_str()))
        .count()
}

fn word_parts(text: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut previous_lower = false;
    for character in text.chars() {
        if !character.is_ascii_alphanumeric() {
            flush(&mut parts, &mut current);
            previous_lower = false;
            continue;
        }
        if character.is_ascii_uppercase() && previous_lower {
            flush(&mut parts, &mut current);
        }
        previous_lower = character.is_ascii_lowercase() || character.is_ascii_digit();
        current.push(character.to_ascii_lowercase());
    }
    flush(&mut parts, &mut current);
    parts
}

fn flush(parts: &mut Vec<String>, current: &mut String) {
    if !current.is_empty() {
        parts.push(std::mem::take(current));
    }
}

fn tokenize(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    for character in text.chars() {
        if character.is_ascii_alphanumeric() || character == '_' {
            current.push(character);
        } else {
            flush(&mut tokens, &mut current);
        }
    }
    flush(&mut tokens, &mut current);
    tokens
}

fn import_specifiers(text: &str) -> Vec<String> {
    let mut specifiers = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        let is_import = trimmed.starts_with("import ")
            || trimmed.starts_with("export ")
            || trimmed.starts_with("} from ")
            || trimmed.contains("import(")
            || trimmed.starts_with("from ");
        if !is_import {
            continue;
        }
        for quote in ['\'', '"'] {
            let mut pieces = trimmed.split(quote);
            pieces.next();
            for (index, piece) in pieces.enumerate() {
                if index % 2 == 0 && (piece.starts_with("@/") || piece.starts_with('.')) {
                    specifiers.push(piece.to_string());
                }
            }
        }
    }
    specifiers
}

fn relative(root: &Path, file: &Path) -> String {
    file.strip_prefix(root)
        .unwrap_or(file)
        .to_string_lossy()
        .to_string()
}

fn rust_files(root: &Path) -> Vec<PathBuf> {
    files_with_extension(root, "rs")
}

fn web_files(root: &Path) -> Vec<PathBuf> {
    let mut files = files_with_extension(root, "ts");
    files.extend(files_with_extension(root, "tsx"));
    files
        .into_iter()
        .filter(|file| !file.to_string_lossy().ends_with(".d.ts"))
        .collect()
}

fn files_with_extension(root: &Path, extension: &str) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut pending = vec![root.to_path_buf()];
    while let Some(directory) = pending.pop() {
        let Ok(entries) = fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|value| value == extension) {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}
