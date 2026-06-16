use anesis::templates::{AnesisTemplate, AnesisTemplateMetadata, AnesisTemplateRepository};

// ── JSON serialization / deserialization ──────────────────────────────────────

#[test]
fn anesis_template_deserializes_from_json() {
  let json = r#"{
    "name": "react-vite",
    "version": "1.0.0",
    "anesisVersion": "0.9.0",
    "repository": {"url": "https://github.com/anesis-dev/react-vite"},
    "metadata": {"displayName": "React + Vite", "description": "React with Vite bundler"}
  }"#;

  let t: AnesisTemplate = serde_json::from_str(json).unwrap();
  assert_eq!(t.name, "react-vite");
  assert_eq!(t.version, "1.0.0");
  assert_eq!(t.anesis_version, "0.9.0");
  assert_eq!(t.repository.url, "https://github.com/anesis-dev/react-vite");
  assert_eq!(t.metadata.display_name, "React + Vite");
  assert_eq!(t.metadata.description, "React with Vite bundler");
}

#[test]
fn anesis_template_serializes_with_camel_case_keys() {
  let template = AnesisTemplate {
    name: "next-app".to_string(),
    version: "2.0.0".to_string(),
    anesis_version: "0.8.0".to_string(),
    repository: AnesisTemplateRepository {
      url: "https://github.com/example/next-app".to_string(),
    },
    metadata: AnesisTemplateMetadata {
      display_name: "Next.js App".to_string(),
      description: "Next.js application template".to_string(),
    },
  };

  let json = serde_json::to_string(&template).unwrap();
  // The rename attributes use camelCase for anesis_version and display_name.
  assert!(
    json.contains("\"anesisVersion\""),
    "should use anesisVersion key"
  );
  assert!(
    json.contains("\"displayName\""),
    "should use displayName key"
  );
  assert!(json.contains("\"next-app\""));
  assert!(
    !json.contains("anesis_version"),
    "should not use snake_case key"
  );
  assert!(
    !json.contains("display_name"),
    "should not use snake_case key"
  );
  assert!(
    !json.contains("official"),
    "should not serialize removed official key"
  );
}

#[test]
fn anesis_template_json_round_trip_preserves_all_fields() {
  let original = AnesisTemplate {
    name: "svelte-kit".to_string(),
    version: "3.1.0".to_string(),
    anesis_version: "0.9.0".to_string(),
    repository: AnesisTemplateRepository {
      url: "https://github.com/anesis-dev/svelte-kit".to_string(),
    },
    metadata: AnesisTemplateMetadata {
      display_name: "SvelteKit".to_string(),
      description: "SvelteKit fullstack template".to_string(),
    },
  };

  let json = serde_json::to_string(&original).unwrap();
  let restored: AnesisTemplate = serde_json::from_str(&json).unwrap();

  assert_eq!(restored.name, original.name);
  assert_eq!(restored.version, original.version);
  assert_eq!(restored.anesis_version, original.anesis_version);
  assert_eq!(restored.repository.url, original.repository.url);
  assert_eq!(
    restored.metadata.display_name,
    original.metadata.display_name
  );
  assert_eq!(restored.metadata.description, original.metadata.description);
}

// ── AnesisTemplateRepository ───────────────────────────────────────────────────

#[test]
fn repository_serializes_and_deserializes() {
  let repo = AnesisTemplateRepository {
    url: "https://github.com/owner/repo".to_string(),
  };
  let json = serde_json::to_string(&repo).unwrap();
  let back: AnesisTemplateRepository = serde_json::from_str(&json).unwrap();
  assert_eq!(back.url, repo.url);
}

// ── AnesisTemplateMetadata ─────────────────────────────────────────────────────

#[test]
fn metadata_deserializes_display_name_camel_case() {
  let json = r#"{"displayName":"My Template","description":"desc"}"#;
  let meta: AnesisTemplateMetadata = serde_json::from_str(json).unwrap();
  assert_eq!(meta.display_name, "My Template");
  assert_eq!(meta.description, "desc");
}
