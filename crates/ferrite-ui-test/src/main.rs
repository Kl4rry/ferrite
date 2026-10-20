use ferrite_imtui::*;
use ferrite_linalg::*;

struct TestAppState {}

impl ImTuiApp for TestAppState {
    type UserEvent = ();
    fn start_of_frame(&mut self, _runtime: &mut Runtime) {}
    fn ui(&mut self, _runtime: &mut Runtime, ui: &mut Ui) {
        ui.push_layer("base layer", ui.rect, Rounding::Ceil);

        let fg = Color::new(1.0, 1.0, 1.0);
        let bg = Color::new(0.2, 0.2, 0.2);
        widgets::TextButton::new("hello".into(), Rect::new(0, 0, 10, 1))
            .style(Style::default().fg(fg).bg(bg))
            .ui(ui, &mut ());

        ui.pop_layer();
    }
    fn event(&mut self, _event: InputEvent<Self::UserEvent>) {}
}

fn main() {
    let (event_loop, _proxy) = ferrite_winit_wgpu_platform::create_event_loop::<()>();
    let test_app_state = TestAppState {};
    let platform = ferrite_winit_wgpu_platform::WinitWgpuPlatform::default();
    platform.run(event_loop, test_app_state);
}
