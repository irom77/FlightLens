//! Corpus-level smoke test for real-world CLI backups.
//!
//! This binary deliberately reports metadata and coverage only. It never prints
//! backup contents, values, or source lines, so it is safe to run against a
//! private backup directory.

use flightlens_core::{analysis, analyze, export, Artifact, ConfigDocument, Scope};
use std::{
    env, fs,
    path::{Path, PathBuf},
    process,
};

#[derive(Default)]
struct Summary {
    files: usize,
    configs: usize,
    unsupported: usize,
    failures: usize,
    parse_errors: usize,
    complete_rate_profiles: usize,
    partial_rate_profiles: usize,
    pid_profiles_with_values: usize,
}

fn usage() {
    eprintln!("usage: corpus_check [BACKUP_DIR] [--strict]");
    eprintln!("  BACKUP_DIR defaults to /home/irom/fpv_cli_dumps/backups");
    eprintln!("  --strict fails when a compatible backup has no complete Rates or PID data");
}

fn collect_files(root: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = fs::read_dir(root).map_err(|e| format!("{}: {e}", root.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("{}: {e}", root.display()))?;
        let path = entry.path();
        let metadata = entry
            .metadata()
            .map_err(|e| format!("{}: {e}", path.display()))?;
        if metadata.is_dir() {
            collect_files(&path, out)?;
        } else if metadata.is_file()
            && path
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("txt"))
        {
            out.push(path);
        }
    }
    Ok(())
}

fn basename(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("<non-utf8 filename>")
        .to_owned()
}

fn known_pid_values(document: &ConfigDocument, profile: u8) -> usize {
    document
        .parameters
        .values()
        .filter(|parameter| {
            parameter.scope == Scope::Pid(profile) && parameter.valid && parameter.supported
        })
        .count()
}

fn known_filter_values(document: &ConfigDocument) -> usize {
    document
        .parameters
        .values()
        .filter(|parameter| {
            parameter.valid
                && parameter.supported
                && (parameter.key.starts_with("gyro_")
                    || parameter.key.starts_with("dterm_")
                    || parameter.key.starts_with("dyn_notch_")
                    || parameter.key.starts_with("rpm_filter_"))
        })
        .count()
}

fn check_config(document: &ConfigDocument, strict: bool, summary: &mut Summary) -> bool {
    let errors = document
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == "error")
        .count();
    summary.parse_errors += errors;

    // Diagnostics are intentionally non-fatal here. Real backups can contain
    // values introduced by a newer target build; those values remain visible
    // as invalid/unknown while the rest of the snapshot is still testable.
    // Structural failures below are fatal because they would make a tab empty
    // without an explanation.
    let mut invariant_failure = false;
    let mut complete_rates = 0;
    let mut partial_rates = 0;
    let mut complete_pids = 0;
    let mut pid_values = 0;

    for profile in &document.rate_profiles {
        let curves = analysis::rates(document, *profile);
        let mut profile_complete = true;
        for curve in &curves {
            if curve.points.is_empty() {
                profile_complete = false;
                if curve.reason.is_none() {
                    eprintln!(
                        "  FAIL rates profile {profile}: {} has no points and no reason",
                        curve.name
                    );
                    invariant_failure = true;
                }
            } else if curve.points.len() != 201 || curve.reason.is_some() {
                eprintln!(
                    "  FAIL rates profile {profile}: {} has invalid curve shape",
                    curve.name
                );
                invariant_failure = true;
            }
        }
        if profile_complete {
            complete_rates += 1;
        } else {
            partial_rates += 1;
        }
    }

    for profile in &document.pid_profiles {
        let count = known_pid_values(document, *profile);
        pid_values += count;
        if count > 0 {
            complete_pids += 1;
        }
    }

    let inspection = analysis::inspect(document, document.selected_rate.unwrap_or(0));
    if inspection.rates.len() != 3 {
        eprintln!(
            "  FAIL inspection: expected 3 rate axes, got {}",
            inspection.rates.len()
        );
        invariant_failure = true;
    }
    if inspection.audits.is_empty() {
        eprintln!("  FAIL inspection: audit coverage is empty");
        invariant_failure = true;
    }

    // Exercise the same export validation boundary used by the Rates and
    // PID/Filters tabs. Unknown dependencies are reported as partial coverage,
    // not as parser failures.
    let export_rates = document.selected_rate.and_then(|rate_profile| {
        document
            .firmware
            .header
            .as_ref()
            .map(|header| export::ExportRequest {
                groups: vec!["rates".into()],
                pid_profile: document.selected_pid.unwrap_or(0),
                rate_profile,
                destination_header: header.clone(),
                include_save: false,
            })
    });
    let rates_exportable = export_rates
        .as_ref()
        .is_some_and(|request| export::export(document, request).is_ok());

    let export_pids = document.selected_pid.and_then(|pid_profile| {
        document
            .firmware
            .header
            .as_ref()
            .map(|header| export::ExportRequest {
                groups: vec!["pids_filters".into()],
                pid_profile,
                rate_profile: document.selected_rate.unwrap_or(0),
                destination_header: header.clone(),
                include_save: false,
            })
    });
    let pids_exportable = export_pids
        .as_ref()
        .is_some_and(|request| export::export(document, request).is_ok());

    if strict && document.firmware.pack_id.is_some() && complete_rates == 0 {
        eprintln!("  FAIL strict Rates: no profile has three complete curves");
        invariant_failure = true;
    }
    if strict && document.firmware.pack_id.is_some() && complete_pids == 0 {
        eprintln!("  FAIL strict PID: no profile has known PID values");
        invariant_failure = true;
    }

    summary.complete_rate_profiles += complete_rates;
    summary.partial_rate_profiles += partial_rates;
    summary.pid_profiles_with_values += complete_pids;

    println!(
        "{} {:<62} fw={} pack={} errors={} rates={}/{} pid={}/{} filters={} ports={} modes={} osd={} audit={} export(r/p)={}/{}{}{}",
        if document.firmware.pack_id.is_some() { "PASS" } else { "INFO" },
        basename(Path::new(&document.title)),
        document.firmware.version.as_deref().unwrap_or("unknown"),
        if document.firmware.pack_id.is_some() { "yes" } else { "no" },
        errors,
        complete_rates,
        document.rate_profiles.len(),
        complete_pids,
        document.pid_profiles.len(),
        known_filter_values(document),
        document.ports.len(),
        document.modes.len(),
        inspection.osd.len(),
        inspection.audits.len(),
        if rates_exportable { "yes" } else { "no" },
        if pids_exportable { "yes" } else { "no" },
        if partial_rates > 0 { " [partial rates]" } else { "" },
        if errors > 0 { " [diagnostics]" } else { "" },
    );
    if pid_values == 0 && document.firmware.pack_id.is_some() {
        eprintln!("  WARN PID coverage: no known PID values in any discovered profile");
    }
    invariant_failure
}

fn main() {
    let mut root = PathBuf::from("/home/irom/fpv_cli_dumps/backups");
    let mut strict = false;
    for argument in env::args().skip(1) {
        if argument == "--strict" {
            strict = true;
        } else if argument == "--help" || argument == "-h" {
            usage();
            return;
        } else {
            root = PathBuf::from(argument);
        }
    }
    if !root.is_dir() {
        eprintln!("error: backup directory does not exist: {}", root.display());
        process::exit(2);
    }

    let mut files = Vec::new();
    if let Err(error) = collect_files(&root, &mut files) {
        eprintln!("error: {error}");
        process::exit(2);
    }
    files.sort();
    if files.is_empty() {
        eprintln!("error: no .txt CLI backups found in {}", root.display());
        process::exit(2);
    }

    println!(
        "FlightLens CLI corpus: {} ({} files)",
        root.display(),
        files.len()
    );
    println!("Legend: rates=complete profiles/seen profiles, pid=profiles with known values/seen profiles");
    println!();
    let mut summary = Summary {
        files: files.len(),
        ..Summary::default()
    };
    for path in files {
        let label = basename(&path);
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) => {
                eprintln!("FAIL {label}: could not read backup ({error})");
                summary.failures += 1;
                continue;
            }
        };
        match analyze(&text, &label, &label) {
            Ok(Artifact::Config(document)) => {
                summary.configs += 1;
                if check_config(&document, strict, &mut summary) {
                    summary.failures += 1;
                }
            }
            Ok(Artifact::Recognized(artifact)) => {
                summary.unsupported += 1;
                println!(
                    "SKIP {:<62} family={} ({})",
                    label, artifact.family, artifact.message
                );
            }
            Err(error) => {
                summary.failures += 1;
                println!("FAIL {:<62} import failed: {}", label, error);
            }
        }
    }
    println!();
    println!(
        "Summary: files={} configs={} skipped={} failures={} parse_errors={} complete_rate_profiles={} partial_rate_profiles={} pid_profiles_with_values={}",
        summary.files,
        summary.configs,
        summary.unsupported,
        summary.failures,
        summary.parse_errors,
        summary.complete_rate_profiles,
        summary.partial_rate_profiles,
        summary.pid_profiles_with_values,
    );
    if summary.failures > 0 {
        process::exit(1);
    }
}
