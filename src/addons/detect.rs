use std::path::Path;

use crate::addons::manifest::{DetectBlock, DetectRule, MatchMode};

pub fn detect_variant(detect: &[DetectBlock], project_root: &Path) -> Option<String> {
  for block in detect {
    let matches = match block.match_mode {
      MatchMode::All => block.rules.iter().all(|r| eval_rule(r, project_root)),
      MatchMode::Any => block.rules.iter().any(|r| eval_rule(r, project_root)),
    };
    if matches {
      return Some(block.id.clone());
    }
  }
  None
}

fn eval_rule(rule: &DetectRule, project_root: &Path) -> bool {
  match rule {
    DetectRule::FileExists { file, negate } => {
      let result = project_root.join(file).exists();
      if *negate { !result } else { result }
    }

    DetectRule::FileContains {
      file,
      contains,
      negate,
    } => {
      let result = std::fs::read_to_string(project_root.join(file))
        .map(|s| s.contains(contains.as_str()))
        .unwrap_or(false);
      if *negate { !result } else { result }
    }

    DetectRule::JsonContains {
      file,
      key_path,
      value,
      negate,
    } => {
      let result = std::fs::read_to_string(project_root.join(file))
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .map(|v| traverse_json(&v, key_path, value.as_deref()))
        .unwrap_or(false);
      if *negate { !result } else { result }
    }

    DetectRule::TomlContains {
      file,
      key_path,
      value,
      negate,
    } => {
      let result = std::fs::read_to_string(project_root.join(file))
        .ok()
        .and_then(|s| toml::from_str::<toml::Value>(&s).ok())
        .map(|v| traverse_toml(&v, key_path, value.as_deref()))
        .unwrap_or(false);
      if *negate { !result } else { result }
    }

    DetectRule::YamlContains {
      file,
      key_path,
      value,
      negate,
    } => {
      let result = std::fs::read_to_string(project_root.join(file))
        .ok()
        .and_then(|s| serde_yaml::from_str::<serde_yaml::Value>(&s).ok())
        .map(|v| traverse_yaml(&v, key_path, value.as_deref()))
        .unwrap_or(false);
      if *negate { !result } else { result }
    }
  }
}

fn traverse_json(mut v: &serde_json::Value, key_path: &str, expected: Option<&str>) -> bool {
  for key in key_path.split('.') {
    match v.get(key) {
      Some(next) => v = next,
      None => return false,
    }
  }
  match expected {
    None => true,
    Some(expected) => match v {
      serde_json::Value::String(s) => s == expected,
      #[allow(clippy::cmp_owned)]
      other => other.to_string() == expected,
    },
  }
}

fn traverse_toml(mut v: &toml::Value, key_path: &str, expected: Option<&str>) -> bool {
  for key in key_path.split('.') {
    match v.get(key) {
      Some(next) => v = next,
      None => return false,
    }
  }
  match expected {
    None => true,
    Some(expected) => match v {
      toml::Value::String(s) => s == expected,
      #[allow(clippy::cmp_owned)]
      other => other.to_string() == expected,
    },
  }
}

fn traverse_yaml(mut v: &serde_yaml::Value, key_path: &str, expected: Option<&str>) -> bool {
  for key in key_path.split('.') {
    match v.get(key) {
      Some(next) => v = next,
      None => return false,
    }
  }
  match expected {
    None => true,
    Some(expected) => match v {
      serde_yaml::Value::String(s) => s == expected,
      other => format!("{other:?}") == expected,
    },
  }
}
