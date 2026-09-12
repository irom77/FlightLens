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

/// Where a report is filed. The reporter opens this in their own browser and
/// submits it themselves, so FlightLens needs no credentials and sends nothing.
const REPOSITORY: &str = "https://github.com/irom77/FlightLens";

/// How long the prefilled issue URL may get. GitHub itself accepts a little
/// more, but browsers and the intermediate sign-in redirect do not agree on a
/// limit, and a URL that is silently truncated files a report missing the half
/// that mattered. Refusing early is visible; truncation is not.
const URL_LIMIT: usize = 8_000;

/// How long a subject and a description may be. Both are bounded so the
/// assembled URL has a chance of fitting; `URL_LIMIT` is still checked, because
/// a description of accented or non-Latin text encodes to several times its
/// length.
const SUBJECT_LIMIT: usize = 120;
const BODY_LIMIT: usize = 2_000;

/// The shortest description worth filing. Not spam resistance -- nothing here
/// reaches a server -- but a report of "broken" costs a maintainer a round trip
/// the reporter can spare them.
const BODY_MINIMUM: usize = 20;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, TS, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReportKind {
    Bug,
    Feature,
}

impl ReportKind {
    /// The GitHub label the prefilled issue carries. Maintainers retriage on
    /// GitHub; this only saves the first sort.
    fn label(self) -> &'static str {
        match self {
            ReportKind::Bug => "bug",
            ReportKind::Feature => "enhancement",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FeedbackRequest {
    pub kind: ReportKind,
    pub subject: String,
    pub body: String,
    /// Whether the reporter chose to attach the open backup. Attaching is
    /// always the reporter's decision, never a default.
    pub include_config: bool,
    /// The running FlightLens version and host platform. The core cannot
    /// observe either, so the shell supplies both.
    pub app_version: String,
    pub platform: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct FeedbackReport {
    /// The prefilled issue URL to open in the reporter's browser.
    pub url: String,
    /// The issue body as it will arrive prefilled, so the dialog can show what
    /// the browser is about to be handed.
    pub issue_body: String,
    /// The redacted backup to place on the clipboard, and the same text the
    /// dialog shows for review. `None` when no configuration was attached.
    pub clipboard: Option<String>,
    /// Every value redaction removed, so the reporter sees what is missing
    /// from what they are about to publish.
    pub removed: Vec<Redaction>,
    /// Whether the redacted backup is too large to paste into an issue body.
    pub oversized: bool,
}

/// Percent-encodes a query parameter value.
///
/// Everything outside the unreserved set is encoded, so a description
/// containing `&`, `#`, or a newline cannot end the parameter early and drop
/// the rest of the report.
fn encode(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                encoded.push(byte as char)
            }
            _ => encoded.push_str(&format!("%{byte:02X}")),
        }
    }
    encoded
}

/// Why the report cannot be filed as it stands, in the order the dialog shows
/// its fields. Empty means it can.
///
/// Returned as reasons rather than a bare boolean so the dialog can say why the
/// submit button is disabled instead of leaving the reporter to guess.
pub fn problems(request: &FeedbackRequest, has_document: bool) -> Vec<String> {
    let subject = request.subject.trim().chars().count();
    let body = request.body.trim().chars().count();
    let mut problems = Vec::new();
    if subject == 0 {
        problems.push("Enter a subject.".into());
    } else if subject > SUBJECT_LIMIT {
        problems.push(format!(
            "Shorten the subject to {SUBJECT_LIMIT} characters."
        ));
    }
    if body < BODY_MINIMUM {
        problems.push(format!(
            "Describe the report in at least {BODY_MINIMUM} characters."
        ));
    } else if body > BODY_LIMIT {
        problems.push(format!(
            "Shorten the description to {BODY_LIMIT} characters."
        ));
    }
    if request.include_config && !has_document {
        problems.push("Open a backup, or do not attach a configuration.".into());
    }
    problems
}

fn counted(n: usize, one: &str, many: &str) -> String {
    format!("{n} {}", if n == 1 { one } else { many })
}

/// A one-line summary of what redaction removed, for the maintainer reading
/// the issue. The absent values are otherwise indistinguishable from settings
/// the firmware never wrote.
fn removal_note(removed: &[Redaction]) -> String {
    let identity = removed
        .iter()
        .filter(|r| r.category == Category::Identity)
        .count();
    let secrets = removed.len() - identity;
    format!(
        "FlightLens replaced {} and {} with `{PLACEHOLDER}` before copying.",
        counted(identity, "name value", "name values"),
        counted(secrets, "radio link identity", "radio link identities")
    )
}

/// The environment lines a maintainer needs to place the report: what was
/// running, and what it was reading.
fn environment(request: &FeedbackRequest, document: Option<&ConfigDocument>) -> String {
    let mut lines = format!(
        "- FlightLens {} on {}\n",
        request.app_version.trim(),
        request.platform.trim()
    );
    match document {
        Some(d) => {
            // The firmware header the backup declares, in preference to the
            // parsed family and version: it also names the target the build was
            // made for, and it is the line a maintainer will ask for anyway.
            lines.push_str(&match d.firmware.header.as_deref() {
                Some(header) => format!("- {}\n", header.trim_start_matches('#').trim()),
                None => format!(
                    "- {} {}\n",
                    d.firmware.family,
                    d.firmware.version.as_deref().unwrap_or("(no version)")
                ),
            });
            if let Some(board) = &d.firmware.board_name {
                lines.push_str(&format!("- Board {board}\n"));
            }
        }
        None => lines.push_str("- No backup was open.\n"),
    }
    lines
}

/// The issue body as GitHub will receive it prefilled.
fn issue_body(
    request: &FeedbackRequest,
    document: Option<&ConfigDocument>,
    redacted: Option<&RedactedConfig>,
) -> String {
    let mut text = format!(
        "### Description\n\n{}\n\n### Environment\n\n{}",
        request.body.trim(),
        environment(request, document)
    );
    text.push_str("\n### Configuration\n\n");
    match redacted {
        Some(r) if r.oversized => text.push_str(&format!(
            "The report and redacted backup together exceed the {ISSUE_BODY_LIMIT} \
             characters an issue body holds. {} Attach it as a file instead of pasting it.\n",
            removal_note(&r.removed)
        )),
        Some(r) => text.push_str(&format!(
            "{} Paste it from the clipboard between the lines below.\n\n```\n\n```\n",
            removal_note(&r.removed)
        )),
        None => text.push_str("The reporter did not attach a configuration.\n"),
    }
    text
}

/// The prefilled issue URL and the clipboard text that goes with it.
///
/// Nothing here contacts GitHub. The URL is opened in the reporter's browser
/// and the configuration is placed on their clipboard, so the report is filed
/// by the reporter, under their own account, after they have read it.
pub fn build_report(
    request: &FeedbackRequest,
    document: Option<&ConfigDocument>,
) -> Result<FeedbackReport, String> {
    let problems = problems(request, document.is_some());
    if let Some(first) = problems.first() {
        return Err(first.clone());
    }
    let mut redacted = request.include_config.then(|| {
        redact(
            document
                .expect("problems() rejects an attached configuration without an open document"),
        )
    });
    let mut body = issue_body(request, document, redacted.as_ref());
    if let Some(r) = &mut redacted {
        if !r.oversized && body.chars().count() + r.text.chars().count() > ISSUE_BODY_LIMIT {
            r.oversized = true;
            body = issue_body(request, document, Some(r));
        }
    }
    let url = format!(
        "{REPOSITORY}/issues/new?labels={}&title={}&body={}",
        encode(request.kind.label()),
        encode(request.subject.trim()),
        encode(&body)
    );
    if url.len() > URL_LIMIT {
        return Err(format!(
            "This report is too long to prefill. Shorten the description to under {BODY_LIMIT} characters."
        ));
    }
    Ok(FeedbackReport {
        url,
        issue_body: body,
        oversized: redacted.as_ref().is_some_and(|r| r.oversized),
        clipboard: redacted.as_ref().map(|r| r.text.clone()),
        removed: redacted.map(|r| r.removed).unwrap_or_default(),
    })
}
