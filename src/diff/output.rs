/*
 * SPDX-License-Identifier: Apache-2.0
 * SPDX-FileCopyrightText: 2026 Alexander Mohr
 */

//! Output formatting for diff results.
//!
//! Provides comprehensive text and JSON output for all diff details including
//! node properties, section changes, row changes, cell types, jump targets, etc.

use std::str::FromStr;

use super::tree_differ::{DiffResult, NodeChange, RowChange, SectionChange};

/// Output format for diff results.
#[derive(Debug, Clone, Copy, Default)]
pub enum OutputFormat {
    /// Human-readable text output (default).
    #[default]
    Text,
    /// JSON output for programmatic consumption.
    Json,
}

impl FromStr for OutputFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "text" => Ok(Self::Text),
            "json" => Ok(Self::Json),
            other => Err(format!(
                "Invalid format '{other}'. Valid options: text, json"
            )),
        }
    }
}

/// Write the diff result to stdout in the specified format.
pub fn write_diff(result: &DiffResult, base_file: &str, compare_file: &str, format: OutputFormat) {
    match format {
        OutputFormat::Text => write_text_diff(result, base_file, compare_file),
        OutputFormat::Json => write_json_diff(result, base_file, compare_file),
    }
}

fn write_text_diff(result: &DiffResult, base_file: &str, compare_file: &str) {
    println!("MDD Diff Report (Exhaustive)");
    println!("{}", "=".repeat(80));
    println!();
    println!("Base file:    {base_file}");
    println!("Compare file: {compare_file}");
    println!();

    // Summary
    println!("Summary");
    println!("{}", "-".repeat(40));
    println!("Total nodes in base:    {}", result.stats.total_base_nodes);
    println!(
        "Total nodes in compare: {}",
        result.stats.total_compare_nodes
    );
    println!("Removed (only in base): {}", result.stats.removed_count);
    println!("Added (only in compare): {}", result.stats.added_count);
    println!("Modified:               {}", result.stats.modified_count);
    println!("Unchanged:              {}", result.stats.unchanged_count);
    println!();

    // Only in base (removed) - with full details
    if !result.only_in_base.is_empty() {
        println!("Removed Nodes (only in base)");
        println!("{}", "=".repeat(60));
        for node in &result.only_in_base {
            println!();
            println!("  - PATH: {}", node.path);
            println!("    Text: {}", node.text);
            println!("    Type: {:?}", node.node_type);
            println!("    Depth: {}", node.details.depth);
            println!("    Has children: {}", node.details.has_children);
            if let Some(ref st) = node.details.section_type {
                println!("    Section type: {st:?}");
            }
            if let Some(ref slt) = node.details.service_list_type {
                println!("    Service list type: {slt:?}");
            }
            if let Some(pid) = node.details.param_id {
                println!("    Param ID: {pid}");
            }
            if !node.details.parent_ref_names.is_empty() {
                println!("    Parent refs: {:?}", node.details.parent_ref_names);
            }
            println!(
                "    Detail sections ({}): {:?}",
                node.details.detail_section_count, node.details.detail_section_titles
            );
        }
        println!();
    }

    // Only in compare (added) - with full details
    if !result.only_in_compare.is_empty() {
        println!("Added Nodes (only in compare)");
        println!("{}", "=".repeat(60));
        for node in &result.only_in_compare {
            println!();
            println!("  + PATH: {}", node.path);
            println!("    Text: {}", node.text);
            println!("    Type: {:?}", node.node_type);
            println!("    Depth: {}", node.details.depth);
            println!("    Has children: {}", node.details.has_children);
            if let Some(ref st) = node.details.section_type {
                println!("    Section type: {st:?}");
            }
            if let Some(ref slt) = node.details.service_list_type {
                println!("    Service list type: {slt:?}");
            }
            if let Some(pid) = node.details.param_id {
                println!("    Param ID: {pid}");
            }
            if !node.details.parent_ref_names.is_empty() {
                println!("    Parent refs: {:?}", node.details.parent_ref_names);
            }
            println!(
                "    Detail sections ({}): {:?}",
                node.details.detail_section_count, node.details.detail_section_titles
            );
        }
        println!();
    }

    // Modified nodes - with exhaustive change details
    if !result.modified.is_empty() {
        println!("Modified Nodes");
        println!("{}", "=".repeat(60));
        for modified in &result.modified {
            println!();
            println!("  ~ PATH: {}", modified.path);
            for change in &modified.changes {
                print_node_change(change, 4);
            }
        }
        println!();
    }

    if result.stats.added_count == 0
        && result.stats.removed_count == 0
        && result.stats.modified_count == 0
    {
        println!("No differences found.");
    }
}

fn print_node_change(change: &NodeChange, indent: usize) {
    let pad = " ".repeat(indent);
    match change {
        NodeChange::PropertyChanged {
            property,
            base,
            compare,
        } => {
            println!("{pad}[Property] {property}:");
            println!("{pad}  Base:    {base}");
            println!("{pad}  Compare: {compare}");
        }
        NodeChange::SectionAdded { section, details } => {
            println!("{pad}[Section Added] {section}");
            for detail in details {
                println!("{pad}  {detail}");
            }
        }
        NodeChange::SectionRemoved { section, details } => {
            println!("{pad}[Section Removed] {section}");
            for detail in details {
                println!("{pad}  {detail}");
            }
        }
        NodeChange::SectionModified { section, changes } => {
            println!("{pad}[Section Modified] {section}");
            for change in changes {
                print_section_change(change, indent.saturating_add(2));
            }
        }
    }
}

fn print_section_change(change: &SectionChange, indent: usize) {
    let pad = " ".repeat(indent);
    match change {
        SectionChange::PropertyChanged {
            property,
            base,
            compare,
        } => {
            println!("{pad}Property '{property}': {base} -> {compare}");
        }
        SectionChange::RowAdded { row_summary } => {
            println!("{pad}+ Row added: {row_summary}");
        }
        SectionChange::RowRemoved { row_summary } => {
            println!("{pad}- Row removed: {row_summary}");
        }
        SectionChange::RowModified { row_index, changes } => {
            println!("{pad}~ Row {row_index} modified:");
            for change in changes {
                print_row_change(change, indent.saturating_add(2));
            }
        }
        SectionChange::HeaderChanged { changes } => {
            println!("{pad}~ Table header changed:");
            for change in changes {
                print_row_change(change, indent.saturating_add(2));
            }
        }
        SectionChange::ConstraintsChanged { base, compare } => {
            println!("{pad}~ Constraints changed:");
            println!("{pad}  Base:    {base}");
            println!("{pad}  Compare: {compare}");
        }
        SectionChange::ContentTypeChanged { base, compare } => {
            println!("{pad}~ Content type changed: {base} -> {compare}");
        }
        SectionChange::LineAdded { line } => {
            println!("{pad}+ Line: {line}");
        }
        SectionChange::LineRemoved { line } => {
            println!("{pad}- Line: {line}");
        }
        SectionChange::SubsectionAdded { title } => {
            println!("{pad}+ Subsection added: {title}");
        }
        SectionChange::SubsectionRemoved { title } => {
            println!("{pad}- Subsection removed: {title}");
        }
        SectionChange::SubsectionModified { title, changes } => {
            println!("{pad}~ Subsection '{title}' modified:");
            for change in changes {
                print_section_change(change, indent.saturating_add(2));
            }
        }
    }
}

fn print_row_change(change: &RowChange, indent: usize) {
    let pad = " ".repeat(indent);
    match change {
        RowChange::CellValueChanged {
            column,
            base,
            compare,
        } => {
            println!("{pad}Cell[{column}] value: \"{base}\" -> \"{compare}\"");
        }
        RowChange::CellTypeChanged {
            column,
            base,
            compare,
        } => {
            println!("{pad}Cell[{column}] type: {base} -> {compare}");
        }
        RowChange::CellJumpTargetChanged {
            column,
            base,
            compare,
        } => {
            println!("{pad}Cell[{column}] jump target: {base} -> {compare}");
        }
        RowChange::IndentChanged { base, compare } => {
            println!("{pad}Indent: {base} -> {compare}");
        }
        RowChange::RowTypeChanged { base, compare } => {
            println!("{pad}Row type: {base} -> {compare}");
        }
        RowChange::MetadataChanged { base, compare } => {
            println!("{pad}Metadata: {base} -> {compare}");
        }
        RowChange::ColumnCountChanged { base, compare } => {
            println!("{pad}Column count: {base} -> {compare}");
        }
    }
}

fn write_json_diff(result: &DiffResult, base_file: &str, compare_file: &str) {
    println!("{{");
    println!("  \"base_file\": \"{}\",", escape_json(base_file));
    println!("  \"compare_file\": \"{}\",", escape_json(compare_file));
    println!("  \"stats\": {{");
    println!(
        "    \"total_base_nodes\": {},",
        result.stats.total_base_nodes
    );
    println!(
        "    \"total_compare_nodes\": {},",
        result.stats.total_compare_nodes
    );
    println!("    \"removed_count\": {},", result.stats.removed_count);
    println!("    \"added_count\": {},", result.stats.added_count);
    println!("    \"modified_count\": {},", result.stats.modified_count);
    println!("    \"unchanged_count\": {}", result.stats.unchanged_count);
    println!("  }},");

    // Removed nodes with full details
    println!("  \"removed\": [");
    let base_len = result.only_in_base.len();
    for (i, node) in result.only_in_base.iter().enumerate() {
        let is_last = i.checked_add(1).is_some_and(|next| next == base_len);
        let comma = if is_last { "" } else { "," };
        println!("    {{");
        println!("      \"path\": \"{}\",", escape_json(&node.path));
        println!("      \"text\": \"{}\",", escape_json(&node.text));
        println!("      \"node_type\": \"{:?}\",", node.node_type);
        println!("      \"depth\": {},", node.details.depth);
        println!("      \"has_children\": {},", node.details.has_children);
        println!(
            "      \"section_type\": {:?},",
            node.details.section_type.map(|s| format!("{s:?}"))
        );
        println!(
            "      \"service_list_type\": {:?},",
            node.details.service_list_type.map(|s| format!("{s:?}"))
        );
        println!("      \"param_id\": {:?},", node.details.param_id);
        println!(
            "      \"parent_ref_names\": {:?},",
            node.details.parent_ref_names
        );
        println!(
            "      \"detail_section_count\": {},",
            node.details.detail_section_count
        );
        print!("      \"detail_section_titles\": [");
        for (j, title) in node.details.detail_section_titles.iter().enumerate() {
            let t_comma = if j
                .checked_add(1)
                .is_some_and(|n| n == node.details.detail_section_titles.len())
            {
                ""
            } else {
                ", "
            };
            print!("\"{}\"{t_comma}", escape_json(title));
        }
        println!("]");
        println!("    }}{comma}");
    }
    println!("  ],");

    // Added nodes with full details
    println!("  \"added\": [");
    let compare_len = result.only_in_compare.len();
    for (i, node) in result.only_in_compare.iter().enumerate() {
        let is_last = i.checked_add(1).is_some_and(|next| next == compare_len);
        let comma = if is_last { "" } else { "," };
        println!("    {{");
        println!("      \"path\": \"{}\",", escape_json(&node.path));
        println!("      \"text\": \"{}\",", escape_json(&node.text));
        println!("      \"node_type\": \"{:?}\",", node.node_type);
        println!("      \"depth\": {},", node.details.depth);
        println!("      \"has_children\": {},", node.details.has_children);
        println!(
            "      \"section_type\": {:?},",
            node.details.section_type.map(|s| format!("{s:?}"))
        );
        println!(
            "      \"service_list_type\": {:?},",
            node.details.service_list_type.map(|s| format!("{s:?}"))
        );
        println!("      \"param_id\": {:?},", node.details.param_id);
        println!(
            "      \"parent_ref_names\": {:?},",
            node.details.parent_ref_names
        );
        println!(
            "      \"detail_section_count\": {},",
            node.details.detail_section_count
        );
        print!("      \"detail_section_titles\": [");
        for (j, title) in node.details.detail_section_titles.iter().enumerate() {
            let t_comma = if j
                .checked_add(1)
                .is_some_and(|n| n == node.details.detail_section_titles.len())
            {
                ""
            } else {
                ", "
            };
            print!("\"{}\"{t_comma}", escape_json(title));
        }
        println!("]");
        println!("    }}{comma}");
    }
    println!("  ],");

    // Modified nodes with exhaustive change details
    println!("  \"modified\": [");
    let modified_len = result.modified.len();
    for (i, modified) in result.modified.iter().enumerate() {
        let is_last = i.checked_add(1).is_some_and(|next| next == modified_len);
        let comma = if is_last { "" } else { "," };
        println!("    {{");
        println!("      \"path\": \"{}\",", escape_json(&modified.path));
        println!("      \"changes\": [");
        let changes_len = modified.changes.len();
        for (j, change) in modified.changes.iter().enumerate() {
            let is_last_change = j.checked_add(1).is_some_and(|next| next == changes_len);
            let change_comma = if is_last_change { "" } else { "," };
            print_json_node_change(change, 8, change_comma);
        }
        println!("      ]");
        println!("    }}{comma}");
    }
    println!("  ]");

    println!("}}");
}

fn print_json_node_change(change: &NodeChange, indent: usize, comma: &str) {
    let pad = " ".repeat(indent);
    match change {
        NodeChange::PropertyChanged {
            property,
            base,
            compare,
        } => {
            println!(
                "{pad}{{ \"type\": \"property_changed\", \"property\": \"{}\", \"base\": \"{}\", \
                 \"compare\": \"{}\" }}{comma}",
                escape_json(property),
                escape_json(base),
                escape_json(compare)
            );
        }
        NodeChange::SectionAdded { section, details } => {
            println!(
                "{pad}{{ \"type\": \"section_added\", \"section\": \"{}\", \"details\": [",
                escape_json(section)
            );
            print_json_string_array(details, indent.saturating_add(2));
            println!("{pad}] }}{comma}");
        }
        NodeChange::SectionRemoved { section, details } => {
            println!(
                "{pad}{{ \"type\": \"section_removed\", \"section\": \"{}\", \"details\": [",
                escape_json(section)
            );
            print_json_string_array(details, indent.saturating_add(2));
            println!("{pad}] }}{comma}");
        }
        NodeChange::SectionModified { section, changes } => {
            println!(
                "{pad}{{ \"type\": \"section_modified\", \"section\": \"{}\", \"changes\": [",
                escape_json(section)
            );
            print_json_section_changes(changes, indent.saturating_add(2));
            println!("{pad}] }}{comma}");
        }
    }
}

fn print_json_string_array(items: &[String], indent: usize) {
    let pad = " ".repeat(indent);
    let len = items.len();
    for (i, item) in items.iter().enumerate() {
        let is_last = i.checked_add(1).is_some_and(|n| n == len);
        let comma = if is_last { "" } else { "," };
        println!("{pad}\"{}\"{comma}", escape_json(item));
    }
}

fn print_json_section_changes(changes: &[SectionChange], indent: usize) {
    let pad = " ".repeat(indent);
    let len = changes.len();
    for (i, change) in changes.iter().enumerate() {
        let is_last = i.checked_add(1).is_some_and(|n| n == len);
        let comma = if is_last { "" } else { "," };
        print_json_section_change(change, indent, comma);
    }
    _ = pad; // silence unused warning for empty changes
}

fn print_json_section_change(change: &SectionChange, indent: usize, comma: &str) {
    let pad = " ".repeat(indent);
    match change {
        SectionChange::PropertyChanged {
            property,
            base,
            compare,
        } => {
            println!(
                "{pad}{{ \"type\": \"property_changed\", \"property\": \"{}\", \"base\": \"{}\", \
                 \"compare\": \"{}\" }}{comma}",
                escape_json(property),
                escape_json(base),
                escape_json(compare)
            );
        }
        SectionChange::RowAdded { row_summary } => {
            println!(
                "{pad}{{ \"type\": \"row_added\", \"summary\": \"{}\" }}{comma}",
                escape_json(row_summary)
            );
        }
        SectionChange::RowRemoved { row_summary } => {
            println!(
                "{pad}{{ \"type\": \"row_removed\", \"summary\": \"{}\" }}{comma}",
                escape_json(row_summary)
            );
        }
        SectionChange::RowModified { row_index, changes } => {
            println!(
                "{pad}{{ \"type\": \"row_modified\", \"row_index\": {row_index}, \"changes\": ["
            );
            print_json_row_changes(changes, indent.saturating_add(2));
            println!("{pad}] }}{comma}");
        }
        SectionChange::HeaderChanged { changes } => {
            println!("{pad}{{ \"type\": \"header_changed\", \"changes\": [");
            print_json_row_changes(changes, indent.saturating_add(2));
            println!("{pad}] }}{comma}");
        }
        SectionChange::ConstraintsChanged { base, compare } => {
            println!(
                "{pad}{{ \"type\": \"constraints_changed\", \"base\": \"{}\", \"compare\": \"{}\" \
                 }}{comma}",
                escape_json(base),
                escape_json(compare)
            );
        }
        SectionChange::ContentTypeChanged { base, compare } => {
            println!(
                "{pad}{{ \"type\": \"content_type_changed\", \"base\": \"{}\", \"compare\": \
                 \"{}\" }}{comma}",
                escape_json(base),
                escape_json(compare)
            );
        }
        SectionChange::LineAdded { line } => {
            println!(
                "{pad}{{ \"type\": \"line_added\", \"line\": \"{}\" }}{comma}",
                escape_json(line)
            );
        }
        SectionChange::LineRemoved { line } => {
            println!(
                "{pad}{{ \"type\": \"line_removed\", \"line\": \"{}\" }}{comma}",
                escape_json(line)
            );
        }
        SectionChange::SubsectionAdded { title } => {
            println!(
                "{pad}{{ \"type\": \"subsection_added\", \"title\": \"{}\" }}{comma}",
                escape_json(title)
            );
        }
        SectionChange::SubsectionRemoved { title } => {
            println!(
                "{pad}{{ \"type\": \"subsection_removed\", \"title\": \"{}\" }}{comma}",
                escape_json(title)
            );
        }
        SectionChange::SubsectionModified { title, changes } => {
            println!(
                "{pad}{{ \"type\": \"subsection_modified\", \"title\": \"{}\", \"changes\": [",
                escape_json(title)
            );
            print_json_section_changes(changes, indent.saturating_add(2));
            println!("{pad}] }}{comma}");
        }
    }
}

fn print_json_row_changes(changes: &[RowChange], indent: usize) {
    let pad = " ".repeat(indent);
    let len = changes.len();
    for (i, change) in changes.iter().enumerate() {
        let is_last = i.checked_add(1).is_some_and(|n| n == len);
        let comma = if is_last { "" } else { "," };
        print_json_row_change(change, indent, comma);
    }
    _ = pad; // silence unused warning for empty changes
}

fn print_json_row_change(change: &RowChange, indent: usize, comma: &str) {
    let pad = " ".repeat(indent);
    match change {
        RowChange::CellValueChanged {
            column,
            base,
            compare,
        } => {
            println!(
                "{pad}{{ \"type\": \"cell_value_changed\", \"column\": {column}, \"base\": \
                 \"{}\", \"compare\": \"{}\" }}{comma}",
                escape_json(base),
                escape_json(compare)
            );
        }
        RowChange::CellTypeChanged {
            column,
            base,
            compare,
        } => {
            println!(
                "{pad}{{ \"type\": \"cell_type_changed\", \"column\": {column}, \"base\": \"{}\", \
                 \"compare\": \"{}\" }}{comma}",
                escape_json(base),
                escape_json(compare)
            );
        }
        RowChange::CellJumpTargetChanged {
            column,
            base,
            compare,
        } => {
            println!(
                "{pad}{{ \"type\": \"cell_jump_target_changed\", \"column\": {column}, \"base\": \
                 \"{}\", \"compare\": \"{}\" }}{comma}",
                escape_json(base),
                escape_json(compare)
            );
        }
        RowChange::IndentChanged { base, compare } => {
            println!(
                "{pad}{{ \"type\": \"indent_changed\", \"base\": {base}, \"compare\": {compare} \
                 }}{comma}"
            );
        }
        RowChange::RowTypeChanged { base, compare } => {
            println!(
                "{pad}{{ \"type\": \"row_type_changed\", \"base\": \"{}\", \"compare\": \"{}\" \
                 }}{comma}",
                escape_json(base),
                escape_json(compare)
            );
        }
        RowChange::MetadataChanged { base, compare } => {
            println!(
                "{pad}{{ \"type\": \"metadata_changed\", \"base\": \"{}\", \"compare\": \"{}\" \
                 }}{comma}",
                escape_json(base),
                escape_json(compare)
            );
        }
        RowChange::ColumnCountChanged { base, compare } => {
            println!(
                "{pad}{{ \"type\": \"column_count_changed\", \"base\": {base}, \"compare\": \
                 {compare} }}{comma}"
            );
        }
    }
}

fn escape_json(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t")
}
