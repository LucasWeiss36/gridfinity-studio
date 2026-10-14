use eframe::egui;

const SIZE: usize = 128;
const RGBA: &[u8; SIZE * SIZE * 4] = include_bytes!("../assets/branding/mark.rgba");

pub fn window_icon() -> egui::IconData {
    egui::IconData {
        rgba: RGBA.to_vec(),
        width: SIZE as u32,
        height: SIZE as u32,
    }
}

pub fn logo(ui: &mut egui::Ui, size: f32) {
    let id = egui::Id::new("gridfinitystudio-brand-logo");
    let texture = ui
        .ctx()
        .data_mut(|data| data.get_temp::<egui::TextureHandle>(id))
        .unwrap_or_else(|| {
            let texture = ui.ctx().load_texture(
                "GridfinityStudio logo",
                egui::ColorImage::from_rgba_unmultiplied([SIZE, SIZE], RGBA),
                egui::TextureOptions::LINEAR,
            );
            ui.ctx()
                .data_mut(|data| data.insert_temp(id, texture.clone()));
            texture
        });
    ui.add(egui::Image::new((texture.id(), egui::vec2(size, size))));
}
