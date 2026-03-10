// SPDX-License-Identifier: Apache-2.0

use crate::db::Database;
use std::error::Error;
use std::path::PathBuf;

pub fn run(db: &Database, file: PathBuf) -> Result<(), Box<dyn Error>> {
    #[derive(serde::Serialize)]
    struct TemplateExport {
        name: String,
        version: String,
        python_version: String,
        packages: Vec<(String, String, bool, String, Option<String>, i64)>,
    }

    #[derive(serde::Serialize)]
    struct FullRegistry {
        environments: Vec<(String, String, String, String, bool, bool)>,
        templates: Vec<TemplateExport>,
    }

    let envs = db.list_envs()?;
    let tpls_data = db.get_all_templates_with_packages()?;
    let templates_export = tpls_data
        .into_iter()
        .map(|(name, version, python_version, packages)| TemplateExport {
            name,
            version,
            python_version,
            packages,
        })
        .collect();

    let registry = FullRegistry {
        environments: envs,
        templates: templates_export,
    };

    let json = serde_json::to_string_pretty(&registry)?;
    std::fs::write(file, json)?;
    println!("Full registry (environments + templates) exported.");
    Ok(())
}
