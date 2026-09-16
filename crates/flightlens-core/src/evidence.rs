//! Compact source evidence for inspection and comparison, never for export.
use crate::{Command, ConfigDocument, DocumentView, SourceEvidence};
use std::collections::BTreeSet;

// Match the existing Inspector's JavaScript /\s/ marker recognition, including
// BOM whitespace (and excluding the Unicode NEL accepted by Rust's trim).
fn marker_space(c: char) -> bool {
    matches!(c, '\u{0009}'..='\u{000d}' | '\u{0020}' | '\u{00a0}' | '\u{1680}'
        | '\u{2000}'..='\u{200a}' | '\u{2028}' | '\u{2029}' | '\u{202f}'
        | '\u{205f}' | '\u{3000}' | '\u{feff}')
}

fn dump_all_marker(raw: &str) -> bool {
    let raw = raw.trim_matches(marker_space);
    let raw = raw.strip_prefix('#').unwrap_or(raw);
    let mut words = raw.split(marker_space).filter(|word| !word.is_empty());
    matches!((words.next(), words.next(), words.next()), (Some(a), Some(b), None)
        if a.eq_ignore_ascii_case("dump") && b.eq_ignore_ascii_case("all"))
}

impl ConfigDocument {
    /// Project inspection fields without cloning the full raw syntax array.
    pub fn document_view(&self) -> DocumentView {
        DocumentView {
            id: self.id.clone(),
            source_id: self.source_id.clone(),
            title: self.title.clone(),
            hash: self.hash.clone(),
            firmware: self.firmware.clone(),
            craft_name: self.craft_name.clone(),
            pilot_name: self.pilot_name.clone(),
            completeness: self.completeness.clone(),
            parameters: self.parameters.clone(),
            derived: self.derived.clone(),
            derived_note: self.derived_note.clone(),
            diagnostics: self.diagnostics.clone(),
            ports: self.ports.clone(),
            modes: self.modes.clone(),
            features: self.features.clone(),
            pid_profiles: self.pid_profiles.clone(),
            rate_profiles: self.rate_profiles.clone(),
            selected_pid: self.selected_pid,
            selected_rate: self.selected_rate,
            source_evidence: self.source_evidence(),
        }
    }

    /// Keep original line numbers, offsets and declaration order. Comparisons
    /// replay collections, defaults and malformed commands, so those must not
    /// be reduced to the parser's final state. Retain other command kinds
    /// conservatively; only comments, blanks and superseded settings are removed.
    pub fn source_evidence(&self) -> SourceEvidence {
        let parameter_lines: BTreeSet<_> = self.parameters.values().map(|p| p.line).collect();
        SourceEvidence {
            line_count: self.syntax.len() as u32,
            has_dump_all: self.syntax.iter().any(|line| dump_all_marker(&line.raw)),
            comparison_syntax: self
                .syntax
                .iter()
                .filter(|line| match line.command {
                    Command::Comment | Command::Blank => false,
                    Command::Set { .. } => parameter_lines.contains(&line.line),
                    _ => true,
                })
                .cloned()
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{analyze, Artifact};

    fn document(text: &str) -> Box<ConfigDocument> {
        let Artifact::Config(document) =
            analyze(text, "Synthetic evidence test", "virtual").unwrap()
        else {
            panic!("expected config");
        };
        document
    }

    #[test]
    fn document_view_preserves_every_non_source_field() {
        let document = document(include_str!(
            "../../../fixtures/configs/betaflight-4.5.0.dump"
        ));
        let original = serde_json::to_value(&document).unwrap();
        let view = serde_json::to_value(document.document_view()).unwrap();
        let mut expected = original.as_object().unwrap().clone();
        expected.remove("syntax");
        expected.insert(
            "sourceEvidence".into(),
            serde_json::to_value(document.source_evidence()).unwrap(),
        );
        assert_eq!(view, serde_json::Value::Object(expected));
        assert!(view.get("syntax").is_none());
        let decoded: DocumentView = serde_json::from_value(view.clone()).unwrap();
        assert_eq!(serde_json::to_value(decoded).unwrap(), view);
        assert!(serde_json::to_value(&document)
            .unwrap()
            .get("syntax")
            .is_some());
    }

    #[test]
    fn marker_matches_existing_inspector_rules() {
        for text in [
            "dump all",
            " # DUMP\tALL ",
            "\u{feff}# dump all\r",
            "#dump all",
        ] {
            assert!(dump_all_marker(text), "{text:?}");
        }
        for text in [
            "# dump",
            "# dump all extra",
            "## dump all",
            "dumpall",
            "\u{0085}dump all",
        ] {
            assert!(!dump_all_marker(text), "{text:?}");
        }
    }

    #[test]
    fn retains_final_parameter_sources_in_every_scope_including_invalid_values() {
        let doc = document("# Betaflight / STM32F405 (S405) 4.5.0\n# dump all\n\nset motor_poles = 12\nset motor_poles = 14\nprofile 0\nset p_roll = 45\nprofile 1\nset p_roll = 46\nrateprofile 0\nset roll_rc_rate = 100\nrateprofile 1\nset roll_rc_rate = invalid\n");
        let evidence = doc.source_evidence();
        assert_eq!(evidence.line_count as usize, doc.syntax.len());
        assert!(evidence.has_dump_all);
        assert!(!evidence.comparison_syntax.iter().any(|line| line.line <= 4));
        assert!(doc.parameters.values().any(|parameter| !parameter.valid));
        for parameter in doc.parameters.values() {
            let retained = evidence
                .comparison_syntax
                .iter()
                .find(|line| line.line == parameter.line)
                .unwrap();
            let original = doc
                .syntax
                .iter()
                .find(|line| line.line == parameter.line)
                .unwrap();
            assert_eq!(
                serde_json::to_value(retained).unwrap(),
                serde_json::to_value(original).unwrap()
            );
        }
    }

    #[test]
    fn preserves_ordered_replay_and_source_declarations_on_verified_and_unknown_firmware() {
        for version in ["4.5.0", "99.0.0"] {
            let doc = document(&format!("# Betaflight / STM32F405 (S405) {version}\n# dump all\nrxrange 0 1000 2000\nrxrange reset\nrxrange broken\ndefaults\ndefaults broken\nrxfail 0 h\nadjrange 0 0 1 900 1200 1 2\nvtx 0 0 1 1 1 900 1200\nvtxtable bands 2\nvtxtable broken\nfeature GPS\nfeature -GPS\nserial 0 64 115200 57600 0 115200\naux 0 0 0 900 1200 0 0\nunknown command\n"));
            let evidence = doc.source_evidence();
            let expected: Vec<_> = doc
                .syntax
                .iter()
                .filter(|line| !matches!(line.command, Command::Comment | Command::Blank))
                .collect();
            assert!(expected
                .iter()
                .any(|line| matches!(line.command, Command::Malformed)));
            assert_eq!(
                serde_json::to_value(&evidence.comparison_syntax).unwrap(),
                serde_json::to_value(expected).unwrap()
            );
        }
    }

    #[test]
    fn unknown_scope_declarations_are_not_treated_as_superseded() {
        let doc = document("set unverified_setting = 1\nset unverified_setting = 2\n");
        let evidence = doc.source_evidence();
        assert_eq!(doc.parameters.len(), 2);
        assert_eq!(evidence.comparison_syntax.len(), 2);
    }

    #[test]
    fn repeated_comments_and_settings_do_not_expand_evidence() {
        let base = "# Betaflight / STM32F405 (S405) 4.5.0\n# dump all\nset motor_poles = 14\n";
        let small = document(base).source_evidence();
        let large = document(&format!(
            "{}{}",
            "# synthetic padding\nset motor_poles = 12\n".repeat(2000),
            base
        ))
        .source_evidence();
        assert_eq!(small.comparison_syntax.len(), 1);
        assert_eq!(large.comparison_syntax.len(), 1);
        assert_eq!(
            large.comparison_syntax[0].raw,
            small.comparison_syntax[0].raw
        );
        assert_eq!(large.line_count, 4003);
        assert_eq!(large.comparison_syntax[0].line, 4003);
    }
}
