// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Editable compact form gallery and controlled geometry diagnostics.
use std::{fs, path::PathBuf};
use wireframe::{
    document::{Dimension, Link, NodeContent, Value},
    layout::TextMetrics,
    model::Kind,
    small_forms as forms,
    workspace::Editor,
};
#[derive(serde::Deserialize)]
struct Case {
    family: String,
    width: f32,
    height: f32,
    sketch: bool,
    #[serde(default)]
    metric: [f32; 2],
    #[serde(default)]
    disabled: bool,
    #[serde(default)]
    point: [f32; 2],
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).is_some_and(|s| s == "--geometry") {
        let cases: Vec<Case> = serde_json::from_slice(&fs::read(&args[2])?)?;
        let out:Vec<_>=cases.into_iter().map(|c|match c.family.as_str(){
   "numericstepper"=>{let(w,h)=forms::stepper_size(TextMetrics{width:c.metric[0],height:c.metric[1],rows:1},(c.width!=-1.).then_some(c.width));serde_json::json!({"size":[w,h],"text":forms::stepper_text(w,h),"button":forms::stepper_button(w,h),"colors":forms::stepper_colors(c.sketch,c.disabled,[51,102,153])})},
   "colorpicker"=>serde_json::json!({"size":forms::fixed_size("color_picker"),"swatch":forms::picker_swatch(c.sketch)}),
   "helpbutton"=>serde_json::json!({"size":forms::fixed_size("help_button")}),
   "point"=>serde_json::json!(forms::stepper_point(c.point,c.width,c.height,c.sketch)),
   _=>panic!("Unknown diagnostic family")}).collect();
        println!("{}", serde_json::to_string(&out)?);
        return Ok(());
    }
    let dir = PathBuf::from(args.get(1).cloned().unwrap_or("small-forms-gallery".into()));
    fs::create_dir_all(&dir)?;
    let mut e = Editor::default();
    e.document.title = "Compact forms".into();
    e.document.pages[0].name = "Form controls".into();
    for (col, kind) in [Kind::NumericStepper, Kind::ColorPicker, Kind::HelpButton]
        .into_iter()
        .enumerate()
    {
        for row in 0..6 {
            e.add(kind, 20. + 320. * col as f32, 20. + 110. * row as f32);
            let id = *e.selection.iter().next().unwrap();
            let mut size = None;
            if kind == Kind::NumericStepper {
                let (text, font, disabled, width) = match row {
                    0 => (None, None, false, None),
                    1 => (Some("-42.5"), None, true, None),
                    2 => (
                        Some("*Bold* _Italic_ [Link](https://example.com)"),
                        None,
                        false,
                        Some(220.),
                    ),
                    3 => (Some(""), Some(24.), false, Some(10.)),
                    4 => (Some("One\nTwo {mockup-name}"), Some(20.), false, Some(200.)),
                    _ => (Some("999"), Some(40.), true, Some(1.)),
                };
                if let Some(t) = text {
                    e.set_control_property(id, "text", Value::Text(t.into()))?;
                }
                if let Some(f) = font {
                    e.set_control_property(id, "font_size", Value::Number(f))?;
                }
                if disabled {
                    e.set_control_property(id, "state", Value::Choice("disabled".into()))?;
                }
                if row == 2 || row == 5 {
                    e.set_control_property(id, "border_color", Value::Color([255, 0, 0, 255]))?;
                }
                size = width.map(|w| (w, if row == 3 { Some(100.) } else { None }));
            } else if kind == Kind::ColorPicker {
                if row > 0 {
                    e.set_control_property(
                        id,
                        "fill",
                        Value::Color(
                            [
                                [255, 0, 0, 255],
                                [0, 0, 0, 255],
                                [255; 4],
                                [0, 255, 0, 255],
                                [255, 255, 0, 255],
                            ][row - 1],
                        ),
                    )?;
                }
            } else if row == 1 {
                e.set_control_property(
                    id,
                    "link",
                    Value::Link(Link::Url("https://example.com".into())),
                )?;
            }
            if col > 0 {
                size = match row {
                    2 => Some((100., Some(100.))),
                    3 => Some((1., Some(1.))),
                    _ => None,
                };
            }
            if let Some((w, h)) = size
                && let NodeContent::Control { size, .. } = &mut e.document.pages[0]
                    .content
                    .nodes
                    .last_mut()
                    .unwrap()
                    .content
            {
                size.width = Dimension::Fixed(w);
                if let Some(h) = h {
                    size.height = Dimension::Fixed(h);
                }
            }
        }
    }
    e.selection.clear();
    e.document.settings.sketch = false;
    e.document.settings.font_size = 13.;
    fs::write(dir.join("Compact-forms.wireframe"), e.document.encode()?)?;
    for (style, sketch) in [("clean", false), ("sketch", true)] {
        for font in [13., 24.] {
            e.document.settings.sketch = sketch;
            e.document.settings.font_size = font;
            let scene = e.rendered_scene(e.page)?;
            fs::write(dir.join(format!("{style}-{font}.png")), scene.png()?)?;
            fs::write(dir.join(format!("{style}-{font}.svg")), scene.svg())?;
        }
    }
    Ok(())
}
