use std::collections::HashSet;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use vizu_notion_core::{App, template};

use crate::args::TemplateCommand;
use crate::commands::confirm;
use crate::output::Out;

pub fn run(cmd: TemplateCommand, app: &App, out: &Out) -> Result<()> {
    match cmd {
        TemplateCommand::New {
            kind,
            sources,
            title,
            links,
            group,
            by_source,
            sum,
            color,
            color_by_source,
            date,
        } => {
            let mut parts: Vec<template::SourcePart> =
                sources.iter().map(template::SourcePart::new).collect();
            for link in &links {
                // `Quelle.Rolle=Ziel`, `Quelle.Rolle` — bei einer Quelle auch nur `Rolle`.
                let (from, target) = match link.split_once('=') {
                    Some((from, target)) => (from, Some(target.to_string())),
                    None => (link.as_str(), None),
                };
                let (source, role) = match from.split_once('.') {
                    Some((source, role)) => (source.to_string(), role.to_string()),
                    None if sources.len() == 1 => (sources[0].clone(), from.to_string()),
                    None => bail!(
                        "--link {link}: bei mehreren Quellen als QUELLE.ROLLE angeben, etwa Aufgaben.projekt"
                    ),
                };
                let Some(part) = parts.iter_mut().find(|p| p.name == source) else {
                    bail!("--link {link}: „{source}\u{201c} ist keine der genannten Quellen");
                };
                part.link = Some(role);
                part.link_to = target;
            }
            let spec = template::Spec {
                title: title.unwrap_or_else(|| format!("{kind} aus {}", sources.join(" und "))),
                kind,
                sources: parts,
                group,
                by_source,
                color,
                color_by_source,
                date,
                sum,
            };
            let body = template::compose(&spec)?;
            if out.json {
                println!("{}", serde_json::json!({ "body": body }));
            } else {
                // Auf stdout, damit man sie umleiten und dann einlesen kann.
                print!("{body}");
            }
            Ok(())
        }

        TemplateCommand::List => {
            out.templates(&template::list(app.conn())?);
            Ok(())
        }

        TemplateCommand::Show { template: needle } => {
            let id = template::resolve(app.conn(), &needle)?;
            out.template(&template::get(app.conn(), id)?);
            Ok(())
        }

        TemplateCommand::Import { files } => {
            let mut imported = Vec::new();
            for path in collect(&files)? {
                let text = std::fs::read_to_string(&path)
                    .with_context(|| format!("{} lesen", path.display()))?;
                let slug = path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or_default()
                    .to_string();
                imported.push(template::import_mmd(app.conn(), &slug, &text)?);
            }
            let slugs: Vec<&str> = imported.iter().map(|t| t.slug.as_str()).collect();
            out.done(
                format!(
                    "{} Vorlagen eingelesen: {}",
                    imported.len(),
                    slugs.join(", ")
                ),
                &imported,
            );
            Ok(())
        }

        TemplateCommand::Rm {
            template: needle,
            yes,
        } => {
            let id = template::resolve(app.conn(), &needle)?;
            let existing = template::get(app.conn(), id)?;
            if !yes && !confirm(&format!("Vorlage „{}\u{201c} löschen?", existing.slug))? {
                if !out.json {
                    println!("Abgebrochen.");
                }
                return Ok(());
            }
            template::delete(app.conn(), id)?;
            out.done(format!("Gelöscht: {}", existing.slug), &existing);
            Ok(())
        }
    }
}

/// Zeichnet eine Vorlage aus dem Zwischenspeicher — ohne Netz.
pub fn render(needle: &str, hidden: &[String], app: &App, out: &Out) -> Result<()> {
    let id = template::resolve(app.conn(), needle)?;
    let found = template::get(app.conn(), id)?;
    let hidden: HashSet<String> = hidden.iter().map(|h| h.trim().to_string()).collect();
    let diagram = template::render(app.conn(), &found, &hidden)?;
    out.diagram(&diagram);
    Ok(())
}

/// Dateien einsammeln: einzelne `.mmd`-Dateien oder alle in einem Verzeichnis.
fn collect(paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for path in paths {
        if path.is_dir() {
            let mut found: Vec<PathBuf> = std::fs::read_dir(path)
                .with_context(|| format!("{} lesen", path.display()))?
                .filter_map(|entry| entry.ok().map(|e| e.path()))
                .filter(|p| p.extension().is_some_and(|e| e == "mmd"))
                .collect();
            found.sort();
            if found.is_empty() {
                bail!("{}: keine .mmd-Datei gefunden", path.display());
            }
            out.extend(found);
        } else {
            out.push(path.clone());
        }
    }
    check_names(&out)?;
    Ok(out)
}

/// Zwei Dateien mit demselben Namen in verschiedenen Verzeichnissen würden
/// einander überschreiben — das ist selten Absicht.
fn check_names(paths: &[PathBuf]) -> Result<()> {
    let mut seen: Vec<&str> = Vec::new();
    for path in paths {
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or_default();
        if seen.contains(&stem) {
            bail!("{stem}: kommt zweimal vor — Kurznamen müssen eindeutig sein");
        }
        seen.push(stem);
    }
    Ok(())
}
