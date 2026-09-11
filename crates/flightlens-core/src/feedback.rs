//! Redaction of a backup before a user submits it with a feedback report.
//!
//! FlightLens never transmits a backup itself. A report is filed by the user on
//! GitHub, so anything this module leaves in the text is published by hand, in
//! public, by someone who is trusting the preview to be complete. Redaction is
//! therefore an explicit list of keys rather than a pattern: a heuristic that
//! guesses from a key's spelling both misses secrets a future firmware names
//! differently and strips settings a maintainer needs to reproduce the defect.
//!
//! The listed keys stay listed even where no bundled firmware declares them, so
//! a backup from a build FlightLens has no compatibility data for is redacted on
//! the same terms as one it recognizes.
use crate::model::*;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Settings whose value names a person or a craft.
///
/// Every one of these is free text the pilot typed, so it carries whatever they
/// chose to put there and none of it helps reproduce a defect.
///
/// The list spans every bundled firmware line rather than the newest, because
/// Betaflight renamed these settings: 4.2 and 4.3 write `name` and
/// `display_name` where 4.4 and later write `craft_name` and `pilot_name`, and
/// only 4.2 has the `box_user_*_name` labels. Redacting just the current
/// spelling leaves the craft name in plain sight in an older backup.
const IDENTITY_KEYS: [&str; 13] = [
    "box_user_1_name",
    "box_user_2_name",
    "box_user_3_name",
    "box_user_4_name",
    "craft_name",
    "display_name",
    "name",
    "osd_profile_1_name",
    "osd_profile_2_name",
    "osd_profile_3_name",
    "pilot_name",
    "profile_name",
    "rateprofile_name",
];

/// Settings whose value is a radio link identity.
///
/// `expresslrs_uid` is derived from the binding phrase, and the SPI receiver
/// identities serve the same purpose for their protocols: publishing one lets a
/// stranger bind to or spoof the link of the aircraft it came from. These are
/// credentials in everything but name.
const LINK_SECRET_KEYS: [&str; 6] = [
    "expresslrs_uid",
    "flysky_spi_tx_id",
    "frsky_spi_bind_hop_data",
    "frsky_spi_tx_id",
    "spektrum_spi_mfg_id",
    "srxl2_unit_id",
];

/// What replaces a redacted value. Chosen to be obvious in a rendered issue and
/// to survive a round trip through the parser as a value that is plainly not a
/// real one.
const PLACEHOLDER: &str = "<redacted>";

/// The `# name:` header a dump prints, which carries the craft name on firmware
/// older than the `craft_name` setting.
const NAME_HEADER: &str = "# name:";

/// GitHub's maximum issue body length, in characters. A backup above this
/// cannot be pasted into an issue at all and has to be attached as a file.
pub const ISSUE_BODY_LIMIT: usize = 65_536;

/// Why a line was redacted, which the preview shows so the user can see that
/// the removal was deliberate rather than a parsing failure.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    /// Names a person or craft.
    Identity,
    /// A radio link identity that lets someone else bind to or spoof the link.
    LinkSecret,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Redaction {
    /// The setting whose value was removed, or `name` for the dump header.
    pub key: String,
    pub line: u32,
    pub category: Category,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct RedactedConfig {
    /// The backup with every listed value replaced, line for line. Lines the
    /// parser could not read are carried through untouched rather than dropped,
    /// because a line nobody can read is often the defect being reported.
    pub text: String,
    /// Every removal, in source order. Shown to the user rather than merely
    /// counted, so a redaction that fires unexpectedly is visible.
    pub removed: Vec<Redaction>,
    /// Whether `text` alone exceeds what a GitHub issue body can hold, before
    /// any surrounding report template is added.
    pub oversized: bool,
}

fn category(key: &str) -> Option<Category> {
    if IDENTITY_KEYS.contains(&key) {
        Some(Category::Identity)
    } else if LINK_SECRET_KEYS.contains(&key) {
        Some(Category::LinkSecret)
    } else {
        None
    }
}

/// Splits a source line into its text and its line terminator.
///
/// `SyntaxLine::raw` carries the terminator the file used, `\r\n` included, and
/// the final line of a file that does not end in a newline carries none.
/// Rewriting a line has to put back exactly what was there, so that a redacted
/// backup still matches its source everywhere it was not redacted.
fn split_terminator(raw: &str) -> (&str, &str) {
    let text = raw
        .strip_suffix('\n')
        .map(|t| t.strip_suffix('\r').unwrap_or(t))
        .unwrap_or(raw);
    (text, &raw[text.len()..])
}

/// Replaces everything after the first `=` in a `set` line, preserving the
/// original spelling and spacing of the part that names the setting.
///
/// `parse_line` only produces `Command::Set` for a line that has an `=`, so the
/// separator is present for every key this is called with.
fn redact_set(raw: &str) -> String {
    let (text, terminator) = split_terminator(raw);
    match text.find('=') {
        Some(at) => format!("{}= {PLACEHOLDER}{terminator}", &text[..at]),
        None => raw.to_owned(),
    }
}

/// The craft name a dump prints in its `# name:` header, which is a comment and
/// so never reaches `Command::Set`.
fn redact_name_header(raw: &str) -> Option<String> {
    let (text, terminator) = split_terminator(raw);
    let trimmed = text.trim_start_matches('\u{feff}');
    let lead = &text[..text.len() - trimmed.trim_start().len()];
    let rest = trimmed.trim_start().strip_prefix(NAME_HEADER)?;
    // An empty header names no craft, so there is nothing to remove and no
    // redaction worth reporting.
    (!rest.trim().is_empty()).then(|| format!("{lead}{NAME_HEADER} {PLACEHOLDER}{terminator}"))
}

/// The backup as the user would paste it, with identifying and link-secret
/// values removed.
///
/// Line structure is preserved exactly: every source line appears in the output
/// at its original position, so the line numbers a reporter quotes still match
/// what a maintainer reads.
pub fn redact(document: &ConfigDocument) -> RedactedConfig {
    let mut removed = Vec::new();
    let mut lines = Vec::with_capacity(document.syntax.len());
    for line in &document.syntax {
        match &line.command {
            Command::Set { key, .. } => match category(key) {
                Some(category) => {
                    removed.push(Redaction {
                        key: key.clone(),
                        line: line.line,
                        category,
                    });
                    lines.push(redact_set(&line.raw));
                }
                None => lines.push(line.raw.clone()),
            },
            Command::Comment => match redact_name_header(&line.raw) {
                Some(redacted) => {
                    removed.push(Redaction {
                        key: "name".into(),
                        line: line.line,
                        category: Category::Identity,
                    });
                    lines.push(redacted);
                }
                None => lines.push(line.raw.clone()),
            },
            _ => lines.push(line.raw.clone()),
        }
    }
    let text = lines.concat();
    RedactedConfig {
        oversized: text.chars().count() > ISSUE_BODY_LIMIT,
        text,
        removed,
    }
}
