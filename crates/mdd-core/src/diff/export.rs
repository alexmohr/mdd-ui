// SPDX-License-Identifier: Apache-2.0
// SPDX-FileCopyrightText: 2026 Alexander Mohr

use std::io::Write;

use super::compare::{DiffResult, ElementDiff, PropertyDiff};
use crate::tree::DiffStatus;

/// Writes a plain-text diff report to the given writer.
///
/// Unchanged elements are omitted to keep output focused on actual changes.
pub fn write_text_report(w: &mut impl Write, diff: &DiffResult) -> std::io::Result<()> {
    writeln!(w, "Comparing: {} vs {}", diff.old_name, diff.new_name)?;
    writeln!(
        w,
        "+{} added, -{} removed, ~{} modified, {} unchanged",
        diff.summary.added, diff.summary.removed, diff.summary.modified, diff.summary.unchanged,
    )?;

    if !diff.ecu_diffs.is_empty() {
        writeln!(w)?;
        writeln!(w, "ECU property changes:")?;
        for prop in &diff.ecu_diffs {
            write_property(w, prop, 1)?;
        }
    }

    write_section(w, "Variants", &diff.variants, "Variant")?;
    write_section(
        w,
        "Functional groups",
        &diff.functional_groups,
        "FunctionalGroup",
    )?;
    write_section(w, "DTCs", &diff.dtcs, "DTC")?;

    Ok(())
}

fn write_section(
    w: &mut impl Write,
    heading: &str,
    elements: &[ElementDiff],
    item_kind: &str,
) -> std::io::Result<()> {
    let has_changes = elements.iter().any(|e| e.status != DiffStatus::Unchanged);
    if elements.is_empty() || !has_changes {
        return Ok(());
    }
    writeln!(w)?;
    writeln!(w, "{heading}:")?;
    for elem in elements {
        write_element(w, elem, 1, Some(item_kind))?;
    }
    Ok(())
}

fn write_element(
    w: &mut impl Write,
    elem: &ElementDiff,
    depth: usize,
    kind: Option<&str>,
) -> std::io::Result<()> {
    if elem.status == DiffStatus::Unchanged {
        return Ok(());
    }

    let indent = "  ".repeat(depth);
    let marker = status_marker(elem.status);

    // Inline single property diff for modified leaf nodes (matches TUI behavior).
    let is_leaf_with_single_diff = elem.children.is_empty() && elem.property_diffs.len() == 1;
    if elem.status == DiffStatus::Modified && is_leaf_with_single_diff {
        let Some(p) = elem.property_diffs.first() else {
            return Ok(());
        };
        let kind_label = kind.map_or(String::new(), |k| format!(" ({k})"));
        return writeln!(
            w,
            "{indent}{marker} {}{kind_label}: {} -> {}",
            elem.name, p.old_value, p.new_value,
        );
    }

    let kind_label = kind.map_or(String::new(), |k| format!(" ({k})"));
    writeln!(w, "{indent}{marker} {}{kind_label}", elem.name)?;

    let child_depth = depth.saturating_add(1);
    for prop in &elem.property_diffs {
        write_property(w, prop, child_depth)?;
    }
    for child in &elem.children {
        let child_kind = derive_child_kind(&elem.name, kind);
        write_element(w, child, child_depth, child_kind)?;
    }
    Ok(())
}

/// Derive the kind label for children based on the parent's name.
///
/// Category nodes created by the compare module have well-known names
/// (e.g. "Services", "`SingleEcuJobs`"). Their children inherit a singular
/// kind label. For non-category parents, children get no label since their
/// context is already clear from the heading.
fn derive_child_kind<'a>(parent_name: &str, _parent_kind: Option<&'a str>) -> Option<&'a str> {
    match parent_name {
        "Services" => Some("Service"),
        "SingleEcuJobs" => Some("SingleEcuJob"),
        "StateCharts" => Some("StateChart"),
        _ => None,
    }
}

fn write_property(w: &mut impl Write, prop: &PropertyDiff, depth: usize) -> std::io::Result<()> {
    let indent = "  ".repeat(depth);
    writeln!(
        w,
        "{indent}{}: {} -> {}",
        prop.name, prop.old_value, prop.new_value,
    )
}

fn status_marker(status: DiffStatus) -> &'static str {
    match status {
        DiffStatus::Added => "[+]",
        DiffStatus::Removed => "[-]",
        DiffStatus::Modified => "[~]",
        DiffStatus::Unchanged => "[ ]",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::compare::DiffSummary;

    #[test]
    fn empty_diff_prints_header_and_summary() {
        let diff = DiffResult {
            old_name: "old.mdd".into(),
            new_name: "new.mdd".into(),
            ecu_diffs: vec![],
            variants: vec![],
            functional_groups: vec![],
            dtcs: vec![],
            summary: DiffSummary::default(),
        };
        let mut buf = Vec::new();
        write_text_report(&mut buf, &diff).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("Comparing: old.mdd vs new.mdd"));
        assert!(output.contains("+0 added, -0 removed, ~0 modified, 0 unchanged"));
    }

    #[test]
    fn unchanged_elements_are_skipped() {
        let diff = DiffResult {
            old_name: "a.mdd".into(),
            new_name: "b.mdd".into(),
            ecu_diffs: vec![],
            variants: vec![
                ElementDiff {
                    name: "V1".into(),
                    status: DiffStatus::Unchanged,
                    property_diffs: vec![],
                    children: vec![],
                },
                ElementDiff {
                    name: "V2".into(),
                    status: DiffStatus::Added,
                    property_diffs: vec![],
                    children: vec![],
                },
            ],
            functional_groups: vec![],
            dtcs: vec![],
            summary: DiffSummary {
                added: 1,
                removed: 0,
                modified: 0,
                unchanged: 1,
            },
        };
        let mut buf = Vec::new();
        write_text_report(&mut buf, &diff).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(
            !output.contains("V1"),
            "unchanged element should be omitted"
        );
        assert!(output.contains("[+] V2 (Variant)"));
    }

    #[test]
    fn all_unchanged_section_is_omitted() {
        let diff = DiffResult {
            old_name: "a.mdd".into(),
            new_name: "b.mdd".into(),
            ecu_diffs: vec![],
            variants: vec![ElementDiff {
                name: "V1".into(),
                status: DiffStatus::Unchanged,
                property_diffs: vec![],
                children: vec![],
            }],
            functional_groups: vec![],
            dtcs: vec![],
            summary: DiffSummary {
                added: 0,
                removed: 0,
                modified: 0,
                unchanged: 1,
            },
        };
        let mut buf = Vec::new();
        write_text_report(&mut buf, &diff).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(
            !output.contains("Variants:"),
            "fully unchanged section should be omitted"
        );
    }

    #[test]
    fn single_property_diff_is_inlined() {
        let diff = DiffResult {
            old_name: "a.mdd".into(),
            new_name: "b.mdd".into(),
            ecu_diffs: vec![],
            variants: vec![],
            functional_groups: vec![],
            dtcs: vec![ElementDiff {
                name: "DTC_001".into(),
                status: DiffStatus::Modified,
                property_diffs: vec![PropertyDiff {
                    name: "text".into(),
                    old_value: "old desc".into(),
                    new_value: "new desc".into(),
                }],
                children: vec![],
            }],
            summary: DiffSummary {
                added: 0,
                removed: 0,
                modified: 1,
                unchanged: 0,
            },
        };
        let mut buf = Vec::new();
        write_text_report(&mut buf, &diff).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(
            output.contains("[~] DTC_001 (DTC): old desc -> new desc"),
            "single diff should be inlined. Got:\n{output}",
        );
    }

    #[test]
    fn kind_labels_and_nested_hierarchy() {
        let diff = DiffResult {
            old_name: "a.mdd".into(),
            new_name: "b.mdd".into(),
            ecu_diffs: vec![],
            variants: vec![ElementDiff {
                name: "BaseVariant".into(),
                status: DiffStatus::Modified,
                property_diffs: vec![],
                children: vec![ElementDiff {
                    name: "Services".into(),
                    status: DiffStatus::Modified,
                    property_diffs: vec![],
                    children: vec![
                        ElementDiff {
                            name: "ReadDID".into(),
                            status: DiffStatus::Added,
                            property_diffs: vec![],
                            children: vec![],
                        },
                        ElementDiff {
                            name: "WriteDID".into(),
                            status: DiffStatus::Modified,
                            property_diffs: vec![
                                PropertyDiff {
                                    name: "addressing".into(),
                                    old_value: "physical".into(),
                                    new_value: "functional".into(),
                                },
                                PropertyDiff {
                                    name: "is_cyclic".into(),
                                    old_value: "false".into(),
                                    new_value: "true".into(),
                                },
                            ],
                            children: vec![],
                        },
                    ],
                }],
            }],
            functional_groups: vec![],
            dtcs: vec![],
            summary: DiffSummary {
                added: 1,
                removed: 0,
                modified: 1,
                unchanged: 0,
            },
        };
        let mut buf = Vec::new();
        write_text_report(&mut buf, &diff).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(
            output.contains("[~] BaseVariant (Variant)"),
            "top-level item gets kind label"
        );
        assert!(
            output.contains("[+] ReadDID (Service)"),
            "service child gets kind label"
        );
        assert!(
            output.contains("[~] WriteDID (Service)"),
            "modified service gets kind label"
        );
        assert!(
            output.contains("addressing: physical -> functional"),
            "multi-property diffs listed"
        );
    }

    #[test]
    fn elements_render_with_markers_and_indentation() {
        let diff = DiffResult {
            old_name: "a.mdd".into(),
            new_name: "b.mdd".into(),
            ecu_diffs: vec![],
            variants: vec![ElementDiff {
                name: "V1".into(),
                status: DiffStatus::Added,
                property_diffs: vec![],
                children: vec![ElementDiff {
                    name: "child".into(),
                    status: DiffStatus::Modified,
                    property_diffs: vec![PropertyDiff {
                        name: "version".into(),
                        old_value: "1".into(),
                        new_value: "2".into(),
                    }],
                    children: vec![],
                }],
            }],
            functional_groups: vec![],
            dtcs: vec![ElementDiff {
                name: "DTC_001".into(),
                status: DiffStatus::Removed,
                property_diffs: vec![],
                children: vec![],
            }],
            summary: DiffSummary {
                added: 1,
                removed: 1,
                modified: 0,
                unchanged: 0,
            },
        };
        let mut buf = Vec::new();
        write_text_report(&mut buf, &diff).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("  [+] V1 (Variant)"));
        // Single-diff inlined: "child: 1 -> 2"
        assert!(output.contains("[~] child: 1 -> 2"));
        assert!(output.contains("  [-] DTC_001 (DTC)"));
    }
}
