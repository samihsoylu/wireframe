// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
use super::*;

/// Row labels are indexed independently for each control in click order.
#[derive(Clone, Debug, PartialEq)]
pub struct RowSelection {
    pub labels: Vec<String>,
    pub allow_none: bool,
    pub value: i32,
    pub mixed: bool,
    pub explicit: bool,
    targets: Vec<(Id, usize, i32)>,
}

impl RowSelection {
    pub fn captions(&self) -> Vec<String> {
        self.labels
            .iter()
            .map(|label| crate::rich_text::RichText::parse(label, false).display_text(false))
            .collect()
    }
}

pub fn selectable_row_labels(kind: &str, values: &Properties) -> Option<Vec<String>> {
    let source = match values.get("text") {
        Some(Value::Text(text)) => text.as_str(),
        _ => "",
    };
    let header = values.get("header") == Some(&Value::Bool(true));
    Some(match kind {
        "list" => crate::list::labels(source, header)
            .into_iter()
            .map(str::to_owned)
            .collect(),
        "dropdown" => crate::dropdown::selectable_labels(source)
            .into_iter()
            .map(str::to_owned)
            .collect(),
        "table" => crate::rich_text::structured::Grid::selectable_labels(source, header),
        "accordion" => crate::accordion::labels(source),
        "tree" => crate::tree::labels(source),
        "menu" | "menu_bar" => crate::menu::labels(source, kind == "menu_bar"),
        "tabs" | "vertical_tabs" => crate::tabs::labels(source, kind == "vertical_tabs"),
        "link_bar" => crate::navigation::labels(source, false),
        "button_bar" => crate::button_bar::labels(source)
            .into_iter()
            .map(str::to_owned)
            .collect(),
        _ => return None,
    })
}

impl Editor {
    pub fn row_selection(&self) -> Option<RowSelection> {
        let ids = self.ordered_selection();
        if ids.is_empty() {
            return None;
        }
        let mut result = RowSelection {
            labels: vec![],
            allow_none: true,
            value: -1,
            mixed: false,
            explicit: false,
            targets: vec![],
        };
        let mut rows = vec![];
        let nodes = self.nodes();
        let by_id: HashMap<_, _> = nodes.iter().map(|node| (node.id, node)).collect();
        let context = self.property_context();
        for (i, id) in ids.into_iter().enumerate() {
            let node = by_id.get(&id)?;
            let NodeContent::Control { kind, .. } = &node.content else {
                return None;
            };
            let spec = catalog::get(kind)?;
            let values = spec.effective(node, &context).ok()?;
            let labels = selectable_row_labels(spec.id, &values)?;
            let Value::Number(value) = values.get("selected_index")? else {
                return None;
            };
            if i == 0 {
                result.value = *value as i32;
            }
            result.mixed |= *value != f64::from(result.value);
            result.allow_none &= spec.id != "dropdown";
            result.explicit |= self
                .explicit_control_properties(id)
                .contains_key("selected_index");
            result.targets.push((id, labels.len(), *value as i32));
            rows.push(labels);
        }
        for row in 0..rows.iter().map(Vec::len).max().unwrap_or(0) {
            result.labels.push(
                rows.iter()
                    .map(|labels| labels.get(row).map_or("-", String::as_str))
                    .collect::<Vec<_>>()
                    .join(" / "),
            );
        }
        Some(result)
    }

    pub fn set_selection_row(&mut self, index: i32) -> Result<(), String> {
        let selection = self
            .row_selection()
            .ok_or("The selection does not share row choices")?;
        if index < -1
            || (index == -1 && !selection.allow_none)
            || (index >= 0 && index as usize >= selection.labels.len())
        {
            return Err("This row is unavailable for the selection".into());
        }
        let before = self.snapshot();
        for (id, count, current) in selection.targets {
            if current == index || (index >= 0 && index as usize >= count) {
                continue;
            }
            if let Err(error) =
                self.set_control_property(id, "selected_index", Value::Number(f64::from(index)))
            {
                self.restore(before);
                return Err(error);
            }
        }
        Ok(())
    }

    pub fn reset_selection_row(&mut self) -> Result<(), String> {
        let selection = self
            .row_selection()
            .ok_or("The selection does not share row choices")?;
        let before = self.snapshot();
        for (id, _, _) in selection.targets {
            if let Err(error) = self.reset_control_property(id, "selected_index") {
                self.restore(before);
                return Err(error);
            }
        }
        Ok(())
    }
}
