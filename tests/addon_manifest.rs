use anesis::addons::manifest::*;

// ── Minimal manifest ──────────────────────────────────────────────────────────

#[test]
fn minimal_manifest_deserializes() {
  let yaml = r#"
schema_version: "1.0"
id: test-addon
name: Test Addon
version: "1.0.0"
description: A test addon
author: Test User
"#;
  let manifest: AddonManifest = serde_yaml::from_str(yaml).unwrap();
  assert_eq!(manifest.schema_version, "1.0");
  assert_eq!(manifest.id, "test-addon");
  assert_eq!(manifest.name, "Test Addon");
  assert_eq!(manifest.version, "1.0.0");
  assert_eq!(manifest.description, "A test addon");
  assert_eq!(manifest.author, "Test User");
  assert!(manifest.requires.is_empty());
  assert!(manifest.inputs.is_empty());
  assert!(manifest.detect.is_empty());
  assert!(manifest.variants.is_empty());
}

#[test]
fn manifest_with_requires_deserializes() {
  let yaml = r#"
schema_version: "1.0"
id: my-addon
name: My Addon
version: "0.1.0"
description: desc
author: author
requires:
  - node
  - npm
"#;
  let manifest: AddonManifest = serde_yaml::from_str(yaml).unwrap();
  assert_eq!(manifest.requires, ["node", "npm"]);
}

// ── InputDef ─────────────────────────────────────────────────────────────────

#[test]
fn input_type_text_deserializes() {
  let yaml = r#"
name: project-name
type: text
description: The project name
required: true
"#;
  let input: InputDef = serde_yaml::from_str(yaml).unwrap();
  assert_eq!(input.name, "project-name");
  assert!(matches!(input.input_type, InputType::Text));
  assert!(input.required);
  assert!(input.default.is_none());
}

#[test]
fn input_type_boolean_deserializes() {
  let yaml = r#"
name: use-typescript
type: boolean
default: "true"
"#;
  let input: InputDef = serde_yaml::from_str(yaml).unwrap();
  assert!(matches!(input.input_type, InputType::Boolean));
  assert_eq!(input.default.as_deref(), Some("true"));
}

#[test]
fn input_type_select_with_options_deserializes() {
  let yaml = r#"
name: package-manager
type: select
options:
  - npm
  - yarn
  - pnpm
"#;
  let input: InputDef = serde_yaml::from_str(yaml).unwrap();
  assert!(matches!(input.input_type, InputType::Select));
  assert_eq!(input.options, ["npm", "yarn", "pnpm"]);
}

// ── DetectBlock & DetectRule ──────────────────────────────────────────────────

#[test]
fn detect_block_file_exists_rule() {
  let yaml = r#"
id: has-package-json
rules:
  - type: file_exists
    file: package.json
"#;
  let block: DetectBlock = serde_yaml::from_str(yaml).unwrap();
  assert_eq!(block.id, "has-package-json");
  assert_eq!(block.rules.len(), 1);
  assert!(matches!(
    &block.rules[0],
    DetectRule::FileExists { file, negate } if file == "package.json" && !negate
  ));
}

#[test]
fn detect_block_file_contains_rule() {
  let yaml = r#"
id: uses-react
rules:
  - type: file_contains
    file: package.json
    contains: '"react"'
    negate: false
"#;
  let block: DetectBlock = serde_yaml::from_str(yaml).unwrap();
  assert!(matches!(
    &block.rules[0],
    DetectRule::FileContains { file, contains, negate }
      if file == "package.json" && contains.contains("react") && !negate
  ));
}

#[test]
fn detect_block_json_contains_rule() {
  let yaml = r#"
id: check-engine
rules:
  - type: json_contains
    file: package.json
    key_path: engines.node
    value: ">=18"
"#;
  let block: DetectBlock = serde_yaml::from_str(yaml).unwrap();
  assert!(matches!(
    &block.rules[0],
    DetectRule::JsonContains { file, key_path, value, .. }
      if file == "package.json" && key_path == "engines.node" && value.as_deref() == Some(">=18")
  ));
}

#[test]
fn detect_block_match_mode_all() {
  let yaml = r#"
id: strict
match: all
rules:
  - type: file_exists
    file: tsconfig.json
"#;
  let block: DetectBlock = serde_yaml::from_str(yaml).unwrap();
  assert!(matches!(block.match_mode, MatchMode::All));
}

#[test]
fn detect_block_match_mode_defaults_to_any() {
  let yaml = r#"
id: loose
rules:
  - type: file_exists
    file: package.json
"#;
  let block: DetectBlock = serde_yaml::from_str(yaml).unwrap();
  assert!(matches!(block.match_mode, MatchMode::Any));
}

// ── Step variants ─────────────────────────────────────────────────────────────

#[test]
fn copy_step_deserializes() {
  let yaml = r#"
type: copy
src: templates/config.ts
dest: src/config.ts
"#;
  let step: Step = serde_yaml::from_str(yaml).unwrap();
  assert!(matches!(step, Step::Copy(_)));
  if let Step::Copy(s) = step {
    assert_eq!(s.src, "templates/config.ts");
    assert_eq!(s.dest, "src/config.ts");
    assert!(matches!(s.if_exists, IfExists::Overwrite));
  }
}

#[test]
fn copy_step_if_exists_skip() {
  let yaml = r#"
type: copy
src: a
dest: b
if_exists: skip
"#;
  let step: Step = serde_yaml::from_str(yaml).unwrap();
  if let Step::Copy(s) = step {
    assert!(matches!(s.if_exists, IfExists::Skip));
  }
}

#[test]
fn create_step_deserializes() {
  let yaml = r#"
type: create
path: .env
content: "PORT=3000\n"
"#;
  let step: Step = serde_yaml::from_str(yaml).unwrap();
  assert!(matches!(step, Step::Create(_)));
}

#[test]
fn inject_step_with_file_target_and_after() {
  let yaml = r#"
type: inject
target:
  type: file
  file: src/index.ts
content: "import './new-module';"
after: "import React"
"#;
  let step: Step = serde_yaml::from_str(yaml).unwrap();
  if let Step::Inject(s) = step {
    assert!(matches!(s.target, Target::File { .. }));
    assert_eq!(s.after.as_deref(), Some("import React"));
    assert!(s.before.is_none());
  } else {
    panic!("expected Inject step");
  }
}

#[test]
fn replace_step_with_glob_target() {
  let yaml = r#"
type: replace
target:
  type: glob
  glob: "src/**/*.ts"
find: "old-package"
replace: "new-package"
"#;
  let step: Step = serde_yaml::from_str(yaml).unwrap();
  if let Step::Replace(s) = step {
    assert!(matches!(s.target, Target::Glob { .. }));
    assert_eq!(s.find, "old-package");
    assert_eq!(s.replace, "new-package");
  } else {
    panic!("expected Replace step");
  }
}

#[test]
fn append_step_deserializes() {
  let yaml = r#"
type: append
target:
  type: file
  file: .gitignore
content: "dist/\n"
"#;
  let step: Step = serde_yaml::from_str(yaml).unwrap();
  assert!(matches!(step, Step::Append(_)));
}

#[test]
fn delete_step_deserializes() {
  let yaml = r#"
type: delete
target:
  type: file
  file: old-file.txt
"#;
  let step: Step = serde_yaml::from_str(yaml).unwrap();
  assert!(matches!(step, Step::Delete(_)));
}

#[test]
fn rename_step_deserializes() {
  let yaml = r#"
type: rename
from: old-name.ts
to: new-name.ts
"#;
  let step: Step = serde_yaml::from_str(yaml).unwrap();
  if let Step::Rename(s) = step {
    assert_eq!(s.from, "old-name.ts");
    assert_eq!(s.to, "new-name.ts");
  } else {
    panic!("expected Rename step");
  }
}

#[test]
fn move_step_deserializes() {
  let yaml = r#"
type: move
from: src/old-dir
to: src/new-dir
"#;
  let step: Step = serde_yaml::from_str(yaml).unwrap();
  if let Step::Move(s) = step {
    assert_eq!(s.from, "src/old-dir");
    assert_eq!(s.to, "src/new-dir");
  } else {
    panic!("expected Move step");
  }
}

// ── IfNotFound default ────────────────────────────────────────────────────────

#[test]
fn if_not_found_defaults_to_warn_and_ask() {
  let yaml = r##"
type: inject
target:
  type: file
  file: README.md
content: "# Section"
"##;
  let step: Step = serde_yaml::from_str(yaml).unwrap();
  if let Step::Inject(s) = step {
    assert!(matches!(s.if_not_found, IfNotFound::WarnAndAsk));
  }
}

#[test]
fn if_not_found_error_variant() {
  let yaml = r#"
type: inject
target:
  type: file
  file: src/main.ts
content: some-code
if_not_found: error
"#;
  let step: Step = serde_yaml::from_str(yaml).unwrap();
  if let Step::Inject(s) = step {
    assert!(matches!(s.if_not_found, IfNotFound::Error));
  }
}

// ── Variant ───────────────────────────────────────────────────────────────────

#[test]
fn variant_with_condition_and_commands() {
  let yaml = r#"
when: "typescript"
commands:
  - name: install
    description: Install TypeScript
    once: true
    steps:
      - type: create
        path: tsconfig.json
        content: "{}"
"#;
  let variant: Variant = serde_yaml::from_str(yaml).unwrap();
  assert_eq!(variant.when.as_deref(), Some("typescript"));
  assert_eq!(variant.commands.len(), 1);
  assert_eq!(variant.commands[0].name, "install");
  assert!(variant.commands[0].once);
  assert_eq!(variant.commands[0].steps.len(), 1);
}

#[test]
fn variant_without_condition() {
  let yaml = r#"
commands:
  - name: setup
    steps: []
"#;
  let variant: Variant = serde_yaml::from_str(yaml).unwrap();
  assert!(variant.when.is_none());
}

// ── Full manifest round-trip ──────────────────────────────────────────────────

#[test]
fn full_manifest_yaml_round_trip() {
  let yaml = r#"
schema_version: "1.0"
id: tailwind
name: Tailwind CSS
version: "1.2.3"
description: Add Tailwind CSS to your project
author: anesis-dev
requires:
  - node
inputs:
  - name: config-format
    type: select
    options:
      - js
      - ts
detect:
  - id: has-vite
    match: all
    rules:
      - type: file_exists
        file: vite.config.ts
variants:
  - when: ts
    commands:
      - name: install
        steps:
          - type: create
            path: tailwind.config.ts
            content: "export default {}"
"#;
  let manifest: AddonManifest = serde_yaml::from_str(yaml).unwrap();
  assert_eq!(manifest.id, "tailwind");
  assert_eq!(manifest.inputs.len(), 1);
  assert_eq!(manifest.detect.len(), 1);
  assert_eq!(manifest.variants.len(), 1);
  assert_eq!(manifest.variants[0].commands[0].steps.len(), 1);
}
