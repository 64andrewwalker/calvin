use crate::domain::entities::Lockfile;

pub fn to_json(lockfile: &Lockfile, filter: Option<&str>) -> anyhow::Result<String> {
    #[derive(serde::Serialize)]
    struct JsonEntry {
        key: String,
        scope: String,
        path: String,
        hash: String,
        source_layer: Option<String>,
        source_layer_path: Option<String>,
        source_asset: Option<String>,
        source_file: Option<String>,
        overrides: Option<String>,
    }

    let entries: Vec<JsonEntry> = lockfile
        .entries()
        .filter(|(k, _)| filter.is_none_or(|f| k.contains(f)))
        .filter_map(|(key, entry)| {
            let (scope, path) = crate::domain::entities::Lockfile::parse_key(key)?;
            Some(JsonEntry {
                key: key.to_string(),
                scope: format!("{:?}", scope).to_lowercase(),
                path: path.to_string(),
                hash: entry.hash().to_string(),
                source_layer: entry.source_layer().map(|s| s.to_string()),
                source_layer_path: entry.source_layer_path().map(|p| p.display().to_string()),
                source_asset: entry.source_asset().map(|s| s.to_string()),
                source_file: entry.source_file().map(|p| p.display().to_string()),
                overrides: entry.overrides().map(|s| s.to_string()),
            })
        })
        .collect();

    let out = serde_json::json!({
        "type": "provenance",
        "count": entries.len(),
        "entries": entries,
    });
    Ok(out.to_string())
}
