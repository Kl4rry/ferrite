use std::{
    collections::HashMap,
    fmt::{self, Display},
    hash::{Hash, Hasher},
    path::PathBuf,
};

use ferrite_utility::{graphemes::RopeGraphemeExt, line_ending::LineEnding};
use history::History;
use ropey::RopeSlice;

use self::completer::{Completer, CompleterContext};
use super::buffer::error::BufferError;
use crate::{
    cmd::Cmd,
    event_loop_proxy::{EventLoopProxy, UserEvent},
    views::one_line_input_view::OneLineInputState,
};

pub mod cmd_parser;
pub mod completer;
mod history;

#[derive(Debug, Clone)]
#[repr(u8)]
pub enum PaletteMode {
    Command,
    Goto,
    Search,
    Replace,
    GlobalSearch,
    Shell,
    Rename { path: PathBuf },
}

impl Hash for PaletteMode {
    fn hash<H>(&self, state: &mut H)
    where
        H: Hasher,
    {
        let discriminant = std::mem::discriminant(self);
        discriminant.hash(state);
    }
}

impl PartialEq for PaletteMode {
    fn eq(&self, other: &Self) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }
}

impl Eq for PaletteMode {}

#[derive(Debug, Clone)]
pub enum PalettePromptEvent {
    Nop,
    Quit,
    Reload,
    CloseCurrent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectedPrompt {
    Alt1,
    Alt2,
    Neither,
}

pub enum PaletteState {
    Input {
        input_state: Box<OneLineInputState>,
        prompt: String,
        mode: PaletteMode,
        focused: bool,
        completer: Completer,
        history_index: usize,
        old_line: String,
    },
    Prompt {
        selected: SelectedPrompt,
        prompt: String,
        alt1_char: char,
        alt1_event: PalettePromptEvent,
        alt2_char: char,
        alt2_event: PalettePromptEvent,
    },
    Message(String),
    Error(String),
    Nothing,
}

pub struct CommandPalette {
    proxy: Box<dyn EventLoopProxy<UserEvent>>,
    state: PaletteState,
    histories: HashMap<PaletteMode, History>,
}

impl CommandPalette {
    pub fn new(proxy: Box<dyn EventLoopProxy<UserEvent>>) -> Self {
        Self {
            state: PaletteState::Nothing,
            proxy,
            histories: Default::default(),
        }
    }

    pub fn set_msg(&mut self, msg: impl Display) {
        self.state = PaletteState::Message(msg.to_string());
    }

    pub fn set_error(&mut self, msg: impl fmt::Display) {
        let msg = msg.to_string();
        tracing::error!("{}", msg);
        match &mut self.state {
            PaletteState::Error(error) => {
                error.push('\n');
                error.push_str(&msg);
            }
            state => *state = PaletteState::Error(msg),
        }
    }

    pub fn reset(&mut self) {
        self.state = PaletteState::Nothing;
    }

    pub fn focus(&mut self, prompt: impl Into<String>, mode: PaletteMode, ctx: CompleterContext) {
        if let PaletteState::Input {
            mode: input_mode,
            focused,
            ..
        } = &mut self.state
            && *input_mode == mode
        {
            *focused = true;
            return;
        }

        let input_state = OneLineInputState::new();
        self.histories.entry(mode.clone()).or_default();
        self.state = PaletteState::Input {
            prompt: prompt.into(),
            mode,
            focused: true,
            completer: Completer::new(&input_state.buffer, ctx),
            input_state: Box::new(input_state),
            history_index: 0,
            old_line: String::new(),
        };
    }

    pub fn set_line(&mut self, content: impl AsRef<str>) {
        if let PaletteState::Input { input_state, .. } = &mut self.state {
            let view_id = input_state.buffer.get_first_view_or_create();
            input_state.buffer.replace(
                view_id,
                0..input_state.buffer.rope().len_bytes(),
                content.as_ref(),
            );
            input_state.buffer.eof(view_id, false);
        }
    }

    pub fn get_line(&self) -> Option<String> {
        if let PaletteState::Input { input_state, .. } = &self.state {
            return Some(input_state.buffer.rope().to_string());
        }
        None
    }

    pub fn update_prompt(&mut self, new_prompt: impl Into<String>) {
        match &mut self.state {
            PaletteState::Input { prompt, .. } => *prompt = new_prompt.into(),
            PaletteState::Prompt { prompt, .. } => *prompt = new_prompt.into(),
            _ => (),
        }
    }

    pub fn set_prompt(
        &mut self,
        prompt: impl Into<String>,
        (alt1_char, alt1_event): (char, PalettePromptEvent),
        (alt2_char, alt2_event): (char, PalettePromptEvent),
    ) {
        assert!(
            alt1_char.is_ascii_alphabetic()
                && alt2_char.is_ascii_alphabetic()
                && alt1_char != alt2_char
        );
        self.state = PaletteState::Prompt {
            selected: SelectedPrompt::Neither,
            prompt: prompt.into(),
            alt1_char: alt1_char.to_ascii_lowercase(),
            alt1_event,
            alt2_char: alt2_char.to_ascii_lowercase(),
            alt2_event,
        };
    }

    pub fn has_focus(&self) -> bool {
        matches!(
            self.state,
            PaletteState::Input { focused: true, .. } | PaletteState::Prompt { .. }
        )
    }

    pub fn unfocus(&mut self) {
        if let PaletteState::Input { focused, .. } = &mut self.state {
            *focused = false;
        }
    }

    pub fn state(&mut self) -> &mut PaletteState {
        &mut self.state
    }

    pub fn mode(&self) -> Option<&PaletteMode> {
        if let PaletteState::Input { mode, .. } = &self.state {
            Some(mode)
        } else {
            None
        }
    }

    pub fn height(&self) -> usize {
        match &self.state {
            PaletteState::Message(string) => string.lines().count(),
            PaletteState::Error(string) => string.lines().count(),
            PaletteState::Prompt {
                selected,
                prompt,
                alt1_char,
                alt2_char,
                ..
            } => Self::get_prompt(*selected, prompt, *alt1_char, *alt2_char)
                .lines()
                .count(),
            PaletteState::Input {
                mode: PaletteMode::Search | PaletteMode::Replace,
                ..
            } => 2,
            _ => 1,
        }
        .max(1)
    }
}

impl CommandPalette {
    pub fn handle_input(&mut self, input: Cmd) -> Result<(), BufferError> {
        match &mut self.state {
            PaletteState::Input {
                input_state,
                mode,
                completer,
                history_index,
                old_line,
                ..
            } => {
                let view_id = input_state.buffer.get_first_view_or_create();
                let mut enter = false;
                input_state.buffer.mark_clean();
                match input {
                    Cmd::Insert { text } => {
                        let rope = RopeSlice::from(text.as_str());
                        let line = rope.line_without_line_ending(0);
                        input_state.buffer.handle_input(
                            view_id,
                            Cmd::Insert {
                                text: line.to_string(),
                            },
                        )?;
                        if line.len_bytes() != rope.len_bytes() {
                            enter = true;
                        }
                    }
                    Cmd::Char { ch } if LineEnding::from_char(ch).is_some() => {
                        enter = true;
                    }
                    Cmd::Enter => {
                        enter = true;
                    }
                    Cmd::TabOrIndent { back }
                        if *mode == PaletteMode::Command || *mode == PaletteMode::Shell =>
                    {
                        if back {
                            completer.backward(&mut input_state.buffer)
                        } else {
                            completer.forward(&mut input_state.buffer)
                        }
                        if completer.options().len() == 1 {
                            input_state.buffer.mark_dirty();
                        }
                    }
                    Cmd::MoveRight { .. } => {
                        input_state.buffer.handle_input(view_id, input)?;
                        // TODO figure out if this really should be zero
                        if input_state.buffer.cursor_is_eof(view_id, 0) {
                            input_state.buffer.mark_dirty();
                        }
                    }
                    Cmd::MoveUp { .. } => {
                        if let Some(history) = self.histories.get(mode)
                            && history.len() > 0
                        {
                            *history_index += 1;
                            *history_index = (*history_index).min(history.len());
                            let string = history
                                .get(history_index.saturating_sub(1))
                                .unwrap()
                                .to_string();
                            if *history_index == 1 {
                                *old_line = input_state.buffer.rope().to_string();
                            }
                            input_state.buffer.replace(
                                view_id,
                                0..input_state.buffer.rope().len_bytes(),
                                &string,
                            );
                            input_state.buffer.eof(view_id, false);
                        }
                    }
                    Cmd::MoveDown { .. } => {
                        if *history_index <= 1 {
                            input_state.buffer.replace(
                                view_id,
                                0..input_state.buffer.rope().len_bytes(),
                                old_line,
                            );
                            input_state.buffer.eof(view_id, false);
                            old_line.clear();
                        } else if let Some(history) = self.histories.get(mode) {
                            *history_index = history_index.saturating_sub(1);
                            let string = history
                                .get(history_index.saturating_sub(1))
                                .unwrap()
                                .to_string();
                            input_state.buffer.replace(
                                view_id,
                                0..input_state.buffer.rope().len_bytes(),
                                &string,
                            );
                            input_state.buffer.eof(view_id, false);
                        }
                    }
                    input => {
                        input_state.buffer.handle_input(view_id, input)?;
                        // Make sure there is only a single cursor
                        input_state.buffer.views[view_id].cursors.clear();
                    }
                }

                if enter && input_state.buffer.rope().len_bytes() > 0 {
                    let history = self.histories.get_mut(mode).unwrap();
                    history.add(input_state.buffer.rope().to_string());
                    self.proxy.send(UserEvent::PaletteFinished {
                        mode: mode.clone(),
                        content: input_state.buffer.rope().to_string(),
                    });
                } else if input_state.buffer.is_dirty()
                    && (*mode == PaletteMode::Command || *mode == PaletteMode::Shell)
                {
                    completer.update_text(&input_state.buffer);
                } else if input_state.buffer.is_dirty() {
                    self.proxy.send(UserEvent::PalettePreview {
                        mode: mode.clone(),
                        content: input_state.buffer.rope().to_string(),
                    });
                }
            }
            PaletteState::Prompt {
                selected,
                alt1_char,
                alt1_event,
                alt2_char,
                alt2_event,
                ..
            } => {
                let mut chars = Vec::new();
                match input {
                    Cmd::Char { ch } => chars.push(ch),
                    Cmd::Insert { text } => chars.extend(text.chars()),
                    Cmd::Enter => chars.push('\n'),
                    _ => (),
                }
                for ch in chars {
                    let ch = ch.to_ascii_lowercase();
                    if ch == *alt1_char {
                        *selected = SelectedPrompt::Alt1;
                    }

                    if ch == *alt2_char {
                        *selected = SelectedPrompt::Alt2;
                    }

                    if LineEnding::from_char(ch).is_some() {
                        match selected {
                            SelectedPrompt::Alt1 => {
                                self.proxy.send(UserEvent::PromptEvent(alt1_event.clone()));
                                self.reset();
                                break;
                            }
                            SelectedPrompt::Alt2 => {
                                self.proxy.send(UserEvent::PromptEvent(alt2_event.clone()));
                                self.reset();
                                break;
                            }
                            SelectedPrompt::Neither => (),
                        }
                    }
                }
            }
            _ => (),
        }

        Ok(())
    }

    pub fn get_prompt(
        selected: SelectedPrompt,
        prompt: &str,
        alt1_char: char,
        alt2_char: char,
    ) -> String {
        let alt1 = if selected == SelectedPrompt::Alt1 {
            alt1_char.to_ascii_uppercase()
        } else {
            alt1_char
        };

        let alt2 = if selected == SelectedPrompt::Alt2 {
            alt2_char.to_ascii_uppercase()
        } else {
            alt2_char
        };

        format!("{prompt}: {alt1} / {alt2}")
    }
}
