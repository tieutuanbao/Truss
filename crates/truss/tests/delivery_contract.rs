//! Native role contracts for the shipped Delivery guidance.
//!
//! These tests deliberately keep the Markdown grammar test-local: the
//! installer does not execute the workflow, so production parser code would
//! misstate its runtime authority. Mutations are confined to in-memory or
//! temporary fixture copies.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

const CURRENT_ROLES: [&str; 8] = [
    "project-manager",
    "ba",
    "architect",
    "detailed-designer",
    "planner",
    "implement",
    "visual-engineering",
    "tester-debugger",
];

const LEGACY_SEVEN_ROLES: [&str; 7] = [
    "project-manager",
    "ba",
    "architect",
    "planner",
    "implement",
    "visual-engineering",
    "tester-debugger",
];

const LEGACY_FIVE_ROLES: [&str; 5] = ["plan", "plan-review", "implement", "review", "consult"];

type SuiteResult<T> = Result<T, String>;

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn payload_path(relative_path: &str) -> PathBuf {
    repository_root()
        .join("distribution/payload")
        .join(relative_path)
}

fn repository_path(relative_path: &str) -> PathBuf {
    repository_root().join(relative_path)
}

fn shipped(relative_path: &str) -> String {
    let path = payload_path(relative_path);
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read shipped {}: {error}", path.display()))
}

fn repository_file(relative_path: &str) -> String {
    let path = repository_path(relative_path);
    fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
}

fn mutate(source: &str, old: &str, new: &str) -> String {
    assert!(source.contains(old), "mutation target vanished: {old}");
    source.replace(old, new)
}

fn source_digest(source: &str) -> String {
    format!("{:x}", Sha256::digest(source.as_bytes()))
}

fn role_name(cell: &str) -> &str {
    cell.trim()
        .strip_prefix('`')
        .and_then(|value| value.strip_suffix('`'))
        .unwrap_or_else(|| panic!("role cell is not backquoted: {cell}"))
}

fn table_roles(source: &str) -> Vec<String> {
    source
        .lines()
        .filter(|line| line.trim_start().starts_with("| `"))
        .map(|line| role_name(line.split('|').nth(1).unwrap()).to_string())
        .collect()
}

fn require_exact_roles(found: &[String], expected: &[&str], surface: &str) -> SuiteResult<()> {
    if found.len() != expected.len() {
        return Err(format!(
            "{surface} has {} roles, expected {}",
            found.len(),
            expected.len()
        ));
    }
    for (index, expected_role) in expected.iter().enumerate() {
        if found[index] != *expected_role {
            if !found.contains(&(*expected_role).to_string()) {
                return Err(format!("{surface} is stale: missing `{expected_role}`"));
            }
            return Err(format!(
                "{surface} order is wrong at {index}: found `{}`, expected `{expected_role}`",
                found[index]
            ));
        }
    }
    Ok(())
}

struct ManagedBlock {
    before: String,
    block: String,
    after: String,
}

fn managed_block(source: &str) -> SuiteResult<ManagedBlock> {
    let begins = source.matches("<!-- delivery:begin -->").count();
    let ends = source.matches("<!-- delivery:end -->").count();
    if begins != 1 || ends != 1 {
        return Err(format!(
            "managed marker integrity failed (begin={begins}, end={ends})"
        ));
    }
    let start = source
        .find("<!-- delivery:begin -->")
        .expect("begin marker was counted");
    let end = source
        .find("<!-- delivery:end -->")
        .expect("end marker was counted");
    if end < start {
        return Err("managed markers are reversed".to_string());
    }
    if source[start..end]
        .find("\n| Role | Truss | Model | Effort |")
        .is_none()
    {
        return Err("managed block has no role tuple table".to_string());
    }

    Ok(ManagedBlock {
        before: source[..start].to_string(),
        block: source[start..end].to_string(),
        after: source[end..].to_string(),
    })
}

fn tuple_rows(source: &str) -> SuiteResult<BTreeMap<String, [String; 3]>> {
    let mut rows = BTreeMap::new();
    for line in source.lines().filter(|line| line.starts_with("| `")) {
        let columns: Vec<&str> = line.split('|').collect();
        if columns.len() != 6 {
            return Err(format!(
                "malformed tuple row has {} cells: {line}",
                columns.len() - 2
            ));
        }
        let role = role_name(columns[1]).to_string();
        let cells = [
            columns[2].trim().to_string(),
            columns[3].trim().to_string(),
            columns[4].trim().to_string(),
        ];
        if cells
            .iter()
            .any(|cell| cell.is_empty() || cell.contains('…'))
        {
            return Err(format!("incomplete tuple for `{role}`"));
        }
        if rows.insert(role.clone(), cells).is_some() {
            return Err(format!("duplicate tuple row for `{role}`"));
        }
    }
    if rows.is_empty() {
        return Err("managed block has no tuple rows".to_string());
    }
    Ok(rows)
}

fn tuple_line(role: &str, cells: &[String; 3]) -> String {
    format!("| `{role}` | {} | {} | {} |", cells[0], cells[1], cells[2])
}

fn require_rows_complete(
    rows: &BTreeMap<String, [String; 3]>,
    expected: &[&str],
    surface: &str,
) -> SuiteResult<()> {
    for role in expected {
        if !rows.contains_key(*role) {
            return Err(format!("{surface} is missing row `{role}`"));
        }
    }
    for role in rows.keys() {
        if !expected.contains(&role.as_str()) {
            return Err(format!("{surface} has unexpected role `{role}`"));
        }
    }
    Ok(())
}

enum NewDesignerTuple {
    Explicit([String; 3]),
    CopiedFromArchitect,
    Unresolved,
}

fn migrated_block(source: &str, new_designer: &NewDesignerTuple) -> SuiteResult<String> {
    let block = managed_block(source)?;
    let roles = table_roles(&block.block);
    let rows = tuple_rows(&block.block)?;

    let output_rows = if require_exact_roles(&roles, &CURRENT_ROLES, "managed block").is_ok()
        && require_rows_complete(&rows, &CURRENT_ROLES, "managed block").is_ok()
    {
        return Ok(source.to_string());
    } else if roles == LEGACY_SEVEN_ROLES {
        require_rows_complete(&rows, &LEGACY_SEVEN_ROLES, "legacy seven-role block")?;
        let cells = match new_designer {
            NewDesignerTuple::Explicit(cells) => cells.clone(),
            NewDesignerTuple::CopiedFromArchitect => {
                return Err("new detailed-designer tuple is copied from architect".to_string())
            }
            NewDesignerTuple::Unresolved => {
                return Err("new detailed-designer tuple is unresolved".to_string())
            }
        };
        let mut output = rows.clone();
        output.insert("detailed-designer".to_string(), cells);
        output
    } else if roles == LEGACY_FIVE_ROLES {
        require_rows_complete(&rows, &LEGACY_FIVE_ROLES, "legacy five-role block")?;
        let mappings = [
            (
                "project-manager",
                [
                    "current".to_string(),
                    "current".to_string(),
                    "current".to_string(),
                ],
            ),
            ("ba", rows["consult"].clone()),
            ("architect", rows["plan-review"].clone()),
            (
                "detailed-designer",
                match new_designer {
                    NewDesignerTuple::Explicit(cells) => cells.clone(),
                    NewDesignerTuple::CopiedFromArchitect => {
                        return Err(
                            "new detailed-designer tuple is copied from architect".to_string()
                        )
                    }
                    NewDesignerTuple::Unresolved => {
                        return Err("new detailed-designer tuple is unresolved".to_string())
                    }
                },
            ),
            ("planner", rows["plan"].clone()),
            ("implement", rows["implement"].clone()),
            ("visual-engineering", rows["implement"].clone()),
            ("tester-debugger", rows["review"].clone()),
        ];
        mappings
            .into_iter()
            .map(|(role, cells)| (role.to_string(), cells))
            .collect::<BTreeMap<_, _>>()
    } else {
        if roles
            .iter()
            .all(|role| LEGACY_SEVEN_ROLES.contains(&role.as_str()))
            && roles.len() < LEGACY_SEVEN_ROLES.len()
        {
            let missing = LEGACY_SEVEN_ROLES
                .iter()
                .find(|role| !roles.iter().any(|found| found == *role))
                .expect("a shorter role set has a missing row");
            return Err(format!(
                "legacy seven-role block is missing row `{missing}`"
            ));
        }
        return Err(format!(
            "unsupported or mixed managed role set: {}",
            roles.join(", ")
        ));
    };

    let absolute_table_start = block
        .block
        .find("\n| Role | Truss | Model | Effort |")
        .expect("table start was validated");
    let before_table = &block.block[..absolute_table_start + 1];
    let after_table_start = block.block[absolute_table_start..]
        .lines()
        .skip(1)
        .take_while(|line| line.starts_with('|'))
        .map(|line| line.len() + 1)
        .sum::<usize>()
        + absolute_table_start;
    let after_table = &block.block[after_table_start..];

    let mut replacement = String::from(before_table);
    replacement.push_str("| Role | Truss | Model | Effort |\n");
    replacement.push_str("| --- | --- | --- | --- |\n");
    for role in CURRENT_ROLES {
        let cells = output_rows.get(role).expect("mapped role row");
        replacement.push_str(&tuple_line(role, cells));
        replacement.push('\n');
    }
    replacement.push_str(after_table);

    Ok(format!("{}{}{}", block.before, replacement, block.after))
}

fn apply_migration_file(path: &Path, new_designer: &NewDesignerTuple) -> SuiteResult<String> {
    let before = fs::read_to_string(path).unwrap_or_else(|error| {
        panic!("cannot read migration fixture {}: {error}", path.display())
    });
    let after = migrated_block(&before, new_designer)?;
    fs::write(path, &after).unwrap_or_else(|error| {
        panic!("cannot write migrated fixture {}: {error}", path.display())
    });
    Ok(after)
}

fn delivery_policy(source: &str) -> SuiteResult<()> {
    let roles = table_roles(source);
    require_exact_roles(&roles, &CURRENT_ROLES, "delivery role table")?;

    let project_row = source
        .lines()
        .find(|line| line.starts_with("| `project-manager` |"))
        .ok_or("project-manager row is missing")?;
    if !project_row.contains("current session") {
        return Err("authority defect: dispatched project-manager".to_string());
    }
    if !source.contains("`project-manager` is never dispatched") {
        return Err("authority defect: project-manager dispatch policy is missing".to_string());
    }

    let visual_row = source
        .lines()
        .find(|line| line.starts_with("| `visual-engineering` |"))
        .ok_or("visual-engineering row is missing")?;
    if !visual_row.contains("when the task needs it") {
        return Err("authority defect: visual-engineering is mandatory".to_string());
    }
    if !source.contains("`visual-engineering` remains\nconditional") {
        return Err("authority defect: visual-engineering conditionality is missing".to_string());
    }
    Ok(())
}

fn architectural_pipeline(source: &str) -> SuiteResult<()> {
    if !source
        .contains("The Architectural sequence is `architect` → `detailed-designer` → `planner`.")
    {
        return Err("pipeline order must be architect -> detailed-designer -> planner".to_string());
    }
    if !source
        .contains("Every Architectural run requires a fresh\n`detailed-designer` dispatch after")
    {
        return Err("detailed design must be mandatory for every Architectural run".to_string());
    }
    if !source.contains("non-UI change are not\nexemptions") {
        return Err("a non-UI change is not an exemption".to_string());
    }
    if !source.contains("A completed architect output precedes a successfully completed fresh") {
        return Err(
            "completed architect and successful design-dispatch prerequisites are missing"
                .to_string(),
        );
    }
    Ok(())
}

fn ownership_policy(source: &str) -> SuiteResult<()> {
    let detailed_row = source
        .lines()
        .find(|line| line.starts_with("| `detailed-designer` |"))
        .ok_or("detailed-designer ownership row is missing")?;
    for required in [
        "API signatures",
        "state machines",
        "error handling and error codes",
        "placeholder-body skeleton",
        "ambiguity audit",
    ] {
        if !detailed_row.contains(required) {
            return Err(format!("detailed-designer does not own `{required}`"));
        }
    }

    let planner_row = source
        .lines()
        .find(|line| line.starts_with("| `planner` |"))
        .ok_or("planner ownership row is missing")?;
    for forbidden in ["API signatures", "placeholder-body skeleton"] {
        if planner_row.contains(forbidden) {
            return Err(format!("planner wrongly owns `{forbidden}`"));
        }
    }
    if source.contains("implementer decides missing structure") {
        return Err("implementer is not authorized to decide structure".to_string());
    }
    Ok(())
}

fn recovery_policy(source: &str) -> SuiteResult<()> {
    if source.contains("use best judgment") {
        return Err("best-judgment structural recovery is forbidden".to_string());
    }
    for required in [
        "pauses the affected work",
        "returns it through Control to\n`detailed-designer`",
        "material correction follows\nrenewed design approval and affected re-planning",
        "before coding resumes",
    ] {
        if !source.contains(required) {
            return Err(format!("recovery contract is missing `{required}`"));
        }
    }
    Ok(())
}

fn design_categories(design: &str) -> SuiteResult<Vec<String>> {
    let start = design
        .find("## Design categories\n")
        .ok_or("design category section is missing")?;
    let end = design[start..]
        .find("## Non-applicability record")
        .map(|relative| start + relative)
        .ok_or("design category section is unterminated")?;
    let section = &design[start..end];
    let mut categories = Vec::new();
    let mut table_lines = section
        .lines()
        .skip_while(|line| !line.starts_with("| ---"));
    table_lines.next();
    for line in table_lines {
        if !line.starts_with("| ") {
            break;
        }
        let columns: Vec<&str> = line.split('|').collect();
        if columns.len() < 5 {
            return Err(format!("malformed category row: {line}"));
        }
        categories.push(columns[1].trim().to_string());
    }
    if categories.is_empty() {
        return Err("design category table is empty".to_string());
    }
    Ok(categories)
}

fn audit_categories(audit: &str) -> SuiteResult<BTreeMap<String, bool>> {
    let start = audit
        .find("## Category coverage\n")
        .ok_or("audit category section is missing")?;
    let end = audit[start..]
        .find("## Ambiguity review")
        .map(|relative| start + relative)
        .ok_or("audit category section is unterminated")?;
    let mut categories = BTreeMap::new();
    for line in audit[start..end]
        .lines()
        .skip_while(|line| !line.starts_with("| Modules"))
    {
        if !line.starts_with("| ") {
            break;
        }
        let columns: Vec<&str> = line.split('|').collect();
        if columns.len() < 5 {
            return Err(format!("malformed audit coverage row: {line}"));
        }
        categories.insert(
            columns[1].trim().to_string(),
            columns[3].trim().eq_ignore_ascii_case("yes"),
        );
    }
    if categories.is_empty() {
        return Err("audit category table is empty".to_string());
    }
    Ok(categories)
}

fn design_category_section(design: &str) -> &str {
    let start = design
        .find("## Design categories\n")
        .expect("design category section was validated");
    let end = design[start..]
        .find("## Non-applicability record")
        .expect("design category end was validated");
    &design[start..start + end]
}

fn package_defects(design: &str, audit: &str) -> Vec<String> {
    let mut defects = Vec::new();
    let design_categories = match design_categories(design) {
        Ok(categories) => categories,
        Err(error) => return vec![error],
    };
    let audit_categories = match audit_categories(audit) {
        Ok(categories) => categories,
        Err(error) => return vec![error],
    };

    for category in &design_categories {
        if !audit_categories.contains_key(category.as_str()) {
            defects.push(format!("audit is missing category `{category}`"));
        } else if !audit_categories[category.as_str()] {
            defects.push(format!("audit category `{category}` is incomplete"));
        }
    }
    for line in design_category_section(design)
        .lines()
        .filter(|line| line.starts_with("| "))
    {
        let category = match line.split('|').nth(1) {
            Some(category) => category.trim(),
            None => continue,
        };
        if !design_categories.contains(&category.to_string()) {
            continue;
        }
        let decision = line.split('|').nth(2).unwrap_or_default().trim();
        let applicability = line.split('|').nth(4).unwrap_or_default().trim();
        if decision != "resolved" || applicability != "applicable" {
            defects.push(format!(
                "design category `{category}` lacks a substantive resolved decision"
            ));
        }
    }
    for category in audit_categories.keys() {
        if !design_categories.contains(category) {
            defects.push(format!("audit has absent design category `{category}`"));
        }
    }
    defects
}

fn complete_design_fixture(template: &str) -> String {
    let categories = design_categories(template).expect("template category grammar");
    let rows = categories
        .iter()
        .map(|category| format!("| {category} | resolved | design.md#{category} | applicable |"))
        .collect::<Vec<_>>()
        .join("\n");
    mutate(
        template,
        "| Modules and file structure | | | |\n| API signatures | | | |\n| Inputs and outputs | | | |\n| Interfaces | | | |\n| Dependency order and sequence | | | |\n| Data structures | | | |\n| Enums and discriminants | | | |\n| State machines and state behavior | | | |\n| Error handling and error codes | | | |",
        &rows,
    )
}

fn complete_audit_fixture(template: &str) -> String {
    let categories = design_categories(template).expect("template category grammar");
    let rows = categories
        .iter()
        .map(|category| format!("| {category} | audited | yes |"))
        .collect::<Vec<_>>()
        .join("\n");
    mutate(
        template,
        "| Modules and file structure | | |\n| API signatures | | |\n| Inputs and outputs | | |\n| Interfaces | | |\n| Dependency order and sequence | | |\n| Data structures | | |\n| Enums and discriminants | | |\n| State machines and state behavior | | |\n| Error handling and error codes | | |",
        &rows,
    )
}

struct ReadinessInput<'a> {
    audit: Option<&'a str>,
    design: Option<&'a str>,
    categories_complete: bool,
    structural_proof_passed: bool,
    placeholder_proof_passed: bool,
    digest_current: bool,
    transitions_conflict: bool,
    architect_complete: bool,
    design_dispatch_complete: bool,
}

fn derive_readiness(input: &ReadinessInput) -> SuiteResult<&'static str> {
    let audit = input.audit.ok_or("NOT_READY: audit artifact is absent")?;
    if !audit.contains("## Package identity") {
        return Err("NOT_READY: audit package identity is missing".to_string());
    }
    let design = input
        .design
        .ok_or("NOT_READY: design artifact identity is absent")?;
    if !audit.contains(&source_digest(design)) {
        return Err("NOT_READY: audit digest is stale".to_string());
    }
    if !input.categories_complete {
        return Err("NOT_READY: design category coverage is incomplete".to_string());
    }
    if !input.architect_complete {
        return Err("NOT_READY: architect output is incomplete".to_string());
    }
    if !input.design_dispatch_complete {
        return Err("NOT_READY: detailed-designer dispatch is incomplete".to_string());
    }
    if !input.structural_proof_passed {
        return Err("NOT_READY: structural proof did not pass".to_string());
    }
    if !input.placeholder_proof_passed {
        return Err("NOT_READY: placeholder-boundary proof did not pass".to_string());
    }
    if !input.digest_current {
        return Err("NOT_READY: audit digest is stale".to_string());
    }
    if input.transitions_conflict {
        return Err("NOT_READY: state transitions conflict".to_string());
    }
    Ok("READY_FOR_PLANNING")
}

fn function_body(source: &str) -> SuiteResult<&str> {
    let start = source
        .find("fn sample")
        .ok_or("sample function is missing")?;
    let open = source[start..]
        .find('{')
        .map(|relative| start + relative)
        .ok_or("sample function has no body")?;
    let close = source[open..]
        .find('}')
        .map(|relative| open + relative)
        .ok_or("sample function body is unterminated")?;
    Ok(source[open + 1..close].trim())
}

const PLACEHOLDER_CANON: &str = "todo!(\"detailed-design placeholder\")";

fn rust_skeleton_defect(source: &str) -> SuiteResult<()> {
    let body = function_body(source)?;
    if body != PLACEHOLDER_CANON {
        return Err(format!(
            "placeholder boundary failed: unsupported body `{body}`"
        ));
    }
    Ok(())
}

fn markdown_skeleton_defect(source: &str) -> SuiteResult<()> {
    for required in [
        "# Detailed design",
        "| Category | Decision |",
        "| Modules and file structure |",
        "# Skeleton inventory",
        "| Target path |",
        "## Structural proof",
        "## Placeholder-boundary proof",
        "# Design audit",
        "## Package identity",
    ] {
        if !source.contains(required) {
            return Err(format!(
                "Markdown skeleton is missing structure `{required}`"
            ));
        }
    }
    if !source.contains(PLACEHOLDER_CANON) {
        return Err("Markdown skeleton does not bind the Rust placeholder canon".to_string());
    }
    Ok(())
}

fn authority_policy(source: &str) -> SuiteResult<()> {
    delivery_policy(source)?;
    if !source.contains("independent review/acceptance")
        && !source.contains("Code review uses the existing `tester-debugger` role")
    {
        return Err("independent review/acceptance authority is missing".to_string());
    }
    for required in ["## Two human gates", "consumer-local"] {
        if !source.contains(required) {
            return Err(format!(
                "retained authority boundary is missing `{required}`"
            ));
        }
    }
    if !source.contains("release") || !source.contains("authorize") {
        return Err("authorized release boundary is missing".to_string());
    }
    Ok(())
}

fn validate_stale_surfaces(delivery: &str, setup: &str, docs: &str) -> SuiteResult<()> {
    let delivery_roles = table_roles(delivery);
    require_exact_roles(&delivery_roles, &CURRENT_ROLES, "delivery")?;
    delivery_policy(delivery)?;
    architectural_pipeline(delivery)?;

    let setup_start = setup
        .find("## Eight roles\n")
        .ok_or("setup role section is missing")?;
    let setup_end = setup[setup_start..]
        .find("## Two paths")
        .map(|relative| setup_start + relative)
        .ok_or("setup role section is unterminated")?;
    let setup_roles = table_roles(&setup[setup_start..setup_end]);
    require_exact_roles(&setup_roles, &CURRENT_ROLES, "setup")?;
    for phrase in [
        "A block that already has the eight roles is preserved byte-identical unless",
        "`detailed-designer` tuple is always explicitly resolved separately",
        "Replace only the managed block",
    ] {
        if !setup.contains(phrase) {
            return Err(format!(
                "setup migration contract is stale: missing `{phrase}`"
            ));
        }
    }

    let docs_roles: Vec<String> = docs
        .lines()
        .filter_map(|line| line.strip_prefix("- `"))
        .map(|line| line[..line.find('`').expect("closing role quote")].to_string())
        .collect();
    require_exact_roles(&docs_roles, &CURRENT_ROLES[1..], "docs role list")?;
    if !docs.contains("for eight roles in total") {
        return Err("docs are stale: role count is wrong".to_string());
    }
    if !docs.contains("architect → detailed-designer → planner") {
        return Err("docs are stale: architectural pipeline is wrong".to_string());
    }
    for phrase in [
        "preserves every existing role's Truss, Model, and Effort\ncell byte-for-byte",
        "asks you to resolve the new `detailed-designer` tuple",
        "ordinary eight-role rerun unchanged",
    ] {
        if !docs.contains(phrase) {
            return Err(format!(
                "docs migration contract is stale: missing `{phrase}`"
            ));
        }
    }
    Ok(())
}

#[test]
fn tc01_current_role_table_is_exactly_ordered() {
    let delivery = shipped(".agents/skills/delivery/SKILL.md");
    let roles = table_roles(&delivery);
    require_exact_roles(&roles, &CURRENT_ROLES, "delivery role table").unwrap();
    delivery_policy(&delivery).unwrap();

    let stale_allowlist = mutate(
        &delivery,
        "| `detailed-designer` | The detailed design package",
        "| `renamed-designer` | The detailed design package",
    );
    let error = require_exact_roles(
        &table_roles(&stale_allowlist),
        &CURRENT_ROLES,
        "legacy seven-role allowlist",
    )
    .unwrap_err();
    assert!(error.contains("`detailed-designer`"), "{error}");
}

#[test]
fn tc02_architectural_pipeline_has_completion_prerequisites() {
    let delivery = shipped(".agents/skills/delivery/SKILL.md");
    architectural_pipeline(&delivery).unwrap();

    let request_only = mutate(
        &delivery,
        "The Architectural sequence is `architect` → `detailed-designer` → `planner`.",
        "An implementer may request `detailed-designer` before coding.",
    );
    assert!(architectural_pipeline(&request_only)
        .unwrap_err()
        .contains("pipeline order"));

    let after_planner = mutate(
        &delivery,
        "The Architectural sequence is `architect` → `detailed-designer` → `planner`.",
        "The Architectural sequence is `architect` → `planner` → `detailed-designer`.",
    );
    assert!(architectural_pipeline(&after_planner)
        .unwrap_err()
        .contains("pipeline order"));
}

#[test]
fn tc03_each_required_design_category_is_discriminated() {
    let template = shipped(".agents/skills/delivery/templates/detailed-design.md");
    let design = complete_design_fixture(&template);
    let audit = complete_audit_fixture(&template);
    assert!(package_defects(&design, &audit).is_empty());

    assert!(audit_categories(&audit)
        .unwrap()
        .values()
        .all(|complete| *complete));
    for category in design_categories(&template).unwrap() {
        let removed_design = mutate(
            &design,
            &format!("| {category} | resolved | design.md#{category} | applicable |\n"),
            "",
        );
        let defects = package_defects(&removed_design, &audit);
        assert!(
            defects.iter().any(|defect| defect.contains(&category)),
            "removing `{category}` was not named: {defects:?}"
        );

        let incomplete_audit = audit
            .lines()
            .map(|line| {
                if line.starts_with(&format!("| {category} |")) {
                    format!("| {category} | audited | no |")
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n");
        let defects = package_defects(&design, &incomplete_audit);
        assert!(
            defects.iter().any(|defect| defect.contains(&category)),
            "incomplete `{category}` audit was not named: {defects:?}"
        );
    }

    let polished_counterexample = mutate(
        &design,
        "| State machines and state behavior | resolved | design.md#State machines and state behavior | applicable |\n| Error handling and error codes | resolved | design.md#Error handling and error codes | applicable |",
        "| State machines and state behavior | polished later | design.md#state | N/A |\n| Error handling and error codes | polished later | design.md#errors | N/A |",
    );
    let defects = package_defects(&polished_counterexample, &audit);
    assert!(defects
        .iter()
        .any(|defect| defect.contains("State machines and state behavior")));
    assert!(defects
        .iter()
        .any(|defect| defect.contains("Error handling and error codes")));
}

#[test]
fn tc04_rust_and_markdown_skeletons_enforce_placeholder_boundaries() {
    let positive = format!("fn sample() {{\n    {PLACEHOLDER_CANON}\n}}\n");
    rust_skeleton_defect(&positive).unwrap();

    let behavior_filled = "fn sample() -> usize {\n    let value = input() + 1;\n    todo!(\"detailed-design placeholder\")\n}\n";
    let error = rust_skeleton_defect(behavior_filled).unwrap_err();
    assert!(error.contains("behavior-filled") || error.contains("unsupported body"));

    let code_first =
        format!("fn sample() {{\n    let configured = 1;\n    {PLACEHOLDER_CANON}\n}}");
    assert!(rust_skeleton_defect(&code_first).is_err());

    let template = shipped(".agents/skills/delivery/templates/detailed-design.md");
    markdown_skeleton_defect(&template).unwrap();
    let prose_only =
        "# Detailed design\n\nThe structure is obvious and the implementer can choose it.\n";
    let error = markdown_skeleton_defect(prose_only).unwrap_err();
    assert!(error.contains("missing structure"), "{error}");
}

#[test]
fn tc05_readiness_is_derived_from_evidence_not_authority_claim() {
    let template = shipped(".agents/skills/delivery/templates/detailed-design.md");
    let design = complete_design_fixture(&template);
    let audit = complete_audit_fixture(&template);
    let audit = mutate(
        &audit,
        "| `design.md` | | |",
        &format!("| `design.md` | design.md | {} |", source_digest(&design)),
    );
    let complete = ReadinessInput {
        audit: Some(&audit),
        design: Some(&design),
        categories_complete: true,
        structural_proof_passed: true,
        placeholder_proof_passed: true,
        digest_current: true,
        transitions_conflict: false,
        architect_complete: true,
        design_dispatch_complete: true,
    };
    assert_eq!(derive_readiness(&complete).unwrap(), "READY_FOR_PLANNING");

    let absent = ReadinessInput {
        audit: None,
        ..complete
    };
    assert!(derive_readiness(&absent)
        .unwrap_err()
        .contains("audit artifact is absent"));

    let missing_category = ReadinessInput {
        categories_complete: false,
        ..complete
    };
    assert!(derive_readiness(&missing_category)
        .unwrap_err()
        .contains("coverage is incomplete"));

    let conflict = ReadinessInput {
        transitions_conflict: true,
        ..complete
    };
    assert!(derive_readiness(&conflict)
        .unwrap_err()
        .contains("state transitions conflict"));

    let stale = ReadinessInput {
        digest_current: false,
        ..complete
    };
    assert!(derive_readiness(&stale).unwrap_err().contains("stale"));

    let unevidenced = ReadinessInput {
        audit: Some(&mutate(
            &audit,
            "`Readiness: READY_FOR_PLANNING | NOT_READY`",
            "`Readiness: READY_FOR_PLANNING`",
        )),
        structural_proof_passed: false,
        placeholder_proof_passed: false,
        ..complete
    };
    assert!(derive_readiness(&unevidenced)
        .unwrap_err()
        .contains("proof did not pass"));
}

#[test]
fn tc06_structural_ownership_is_not_transferred() {
    let delivery = shipped(".agents/skills/delivery/SKILL.md");
    ownership_policy(&delivery).unwrap();

    let planner_signature_owner = mutate(
        &delivery,
        "| `detailed-designer` | The detailed design package: modules and files, API signatures,",
        "| `detailed-designer` | The detailed design package: modules and files,",
    );
    assert!(ownership_policy(&planner_signature_owner)
        .unwrap_err()
        .contains("does not own `API signatures`"));

    let implementer_decides = mutate(
        &delivery,
        "no\nbest-judgment structural choice is authorized",
        "the\nimplementer decides missing structure",
    );
    assert!(ownership_policy(&implementer_decides)
        .unwrap_err()
        .contains("implementer is not authorized"));
}

#[test]
fn tc07_structural_recovery_returns_to_design_and_replans() {
    let delivery = shipped(".agents/skills/delivery/SKILL.md");
    recovery_policy(&delivery).unwrap();

    let best_judgment = mutate(
        &delivery,
        "no\nbest-judgment structural choice is authorized",
        "use best judgment\nand continue coding",
    );
    let error = recovery_policy(&best_judgment).unwrap_err();
    assert!(error.contains("best-judgment"), "{error}");
}

#[test]
fn tc08_role_authority_mutations_are_rejected() {
    let delivery = shipped(".agents/skills/delivery/SKILL.md");
    authority_policy(&delivery).unwrap();

    let dispatched_pm = mutate(
        &delivery,
        "authorized release | current session |",
        "authorized release | fresh dispatched worker |",
    );
    assert!(authority_policy(&dispatched_pm)
        .unwrap_err()
        .contains("dispatched project-manager"));

    let mandatory_visual = mutate(
        &delivery,
        "including relevant code, docs, and tests | fresh dispatched worker when the task needs it |",
        "including relevant code, docs, and tests | fresh dispatched worker |",
    );
    assert!(authority_policy(&mandatory_visual)
        .unwrap_err()
        .contains("visual-engineering is mandatory"));
}

fn legacy_block(roles_and_tuples: &[(&str, &str, &str, &str)]) -> String {
    let mut output = String::from("before bytes\n<!-- delivery:begin -->\n## Delivery\n\nPreferences:\n| Role | Truss | Model | Effort |\n| --- | --- | --- | --- |\n");
    for (role, truss, model, effort) in roles_and_tuples {
        output.push_str(&format!("| `{role}` | {truss} | {model} | {effort} |\n"));
    }
    output.push_str("<!-- delivery:end -->\nafter bytes\n");
    output
}

#[test]
fn tc09_managed_block_migration_is_complete_preserving_and_fail_closed() {
    let explicit = NewDesignerTuple::Explicit([
        "design-truss".to_string(),
        "design-model".to_string(),
        "high".to_string(),
    ]);
    let five = legacy_block(&[
        ("plan", "planner-truss", "planner-model", "high"),
        (
            "plan-review",
            "architect-truss",
            "architect-model",
            "default",
        ),
        ("implement", "shared-truss", "shared-model", "medium"),
        ("review", "review-truss", "review-model", "high"),
        ("consult", "ba-truss", "ba-model", "high"),
    ]);
    let migrated_five = migrated_block(&five, &explicit).unwrap();
    assert!(migrated_five.starts_with("before bytes\n"));
    assert!(migrated_five.ends_with("after bytes\n"));
    assert!(migrated_five.contains("| `project-manager` | current | current | current |"));
    assert!(migrated_five.contains("| `planner` | planner-truss | planner-model | high |"));
    assert!(migrated_five.contains("| `architect` | architect-truss | architect-model | default |"));
    assert!(migrated_five.contains("| `implement` | shared-truss | shared-model | medium |"));
    assert!(
        migrated_five.contains("| `visual-engineering` | shared-truss | shared-model | medium |")
    );
    assert!(migrated_five.contains("| `detailed-designer` | design-truss | design-model | high |"));
    assert_eq!(
        migrated_block(&migrated_five, &explicit).unwrap(),
        migrated_five
    );

    let seven = legacy_block(&[
        ("project-manager", "pm", "pm", "current"),
        ("ba", "ba", "ba", "high"),
        ("architect", "architect", "architect", "default"),
        ("planner", "custom-planner", "custom-model", "xhigh"),
        ("implement", "impl", "impl", "low"),
        ("visual-engineering", "visual", "visual", "high"),
        ("tester-debugger", "tester", "tester", "high"),
    ]);
    let migrated_seven = migrated_block(&seven, &explicit).unwrap();
    for row in seven.lines().filter(|line| line.starts_with("| `")) {
        assert!(migrated_seven.contains(row));
    }
    assert!(migrated_seven.contains("| `detailed-designer` | design-truss | design-model | high |"));
    let eight = legacy_block(&[
        ("project-manager", "pm", "pm", "current"),
        ("ba", "ba", "ba", "high"),
        ("architect", "architect", "architect", "default"),
        ("detailed-designer", "design", "design", "high"),
        ("planner", "planner", "planner", "high"),
        ("implement", "impl", "impl", "low"),
        ("visual-engineering", "visual", "visual", "high"),
        ("tester-debugger", "tester", "tester", "high"),
    ]);
    assert_eq!(
        migrated_block(&eight, &NewDesignerTuple::Unresolved).unwrap(),
        eight
    );

    let fixture_directory = tempfile::tempdir().unwrap();
    let fixture_path = fixture_directory.path().join("AGENTS.md");
    for (name, fixture, tuple, expected_diagnostic) in [
        (
            "unterminated",
            seven.replace("<!-- delivery:end -->", ""),
            NewDesignerTuple::Explicit([
                "design".to_string(),
                "design".to_string(),
                "high".to_string(),
            ]),
            "marker integrity failed",
        ),
        (
            "duplicate",
            seven.replace(
                "| `ba` | ba | ba | high |\n",
                "| `ba` | ba | ba | high |\n| `ba` | ba-2 | ba-2 | high |\n",
            ),
            NewDesignerTuple::Explicit([
                "design".to_string(),
                "design".to_string(),
                "high".to_string(),
            ]),
            "duplicate tuple row for `ba`",
        ),
        (
            "mixed",
            seven.replace(
                "| `planner` |",
                "| `plan` | mixed | mixed | mixed |\n| `planner` |",
            ),
            NewDesignerTuple::Explicit([
                "design".to_string(),
                "design".to_string(),
                "high".to_string(),
            ]),
            "unsupported or mixed managed role set",
        ),
        (
            "missing-row",
            seven.replace(
                "| `planner` | custom-planner | custom-model | xhigh |\n",
                "",
            ),
            NewDesignerTuple::Explicit([
                "design".to_string(),
                "design".to_string(),
                "high".to_string(),
            ]),
            "missing row `planner`",
        ),
        (
            "unresolved",
            seven.clone(),
            NewDesignerTuple::Unresolved,
            "tuple is unresolved",
        ),
        (
            "architect-copied",
            seven.clone(),
            NewDesignerTuple::CopiedFromArchitect,
            "tuple is copied from architect",
        ),
    ] {
        fs::write(&fixture_path, &fixture).unwrap();
        let error = apply_migration_file(&fixture_path, &tuple).unwrap_err();
        assert!(error.contains(expected_diagnostic), "{name}: {error}");
        assert_eq!(
            fs::read_to_string(&fixture_path).unwrap(),
            fixture,
            "{name} bytes changed"
        );
    }
}

#[test]
fn tc10_source_surfaces_agree_and_stale_copies_fail() {
    let delivery = shipped(".agents/skills/delivery/SKILL.md");
    let setup = shipped(".agents/skills/delivery-setup/SKILL.md");
    let docs = repository_file("docs/delivery.md");
    validate_stale_surfaces(&delivery, &setup, &docs).unwrap();

    let stale_setup = mutate(
        &setup,
        "| `detailed-designer` | The detailed design package",
        "",
    );
    let error = validate_stale_surfaces(&delivery, &stale_setup, &docs).unwrap_err();
    assert!(error.contains("setup"), "{error}");

    let stale_docs = mutate(
        &docs,
        "architect → detailed-designer → planner",
        "architect → planner",
    );
    let error = validate_stale_surfaces(&delivery, &setup, &stale_docs).unwrap_err();
    assert!(error.contains("docs"), "{error}");

    let manifest = repository_file("scripts/delivery-install-files.txt");
    for path in [
        ".agents/skills/delivery/SKILL.md",
        ".agents/skills/delivery/templates/detailed-design.md",
        ".agents/skills/delivery-setup/SKILL.md",
    ] {
        assert!(
            manifest.lines().any(|line| line.trim() == path),
            "delivery population is missing {path}"
        );
    }
}

#[test]
fn tc11_unsupported_skeleton_grammar_fails_closed() {
    let template = shipped(".agents/skills/delivery/templates/detailed-design.md");
    markdown_skeleton_defect(&template).unwrap();

    let unsupported = "fn sample() {\n    unimplemented!(\"will finish later\")\n}";
    let error = rust_skeleton_defect(unsupported).unwrap_err();
    assert!(error.contains("unsupported body"), "{error}");
}
