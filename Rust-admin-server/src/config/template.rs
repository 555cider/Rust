use anyhow::{anyhow, Result};
use tera::Tera;
use tracing::error;

/// Initialize and configure Tera template engine with glob pattern support
///
/// # Errors
///
/// Returns an error if template loading or parsing fails
pub fn setup_tera() -> Result<Tera> {
    // Try to find the templates directory
    let template_dir = std::env::current_dir()?.join("static/templates");

    if !template_dir.exists() {
        return Err(anyhow!(
            "Templates directory not found at: {}",
            template_dir.display()
        ));
    }

    // Initialize Tera with an empty template
    let mut tera = Tera::default();

    // Load all HTML files in the templates directory
    let main_glob = format!("{}/**/*.html", template_dir.display());

    // Use parse_glob to load all templates including subdirectories
    if let Err(e) = Tera::new(&main_glob).map(|t| tera = t) {
        error!("Failed to parse templates: {}", e);
        return Err(anyhow!("Failed to parse templates: {}", e));
    }

    // Configure auto-escaping for HTML files
    tera.autoescape_on(vec![".html"]);

    Ok(tera)
}

/// A more detailed version that allows for custom template loading logic with fallbacks
///
/// # Errors
///
/// Returns an error if template loading or parsing fails
pub fn setup_tera_with_fallback() -> Result<Tera> {
    // First try the standard way
    match setup_tera() {
        Ok(tera) => Ok(tera),
        Err(e) => {
            tracing::warn!("Standard template loading failed: {}", e);

            // Try multiple possible template directory locations
            let possible_paths = [
                std::env::current_dir()?.join("static/templates"),
                std::env::current_dir()?.join("../static/templates"),
                std::env::current_dir()?.join("../../static/templates"),
            ];

            // Find the first path that exists and is a directory
            let templates_dir = possible_paths
                .iter()
                .find(|path| path.is_dir())
                .ok_or_else(|| {
                    anyhow!(
                        "Could not find templates directory in fallback mode. Tried: {:?}",
                        possible_paths
                    )
                })?;

            tracing::info!(
                "Attempting fallback template loading from: {}",
                templates_dir.display()
            );

            // Create a new Tera instance with the templates directory as a template root
            let template_glob = format!("{}/**/*.html", templates_dir.display());
            let mut tera = match Tera::new(&template_glob) {
                Ok(t) => t,
                Err(e) => {
                    tracing::error!("Failed to parse templates: {}", e);
                    return Err(anyhow!("Failed to parse templates: {}", e));
                }
            };

            // Enable autoescaping for HTML safety
            tera.autoescape_on(vec![".html"]);

            // Additional debugging for template loading
            tracing::debug!(
                "Successfully loaded templates from: {}",
                templates_dir.display()
            );

            // Verify base template is loaded
            if !tera.get_template_names().any(|name| name == "base.html") {
                tracing::error!("Base template (base.html) not found in the templates directory");
                return Err(anyhow!("Base template (base.html) not found"));
            }

            let loaded_templates: Vec<_> = tera.get_template_names().collect();
            tracing::info!(
                "Successfully loaded {} templates in fallback mode: {:?}",
                loaded_templates.len(),
                loaded_templates
            );

            Ok(tera)
        }
    }
}
