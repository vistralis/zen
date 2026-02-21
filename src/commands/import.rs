// SPDX-License-Identifier: Apache-2.0

use crate::db::Database;
use std::error::Error;
use std::path::PathBuf;

pub fn run(db: &Database, file: PathBuf) -> Result<(), Box<dyn Error>> {
    #[derive(serde::Deserialize)]
    struct FullRegistry {
        environments: Vec<(String, String, String, String, bool)>,
        templates: Vec<TemplateExport>,
    }
    #[derive(serde::Deserialize)]
    struct TemplateExport {
        name: String,
        version: String,
        python_version: String,
        packages: Vec<(String, String, bool, String, Option<String>, i64)>,
    }

    let content = std::fs::read_to_string(file)?;
    let registry: FullRegistry = serde_json::from_str(&content)?;

    for (name, path, python, ..) in registry.environments {
        db.register_env(&name, &path, &python)?;
    }

    for t in registry.templates {
        let (t_id, _) = db.create_template(&t.name, &t.version, &t.python_version)?;
        for (p_name, p_ver, is_pinned, install_type, install_args, step) in t.packages {
            db.add_template_package(
                t_id,
                &p_name,
                &p_ver,
                is_pinned,
                &install_type,
                install_args.as_deref(),
                step,
            )?;
        }
    }
    println!("Full registry (environments + templates) imported.");
    Ok(())
}
