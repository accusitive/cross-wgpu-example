use epi::App;

pub struct TropicGui {
    pub wireframe: bool,

}
impl App for TropicGui {
    fn update(&mut self, ctx: &egui::Context, frame: &epi::Frame) {
        egui::Window::new("Main control").show(ctx, |ui| {
            ui.separator();
            ui.label("Tropic control");
            #[cfg(not(target_arch = "wasm32"))]
            ui.checkbox(&mut self.wireframe, "Wireframe Rendering");


        });
        
        
    }

    fn name(&self) -> &str {
        "Tropic"
    }
}