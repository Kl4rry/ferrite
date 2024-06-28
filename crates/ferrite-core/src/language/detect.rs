use ferrite_utility::graphemes::RopeGraphemeExt;
use ropey::{Rope, RopeSlice};

pub fn detect_language(inital_guess: Option<&str>, content: Rope) -> Option<&'static str> {
    tracing::info!("inital_guess: {inital_guess:?}");
    if inital_guess == Some("c") {
        let cpp_markers = [
            "public",
            "protected",
            "private",
            "std::",
            "dynamic_cast",
            "static_cast",
            "reinterpret_cast",
            "#include <iostream>",
            "#include <vector>",
            "#include <string>",
            "class",
            "throw",
            "catch",
            "try",
            "nullptr",
            "const&",
            "final",
            "using",
        ];
        if detect_markers(content.slice(..), &cpp_markers) > 3 {
            return Some("cpp");
        }
    }

    detect_shebang(content.slice(..))
}

fn detect_shebang(content: RopeSlice) -> Option<&'static str> {
    for line in content.lines() {
        if line.is_whitespace() {
            continue;
        }
    }

    let first_non_empty = content.lines().find(|line| !line.is_whitespace())?;
    let first_line = first_non_empty
        .slice(..first_non_empty.len_chars().min(1000))
        .to_string();

    let shebangs = [
        ("python3", "python"),
        ("python2", "python"),
        ("python", "python"),
        ("#!/bin/bash", "bash"),
        ("#!/usr/bin/bash", "bash"),
        ("#!/bin/sh", "bash"),
        ("#!/usr/bin/env bash", "bash"),
        ("zsh", "bash"),
        ("<?xml", "xml"),
        ("<!DOCTYPE html>", "html"),
        ("diff", "diff"),
    ];

    for (shebang, language) in shebangs {
        if first_line.contains(shebang) {
            return Some(language);
        }
    }

    None
}

fn detect_markers(content: RopeSlice, markers: &[&str]) -> usize {
    let start = content.slice(..content.len_chars().min(1000)).to_string();
    let mut count = 0;
    for marker in markers {
        count += start.contains(marker) as usize;
    }
    count
}
