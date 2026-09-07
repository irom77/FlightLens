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
    complete_pid_profiles: usize,
    /// Files whose *selected* rate profile renders all three curves, which is
    /// what a user actually opens the Rates tab to see.
    selected_rate_complete: usize,
    files_with_derived: usize,
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
    ["roll", "pitch", "yaw"]
        .into_iter()
        .flat_map(|axis| ["p", "i", "d", "f"].map(move |gain| format!("{gain}_{axis}")))
        .filter(|key| document.number(&Scope::Pid(profile), key).is_some())
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
    let mut pids_with_values = 0;

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
            pids_with_values += 1;
        }
        if count == 12 {
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
        eprintln!("  FAIL strict PID: no profile has all 12 known P/I/D/F gains");
        invariant_failure = true;
    }

    if inspection.rates.iter().all(|c| !c.points.is_empty()) {
        summary.selected_rate_complete += 1;
    }
    if !document.derived.is_empty() {
        summary.files_with_derived += 1;
    }
    summary.complete_rate_profiles += complete_rates;
    summary.partial_rate_profiles += partial_rates;
    summary.pid_profiles_with_values += pids_with_values;
    summary.complete_pid_profiles += complete_pids;

    println!(
        "{} {:<62} fw={} pack={} errors={} rates={}/{} derived={} pid={}/{} filters={} ports={} modes={} osd={} audit={} export(r/p)={}/{}{}{}",
        if invariant_failure { "FAIL" } else if document.firmware.pack_id.is_none() { "INFO" } else if complete_rates == 0 || complete_pids == 0 { "PARTIAL" } else { "PASS" },
        basename(Path::new(&document.title)),
        document.firmware.version.as_deref().unwrap_or("unknown"),
        if document.firmware.pack_id.is_some() { "yes" } else { "no" },
        errors,
        complete_rates,
        document.rate_profiles.len(),
        document.derived.len(),
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
    println!("Legend: rates=complete profiles/seen profiles, pid=complete P/I/D/F profiles/seen profiles");
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
        "Summary: files={} configs={} skipped={} failures={} parse_errors={} selected_rate_complete={} files_with_derived={} complete_rate_profiles={} partial_rate_profiles={} pid_profiles_with_values={} complete_pid_profiles={}",
        summary.files,
        summary.configs,
        summary.unsupported,
        summary.failures,
        summary.parse_errors,
        summary.selected_rate_complete,
        summary.files_with_derived,
        summary.complete_rate_profiles,
        summary.partial_rate_profiles,
        summary.pid_profiles_with_values,
        summary.complete_pid_profiles,
    );
    if summary.failures > 0 {
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document(pid_lines: &str) -> ConfigDocument {
        // Keep complete rates so a strict failure isolates PID coverage.
        let fixture = include_str!("../../../../fixtures/configs/betaflight-4.5.0.dump");
        let rates = fixture.split("rateprofile 0").nth(1).unwrap();
        let text =
            format!("# Betaflight / STM32F405 4.5.0\nprofile 0\n{pid_lines}\nrateprofile 0{rates}");
        let Artifact::Config(document) = analyze(&text, "synthetic", "synthetic").unwrap() else {
            panic!("expected configuration");
        };
        *document
    }

    fn gains() -> String {
        ["roll", "pitch", "yaw"]
            .into_iter()
            .flat_map(|axis| {
                ["p", "i", "d", "f"].map(move |gain| format!("set {gain}_{axis} = 0\n"))
            })
            .collect()
    }

    #[test]
    fn filters_are_not_pid_gains() {
        let d = document("set dterm_lpf1_static_hz = 100");
        assert_eq!(known_pid_values(&d, 0), 0);
        assert!(check_config(&d, true, &mut Summary::default()));
    }

    #[test]
    fn one_gain_does_not_make_a_complete_pid_profile() {
        let d = document("set p_roll = 45");
        assert_eq!(known_pid_values(&d, 0), 1);
        assert!(check_config(&d, true, &mut Summary::default()));
    }

    #[test]
    fn complete_zero_gains_are_known() {
        let d = document(&gains());
        assert_eq!(known_pid_values(&d, 0), 12);
        assert!(!check_config(&d, true, &mut Summary::default()));
    }

    #[test]
    fn invalid_gain_does_not_count_as_known() {
        let d = document(&format!("{}set p_roll = invalid\n", gains()));
        assert_eq!(known_pid_values(&d, 0), 11);
        assert!(check_config(&d, true, &mut Summary::default()));
    }

    #[test]
    fn gains_cannot_be_combined_across_profiles() {
        let d = document(
            "set p_roll = 45\nprofile 1\nset i_roll = 80\nset d_roll = 30\nset f_roll = 120",
        );
        assert!(check_config(&d, true, &mut Summary::default()));
    }
}
