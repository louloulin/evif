use evif_core::{EvifPlugin, WriteFlags};
use evif_plugins::SkillFsPlugin;
use std::collections::HashSet;

#[tokio::test]
async fn skillfs_exposes_builtin_skills_and_trigger_matching() {
    let plugin = SkillFsPlugin::new();

    let root = plugin.readdir("/").await.expect("root listing");
    let names: Vec<String> = root.into_iter().map(|entry| entry.name).collect();
    assert!(names.iter().any(|name| name == "code-review"));
    assert!(names.iter().any(|name| name == "test-gen"));
    assert!(names.iter().any(|name| name == "doc-gen"));
    assert!(names.iter().any(|name| name == "refactor"));
    assert!(names.iter().any(|name| name == "README"));

    let skill = plugin
        .read("/code-review/SKILL.md", 0, 0)
        .await
        .expect("skill doc");
    let skill_str = String::from_utf8(skill).expect("utf8");
    assert!(skill_str.contains("name: code-review"));
    assert!(skill_str.contains("review"));

    let matched = plugin
        .match_skill("please review this patch for bugs")
        .await
        .expect("match");
    assert_eq!(matched.as_deref(), Some("code-review"));
}

#[tokio::test]
async fn skillfs_registers_custom_skills_and_produces_output_from_input() {
    let plugin = SkillFsPlugin::new();

    plugin.mkdir("/demo-skill", 0o755).await.expect("mkdir skill");
    plugin
        .write(
            "/demo-skill/SKILL.md",
            br#"---
name: demo-skill
description: "Demo skill"
triggers:
  - "demo"
  - "custom"
---

# Demo Skill

Run a demo workflow.
"#
            .to_vec(),
            0,
            WriteFlags::TRUNCATE,
        )
        .await
        .expect("write skill doc");

    let matched = plugin
        .match_skill("run custom demo for me")
        .await
        .expect("match custom");
    assert_eq!(matched.as_deref(), Some("demo-skill"));

    plugin
        .write(
            "/demo-skill/input",
            br#"{"topic":"demo"}"#.to_vec(),
            0,
            WriteFlags::TRUNCATE,
        )
        .await
        .expect("write input");

    let output = plugin
        .read("/demo-skill/output", 0, 0)
        .await
        .expect("read output");
    let output_str = String::from_utf8(output).expect("utf8");
    assert!(output_str.contains("demo-skill"));
    assert!(output_str.contains("topic"));
}

#[tokio::test]
async fn skillfs_parses_complex_yaml_with_multiline_and_special_chars() {
    let plugin = SkillFsPlugin::new();

    plugin
        .mkdir("/complex-skill", 0o755)
        .await
        .expect("mkdir complex-skill");

    let complex_doc = br#"---
name: complex-skill
description: >
  A skill that handles complex scenarios including
  special characters: @#$%, quotes "hello", and
  multiline descriptions that span several lines.
triggers:
  - "complex"
  - "special chars: @#$%"
  - "multi
    line"
---

# Complex Skill

This skill demonstrates complex YAML parsing with:
- Multiline descriptions
- Special characters in triggers
- Multiple trigger patterns

## Usage

Run `complex-skill --verbose` for detailed output.
"#
    .to_vec();

    plugin
        .write(
            "/complex-skill/SKILL.md",
            complex_doc,
            0,
            WriteFlags::TRUNCATE,
        )
        .await
        .expect("write complex skill doc");

    // Verify the skill was parsed and registered by matching its trigger
    let matched = plugin
        .match_skill("run complex analysis")
        .await
        .expect("match complex");
    assert_eq!(matched.as_deref(), Some("complex-skill"));

    // Verify the SKILL.md content is readable and intact
    let skill_content = plugin
        .read("/complex-skill/SKILL.md", 0, 0)
        .await
        .expect("read complex skill");
    let skill_str = String::from_utf8(skill_content).expect("utf8");
    assert!(skill_str.contains("name: complex-skill"));
    assert!(skill_str.contains("special characters"));
    assert!(skill_str.contains("Complex Skill"));
    assert!(skill_str.contains("Usage"));

    // Verify input/output flow works for the complex skill
    plugin
        .write(
            "/complex-skill/input",
            br#"{"data":"test"}"#.to_vec(),
            0,
            WriteFlags::TRUNCATE,
        )
        .await
        .expect("write input");

    let output = plugin
        .read("/complex-skill/output", 0, 0)
        .await
        .expect("read output");
    let output_str = String::from_utf8(output).expect("utf8");
    assert!(output_str.contains("complex-skill"));
    assert!(output_str.contains("test"));
}

#[tokio::test]
async fn skillfs_rejects_invalid_skill_without_frontmatter() {
    let plugin = SkillFsPlugin::new();

    plugin
        .mkdir("/bad-skill", 0o755)
        .await
        .expect("mkdir bad-skill");

    // A SKILL.md with no frontmatter at all
    let no_frontmatter = br#"# Bad Skill

This skill has no YAML frontmatter at all.
"#
    .to_vec();

    let result = plugin
        .write(
            "/bad-skill/SKILL.md",
            no_frontmatter,
            0,
            WriteFlags::TRUNCATE,
        )
        .await;

    assert!(
        result.is_err(),
        "Expected error when writing SKILL.md without frontmatter"
    );

    // A SKILL.md with empty content (no frontmatter, no body)
    let empty_doc = b"".to_vec();
    let result2 = plugin
        .write(
            "/bad-skill/SKILL.md",
            empty_doc,
            0,
            WriteFlags::TRUNCATE,
        )
        .await;

    assert!(
        result2.is_err(),
        "Expected error when writing empty SKILL.md"
    );
}

#[tokio::test]
async fn skillfs_exports_claude_compatible_skills_dir() {
    let plugin = SkillFsPlugin::new();
    let tmp = tempfile::TempDir::new().expect("create temp dir");
    let output_dir = tmp.path().join("skills");
    let output_dir_str = output_dir.to_str().expect("valid utf8 path");

    let exported = plugin
        .export_claude_skills_dir(output_dir_str)
        .await
        .expect("export claude skills");

    assert_eq!(exported.len(), 4, "should export 4 builtin skills");

    let exported_set: HashSet<&str> = exported.iter().map(|s| s.as_str()).collect();
    assert!(exported_set.contains("code-review"));
    assert!(exported_set.contains("test-gen"));
    assert!(exported_set.contains("doc-gen"));
    assert!(exported_set.contains("refactor"));

    // Verify each SKILL.md file exists and contains valid frontmatter + body
    for skill_name in &exported {
        let skill_md_path = output_dir.join(skill_name).join("SKILL.md");
        assert!(skill_md_path.exists(), "SKILL.md should exist for {}", skill_name);

        let content = tokio::fs::read_to_string(&skill_md_path)
            .await
            .expect("read SKILL.md");

        // Must contain YAML frontmatter delimiters
        assert!(content.starts_with("---"), "should start with frontmatter");
        assert!(
            content.contains("name:"),
            "should contain name field in frontmatter"
        );
        assert!(
            content.contains("description:"),
            "should contain description field"
        );
        assert!(
            content.contains("triggers:"),
            "should contain triggers field"
        );

        // Must have a Markdown body after the closing ---
        let parts: Vec<&str> = content.splitn(3, "---").collect();
        assert!(parts.len() >= 3, "should have frontmatter and body");
        let body = parts[2].trim();
        assert!(!body.is_empty(), "body should not be empty");
    }
}

#[tokio::test]
async fn skillfs_exports_codex_agents_yaml() {
    let plugin = SkillFsPlugin::new();
    let tmp = tempfile::TempDir::new().expect("create temp dir");
    let yaml_path = tmp.path().join("agents").join("openai.yaml");
    let yaml_path_str = yaml_path.to_str().expect("valid utf8 path");

    plugin
        .export_codex_agents_yaml(yaml_path_str)
        .await
        .expect("export codex yaml");

    assert!(yaml_path.exists(), "openai.yaml should be created");

    let content = tokio::fs::read_to_string(&yaml_path)
        .await
        .expect("read yaml");

    // Verify top-level structure
    assert!(content.contains("version:"), "should contain version");
    assert!(content.contains("generated_by: evif-skillfs"), "should contain generated_by");
    assert!(content.contains("skills:"), "should contain skills list");

    // Verify each builtin skill is represented
    for name in &["code-review", "test-gen", "doc-gen", "refactor"] {
        assert!(
            content.contains(&format!("name: {}", name)),
            "should contain skill: {}",
            name
        );
    }

    // Verify triggers are present
    assert!(content.contains("triggers:"), "should contain triggers");

    // Verify descriptions are present
    assert!(content.contains("description:"), "should contain descriptions");

    // Parse the YAML to validate structure
    let parsed: serde_yaml::Value = serde_yaml::from_str(&content).expect("valid yaml");
    let skills = parsed.get("skills").expect("skills key").as_sequence().expect("skills is a list");
    assert_eq!(skills.len(), 4, "should have 4 skills in the yaml");
}

#[tokio::test]
async fn skillfs_imports_claude_skills_dir() {
    let plugin = SkillFsPlugin::new();
    let tmp = tempfile::TempDir::new().expect("create temp dir");

    // Create a temp skills directory with a custom skill
    let custom_skill_dir = tmp.path().join("custom-review");
    tokio::fs::create_dir_all(&custom_skill_dir)
        .await
        .expect("create skill dir");

    let skill_md = r#"---
name: custom-review
description: "A custom review skill imported from Claude Code"
triggers:
  - "inspect"
  - "custom-review-trigger"
---

# Custom Review

This is a custom skill imported from a Claude Code skills directory.
"#
    .to_string();

    tokio::fs::write(custom_skill_dir.join("SKILL.md"), &skill_md)
        .await
        .expect("write SKILL.md");

    // Also create a directory without SKILL.md to verify it is skipped
    let no_skill_dir = tmp.path().join("no-skill-here");
    tokio::fs::create_dir_all(&no_skill_dir)
        .await
        .expect("create no-skill dir");

    let input_dir_str = tmp.path().to_str().expect("valid utf8 path");
    let imported = plugin
        .import_claude_skills_dir(input_dir_str)
        .await
        .expect("import skills");

    assert_eq!(imported.len(), 1, "should import exactly 1 skill");
    assert!(
        imported.contains(&"custom-review".to_string()),
        "should have imported custom-review"
    );

    // Verify the skill is registered and triggerable
    let matched = plugin
        .match_skill("please inspect this code for issues")
        .await
        .expect("match custom-review");
    assert_eq!(matched.as_deref(), Some("custom-review"));

    // Verify the skill is readable via the filesystem
    let skill_content = plugin
        .read("/custom-review/SKILL.md", 0, 0)
        .await
        .expect("read imported skill");
    let content_str = String::from_utf8(skill_content).expect("utf8");
    assert!(content_str.contains("custom-review"));
    assert!(content_str.contains("Custom Review"));
}

#[tokio::test]
async fn skillfs_list_skill_definitions_returns_all_skills() {
    let plugin = SkillFsPlugin::new();

    let definitions = plugin.list_skill_definitions().await;

    assert_eq!(definitions.len(), 4, "should have 4 builtin skills");

    let names: HashSet<String> = definitions.iter().map(|(name, _, _)| name.clone()).collect();
    assert!(names.contains("code-review"));
    assert!(names.contains("test-gen"));
    assert!(names.contains("doc-gen"));
    assert!(names.contains("refactor"));

    // Verify each definition has non-empty description and triggers
    for (name, description, triggers) in &definitions {
        assert!(
            !description.is_empty(),
            "skill {} should have a description",
            name
        );
        assert!(
            !triggers.is_empty(),
            "skill {} should have at least one trigger",
            name
        );
    }

    // Spot-check specific skill details
    let code_review = definitions
        .iter()
        .find(|(name, _, _)| name == "code-review")
        .expect("code-review should exist");
    assert!(
        code_review.1.contains("Review code"),
        "code-review description should mention 'Review code'"
    );
    assert!(
        code_review.2.contains(&"review".to_string()),
        "code-review triggers should contain 'review'"
    );
}

#[tokio::test]
async fn skillfs_generate_openai_yaml_produces_valid_codex_format() {
    let plugin = SkillFsPlugin::new();

    let yaml_str = plugin
        .generate_openai_yaml()
        .await
        .expect("generate_openai_yaml should succeed");

    // Parse the YAML to validate structure
    let parsed: serde_yaml::Value =
        serde_yaml::from_str(&yaml_str).expect("output should be valid YAML");

    // Top-level must be "agents" list
    let agents = parsed
        .get("agents")
        .expect("should have 'agents' key")
        .as_sequence()
        .expect("'agents' should be a sequence");
    assert_eq!(agents.len(), 4, "should have 4 builtin agents");

    // Verify each agent entry has the required fields
    for agent in agents {
        let agent_map = agent.as_mapping().expect("agent should be a mapping");
        assert!(
            agent_map.contains_key(serde_yaml::Value::String("name".into())),
            "agent must have 'name'"
        );
        assert!(
            agent_map.contains_key(serde_yaml::Value::String("description".into())),
            "agent must have 'description'"
        );
        assert!(
            agent_map.contains_key(serde_yaml::Value::String("triggers".into())),
            "agent must have 'triggers'"
        );
        assert!(
            agent_map.contains_key(serde_yaml::Value::String("instructions".into())),
            "agent must have 'instructions'"
        );
    }

    // Verify all builtin skill names are present
    let names: HashSet<String> = agents
        .iter()
        .map(|a| {
            a.get("name")
                .unwrap()
                .as_str()
                .unwrap()
                .to_string()
        })
        .collect();
    assert!(names.contains("code-review"));
    assert!(names.contains("test-gen"));
    assert!(names.contains("doc-gen"));
    assert!(names.contains("refactor"));

    // Verify instructions contain actual skill body content (not empty)
    for agent in agents {
        let instructions = agent
            .get("instructions")
            .unwrap()
            .as_str()
            .unwrap();
        assert!(
            !instructions.trim().is_empty(),
            "instructions should not be empty"
        );
    }
}

#[tokio::test]
async fn skillfs_generate_openai_yaml_includes_custom_skills() {
    let plugin = SkillFsPlugin::new();

    // Register a custom skill
    plugin.mkdir("/my-custom", 0o755).await.expect("mkdir");
    plugin
        .write(
            "/my-custom/SKILL.md",
            br#"---
name: my-custom
description: "Custom skill for Codex"
triggers:
  - "custom-codex"
  - "codex run"
---

# My Custom Skill

Do something special with Codex.
"#
            .to_vec(),
            0,
            WriteFlags::TRUNCATE,
        )
        .await
        .expect("write custom skill");

    let yaml_str = plugin
        .generate_openai_yaml()
        .await
        .expect("generate yaml");

    let parsed: serde_yaml::Value = serde_yaml::from_str(&yaml_str).expect("valid yaml");
    let agents = parsed.get("agents").unwrap().as_sequence().unwrap();

    // Should have 4 builtin + 1 custom = 5
    assert_eq!(agents.len(), 5, "should have 5 agents total");

    // Find and verify the custom skill
    let custom = agents
        .iter()
        .find(|a| a.get("name").unwrap().as_str().unwrap() == "my-custom")
        .expect("custom skill should be in output");

    assert_eq!(
        custom.get("description").unwrap().as_str().unwrap(),
        "Custom skill for Codex"
    );

    let triggers = custom
        .get("triggers")
        .unwrap()
        .as_sequence()
        .unwrap();
    let trigger_strs: Vec<&str> = triggers
        .iter()
        .map(|t| t.as_str().unwrap())
        .collect();
    assert!(trigger_strs.contains(&"custom-codex"));
    assert!(trigger_strs.contains(&"codex run"));

    let instructions = custom.get("instructions").unwrap().as_str().unwrap();
    assert!(instructions.contains("My Custom Skill"));
    assert!(instructions.contains("Do something special"));
}

#[tokio::test]
async fn skillfs_sync_to_claude_skills_returns_all_paths_and_content() {
    let plugin = SkillFsPlugin::new();

    let entries = plugin
        .sync_to_claude_skills()
        .await
        .expect("sync_to_claude_skills should succeed");

    assert_eq!(entries.len(), 4, "should have 4 entries for builtin skills");

    // Collect all relative paths
    let paths: HashSet<String> = entries.iter().map(|(path, _)| path.clone()).collect();
    assert!(paths.contains("code-review/SKILL.md"));
    assert!(paths.contains("test-gen/SKILL.md"));
    assert!(paths.contains("doc-gen/SKILL.md"));
    assert!(paths.contains("refactor/SKILL.md"));

    // Each content should be a valid SKILL.md with frontmatter
    for (path, content) in &entries {
        assert!(
            content.starts_with("---"),
            "content for {} should start with frontmatter",
            path
        );
        assert!(
            content.contains("name:"),
            "content for {} should contain name field",
            path
        );
        assert!(
            content.contains("description:"),
            "content for {} should contain description field",
            path
        );
        assert!(
            content.contains("triggers:"),
            "content for {} should contain triggers field",
            path
        );
    }
}

#[tokio::test]
async fn skillfs_sync_to_claude_skills_includes_custom_skills() {
    let plugin = SkillFsPlugin::new();

    // Register a custom skill
    plugin.mkdir("/interop-skill", 0o755).await.expect("mkdir");
    plugin
        .write(
            "/interop-skill/SKILL.md",
            br#"---
name: interop-skill
description: "Interop test skill"
triggers:
  - "interop"
---

# Interop Skill

Test interop with Claude Code.
"#
            .to_vec(),
            0,
            WriteFlags::TRUNCATE,
        )
        .await
        .expect("write skill");

    let entries = plugin
        .sync_to_claude_skills()
        .await
        .expect("sync should succeed");

    assert_eq!(entries.len(), 5, "should have 4 builtin + 1 custom");

    let (path, content) = entries
        .iter()
        .find(|(p, _)| p == "interop-skill/SKILL.md")
        .expect("custom skill should be present");

    assert_eq!(path, "interop-skill/SKILL.md");
    assert!(content.contains("name: interop-skill"));
    assert!(content.contains("Interop Skill"));
    assert!(content.contains("interop"));
}

#[tokio::test]
async fn skillfs_sync_to_claude_skills_can_be_written_to_disk() {
    let plugin = SkillFsPlugin::new();
    let tmp = tempfile::TempDir::new().expect("create temp dir");
    let skills_dir = tmp.path().join(".claude").join("skills");

    let entries = plugin
        .sync_to_claude_skills()
        .await
        .expect("sync should succeed");

    // Simulate what a caller would do: write each entry to disk
    for (relative_path, content) in &entries {
        let full_path = skills_dir.join(relative_path);
        if let Some(parent) = full_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .expect("create parent dir");
        }
        tokio::fs::write(&full_path, content)
            .await
            .expect("write file");
    }

    // Verify the written files match what Claude Code expects
    for skill_name in &["code-review", "test-gen", "doc-gen", "refactor"] {
        let skill_md = skills_dir.join(skill_name).join("SKILL.md");
        assert!(skill_md.exists(), "SKILL.md for {} should exist", skill_name);

        let written = tokio::fs::read_to_string(&skill_md)
            .await
            .expect("read file");
        assert!(written.starts_with("---"), "written file should have frontmatter");
        assert!(written.contains(&format!("name: {}", skill_name)));
    }
}
