use std::cell::RefCell;

use egui::Slider;
use epi::App;

pub struct TropicGui {
    pub wireframe: bool,
    pub camera_speed: f32 

}
impl App for TropicGui {
    fn update(&mut self, ctx: &egui::Context, frame: &epi::Frame) {
        egui::Window::new("Main control").show(ctx, |ui| {
            ui.separator();
            ui.label("Tropic control");
            #[cfg(not(target_arch = "wasm32"))]
            ui.checkbox(&mut self.wireframe, "Wireframe Rendering");
            // ui.slider
            ui.add(Slider::new(&mut self.camera_speed, 0.0f32..=5.0f32));

        });
        
        
    }

    fn name(&self) -> &str {
        "Tropic"
    }
}