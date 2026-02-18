//! Template expansion for cycle prompts.
//!
//! Supports `{{variable_name}}` syntax. Unknown variables are left as-is.

use std::collections::HashMap;
use std::path::Path;

/// Expand `{{variable_name}}` patterns in a template string.
///
/// Resolution: looks up each `{{name}}` in `vars`. If found, replaces with
/// the value. If not found, leaves the `{{name}}` literal in the output.
/// Partial syntax like `{{incomplete` is also left as-is.
#[must_use]
#[allow(clippy::implicit_hasher)]
pub fn expand_template(template: &str, vars: &HashMap<String, String>) -> String {
    let mut result = String::with_capacity(template.len());
    let bytes = template.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if i + 1 < len && bytes[i] == b'{' && bytes[i + 1] == b'{' {
            // Found opening `{{` — look for closing `}}`
            if let Some(close) = template[i + 2..].find("}}") {
                let var_name = &template[i + 2..i + 2 + close];
                // Only match if the variable name contains no whitespace
                if !var_name.is_empty() && !var_name.contains(char::is_whitespace) {
                    if let Some(value) = vars.get(var_name) {
                        result.push_str(value);
                    } else {
                        // Unknown variable — leave as-is
                        result.push_str(&template[i..i + 2 + close + 2]);
                    }
                    i += 2 + close + 2;
                    continue;
                }
            }
            // No closing `}}` found or whitespace in name — emit literal `{{`
            result.push_str("{{");
            i += 2;
        } else {
            result.push(template[i..].chars().next().unwrap());
            i += template[i..].chars().next().unwrap().len_utf8();
        }
    }

    result
}

/// Build the full template variable map from custom vars + runtime built-ins.
///
/// Built-in variables override custom vars with the same name.
#[must_use]
#[allow(clippy::implicit_hasher)]
pub fn build_template_vars(
    custom_vars: &HashMap<String, String>,
    project_dir: &Path,
    todo_file: &Path,
    cycle_name: &str,
    step_name: &str,
    iteration: u32,
    max_iterations: u32,
) -> HashMap<String, String> {
    // Start with custom vars, then override with built-ins
    let mut vars = custom_vars.clone();
    vars.insert(
        "project_dir".to_string(),
        project_dir.to_string_lossy().to_string(),
    );
    vars.insert(
        "todo_file".to_string(),
        todo_file.to_string_lossy().to_string(),
    );
    vars.insert("cycle_name".to_string(), cycle_name.to_string());
    vars.insert("step_name".to_string(), step_name.to_string());
    vars.insert("iteration".to_string(), iteration.to_string());
    vars.insert("max_iterations".to_string(), max_iterations.to_string());
    vars
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn test_no_variables_returns_unchanged() {
        let result = expand_template("Hello world", &HashMap::new());
        assert_eq!(result, "Hello world");
    }

    #[test]
    fn test_single_variable_expanded() {
        let result = expand_template("Hello {{name}}", &vars(&[("name", "world")]));
        assert_eq!(result, "Hello world");
    }

    #[test]
    fn test_multiple_variables_expanded() {
        let result = expand_template(
            "{{greeting}} {{name}}!",
            &vars(&[("greeting", "Hi"), ("name", "Alice")]),
        );
        assert_eq!(result, "Hi Alice!");
    }

    #[test]
    fn test_same_variable_used_twice() {
        let result = expand_template("{{x}} and {{x}}", &vars(&[("x", "a")]));
        assert_eq!(result, "a and a");
    }

    #[test]
    fn test_unknown_variable_left_as_is() {
        let result = expand_template("{{unknown}}", &HashMap::new());
        assert_eq!(result, "{{unknown}}");
    }

    #[test]
    fn test_empty_template_returns_empty() {
        let result = expand_template("", &vars(&[("x", "y")]));
        assert_eq!(result, "");
    }

    #[test]
    fn test_variable_at_start_and_end() {
        let result = expand_template("{{a}}middle{{b}}", &vars(&[("a", "start-"), ("b", "-end")]));
        assert_eq!(result, "start-middle-end");
    }

    #[test]
    fn test_adjacent_variables() {
        let result = expand_template("{{a}}{{b}}", &vars(&[("a", "x"), ("b", "y")]));
        assert_eq!(result, "xy");
    }

    #[test]
    fn test_partial_syntax_not_expanded() {
        let result = expand_template("{{incomplete", &HashMap::new());
        assert_eq!(result, "{{incomplete");
    }

    #[test]
    fn test_whitespace_in_var_name_not_matched() {
        // Strict matching: spaces inside {{ }} means it's not a valid variable reference
        let result = expand_template("{{ name }}", &vars(&[("name", "world")]));
        assert_eq!(result, "{{ name }}");
    }

    #[test]
    fn test_build_template_vars_includes_builtins() {
        let custom = vars(&[("project_name", "flow")]);
        let result = build_template_vars(
            &custom,
            Path::new("/tmp/project"),
            Path::new("TODO.md"),
            "coding",
            "",
            1,
            20,
        );
        assert_eq!(result.get("project_dir").unwrap(), "/tmp/project");
        assert_eq!(result.get("todo_file").unwrap(), "TODO.md");
        assert_eq!(result.get("cycle_name").unwrap(), "coding");
        assert_eq!(result.get("step_name").unwrap(), "");
        assert_eq!(result.get("iteration").unwrap(), "1");
        assert_eq!(result.get("max_iterations").unwrap(), "20");
        // Custom vars also present
        assert_eq!(result.get("project_name").unwrap(), "flow");
    }

    #[test]
    fn test_build_template_vars_builtins_override_custom() {
        let custom = vars(&[("cycle_name", "user-defined"), ("custom_key", "custom_val")]);
        let result = build_template_vars(
            &custom,
            Path::new("/tmp"),
            Path::new("TODO.md"),
            "coding",
            "plan",
            3,
            10,
        );
        // Built-in wins over custom
        assert_eq!(result.get("cycle_name").unwrap(), "coding");
        // Custom keys still present
        assert_eq!(result.get("custom_key").unwrap(), "custom_val");
    }

    #[test]
    fn test_builtin_vars_override_custom() {
        // Built-in variables should take priority over custom vars
        let mut v = vars(&[("cycle_name", "user-defined")]);
        // Simulate build_template_vars logic: built-ins inserted after custom
        v.insert("cycle_name".to_string(), "coding".to_string());
        let result = expand_template("Running {{cycle_name}}", &v);
        assert_eq!(result, "Running coding");
    }

    #[test]
    fn test_realistic_prompt_template() {
        let v = vars(&[
            ("cycle_name", "coding"),
            ("project_name", "flow"),
            ("iteration", "3"),
            ("max_iterations", "20"),
        ]);
        let template = "You are {{project_name}}'s {{cycle_name}} cycle. \
                         Iteration {{iteration}}/{{max_iterations}}.";
        let result = expand_template(template, &v);
        assert_eq!(result, "You are flow's coding cycle. Iteration 3/20.");
    }
}
