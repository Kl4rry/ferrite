use ferrite_runtime::{Runtime, any_view::AnyView, input::event::InputEvent};

use crate::{
    cmd::Cmd,
    engine::Engine,
    event_loop_proxy::{EventLoopControlFlow, UserEvent},
    keymap,
    views::{
        chord_view::ChordView,
        container::Container,
        lens::Lens,
        main_view::MainView,
        palette_view::PaletteView,
        pane_view::PaneView,
        picker_view::{PickerView, TextAlign},
        zstack::ZStack,
    },
};

#[profiling::function]
pub fn update(runtime: &mut Runtime<Engine>, control_flow: &mut EventLoopControlFlow) {
    runtime.state.do_polling(control_flow);
    runtime.scale = runtime.state.scale;
    runtime.font_family = runtime.state.config.editor.gui.font_family.clone();
    runtime.font_weight = runtime.state.config.editor.gui.font_weight as u16;
    runtime.state.last_render_time = runtime.last_render_time;
}

#[profiling::function]
pub fn input(
    engine: &mut Engine,
    input: InputEvent<UserEvent>,
    control_flow: &mut EventLoopControlFlow,
) {
    let cmd = match input {
        InputEvent::Key(key, modifiers) => keymap::get_command_from_input(
            key,
            modifiers,
            engine.get_current_keymappings(),
            engine.get_input_ctx(),
        ),
        InputEvent::Text(text) => Some(Cmd::Insert { text }),
        InputEvent::Paste(text) => Some(Cmd::Insert { text }),
        InputEvent::Scroll(_x, y) => {
            engine.handle_single_input_command(
                Cmd::VerticalScroll {
                    distance: -y as f64 * 3.0,
                },
                &mut EventLoopControlFlow::Poll,
            );
            None
        }
        InputEvent::UserEvent(event) => {
            engine.handle_app_event(event, control_flow);
            return;
        }
    };
    if let Some(cmd) = cmd {
        engine.handle_input_command(cmd, control_flow);
    }
}

#[profiling::function]
pub fn layout(engine: &mut Engine) -> AnyView<Engine> {
    profiling::scope!("layout");
    let theme = engine.themes[&engine.config.editor.theme].clone();
    let config = engine.config.editor.clone();

    let mut stack = Vec::new();

    stack.push(AnyView::new(MainView::new(
        PaneView::new(engine),
        PaletteView::new(theme.clone(), config.clone(), engine.palette.has_focus()),
    )));
    if engine.chord.is_some() {
        stack.push(AnyView::new(ChordView::new(theme.clone())));
    }

    let m_x = 4;
    let m_y = 2;
    if engine.file_picker.is_some() {
        profiling::scope!("render tui file picker");
        let p = Lens::new(
            PickerView::new(theme.clone(), config.clone(), "Open file"),
            |engine: &mut Engine| engine.file_picker.as_mut().unwrap(),
        );
        stack.push(AnyView::new(
            Container::new(p).margin(m_x, m_y).grid_alinged(true),
        ));
    } else if engine.buffer_picker.is_some() {
        profiling::scope!("render tui buffer picker");
        let p = Lens::new(
            PickerView::new(theme.clone(), config.clone(), "Open buffer"),
            |engine: &mut Engine| engine.buffer_picker.as_mut().unwrap(),
        );
        stack.push(AnyView::new(
            Container::new(p).margin(m_x, m_y).grid_alinged(true),
        ));
    } else if engine.global_search_picker.is_some() {
        profiling::scope!("render tui search picker");
        let p = Lens::new(
            PickerView::new(theme.clone(), config.clone(), "Matches")
                .set_text_align(TextAlign::Left),
            |engine: &mut Engine| engine.global_search_picker.as_mut().unwrap(),
        );
        stack.push(AnyView::new(
            Container::new(p).margin(m_x, m_y).grid_alinged(true),
        ));
    };

    AnyView::new(ZStack::new(stack))
}
