// SPDX-License-Identifier: GPL-3.0-only
// Copyright 2026 Wireframe contributors
//! Read JSON lines {"text":"...","bullets":false} and print the parsed text model.
use std::io::{self, BufRead};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    for line in io::stdin().lock().lines() {
        let input: serde_json::Value = serde_json::from_str(&line?)?;
        let text = input["text"].as_str().ok_or("text must be a string")?;
        let parsed = wireframe::rich_text::RichText::parse(
            text,
            input["bullets"].as_bool().unwrap_or(false),
        );
        println!("{}", serde_json::to_string(&parsed)?);
    }
    Ok(())
}
