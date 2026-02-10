use crate::cli::detector::ProjectTemplate;
use dialoguer::MultiSelect;
use std::path::Path;

/// Show preview of what will be created
pub fn show_preview(group_name: &str, directory: &Path, projects: &[ProjectTemplate]) {
    println!("\n╔════════════════════════════════════════════════════════╗");
    println!("║           PREVIEW: Group to be created                 ║");
    println!("╚════════════════════════════════════════════════════════╝");
    println!("  Group Name: {}", group_name);
    println!("  Directory:  {}", directory.display());
    println!("  Projects:   {}", projects.len());
    println!("  ───────────────────────────────────────────────────────");

    for (i, project) in projects.iter().enumerate() {
        println!("  {}. {}", i + 1, project.name);
        println!("     └─ {}", project.description);
        println!("     └─ $ {}", project.command);
        if i < projects.len() - 1 {
            println!();
        }
    }
    println!("  ═══════════════════════════════════════════════════════\n");
}

/// Prompt user for project selection using dialoguer
pub fn prompt_project_selection(
    projects: &[ProjectTemplate],
) -> Result<Vec<usize>, Box<dyn std::error::Error>> {
    if projects.is_empty() {
        return Ok(Vec::new());
    }

    let items: Vec<String> = projects
        .iter()
        .map(|p| format!("{} - {}", p.name, p.description))
        .collect();

    let defaults: Vec<bool> = vec![true; projects.len()];

    let selection = MultiSelect::new()
        .with_prompt("Select projects to include (use arrow keys to navigate, space to select/deselect, enter to confirm)")
        .items(&items)
        .defaults(&defaults)
        .interact()?;

    Ok(selection)
}

/// Prompt user for confirmation
pub fn prompt_confirmation(message: &str) -> Result<bool, Box<dyn std::error::Error>> {
    use dialoguer::Confirm;

    let confirmed = Confirm::new()
        .with_prompt(message)
        .default(true)
        .interact()?;

    Ok(confirmed)
}
