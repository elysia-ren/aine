// 最小 egui 渲染测试
use eframe::egui;

struct Hello {
    count: i32,
}

impl eframe::App for Hello {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("HELLO EGUI RENDER TEST");
            if ui.button("click me").clicked() {
                self.count += 1;
            }
            ui.label(format!("count = {}", self.count));
        });
    }
}

fn main() -> eframe::Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default().with_inner_size([400.0, 300.0]),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };
    eframe::run_native(
        "egui-min",
        options,
        Box::new(|_cc| Box::new(Hello { count: 0 })),
    )
}
