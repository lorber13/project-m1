use crate::gui::edit_image::utils::obscure_screen;
use eframe::egui;
use eframe::egui::{Context, CursorIcon, Stroke, TextureHandle, TextureOptions};
use egui::{pos2, Color32, ColorImage, Pos2, Rect, Sense, Vec2};
use image::RgbaImage;

/// Struct che memorizza lo stato del componente della gui che mette a disposizione un'interfaccia per limitare lo screenshot
/// ad un'area rettangolare attraverso operazione di drag & drop.<br>
/// Di fatto, l'operazione corrisponde al ritaglio di uno screenshot precedentemente acquisito. Questo screenshot viene acquisito
/// nel momento della pressione sul bottone "Acquire", successivamente viene messo come
/// sfondo del frame mostrato a dimensione fullscreen.<br>
/// Per questo motivo, nella struct la stessa immagine è memorizzata in due forme diverse:
/// - La <i>TextureHandle</i> viene usata per mostrare tale immagine come sfondo;
/// - La <i>RgbaImage</i> sarà ritagliata per produrre l'output.<br>
///
/// La struct memorizza inoltre, al suo interno, se e da quale punto è stata avviata un'operazione di drag.
pub struct RectSelection {
    texture_handle: TextureHandle,
    start_drag_point: Option<Pos2>,
    rgba: RgbaImage,
}

impl RectSelection {
    /// Esegue <i>Context::load_texture()</i> per poter impostare l'immagine come sfondo.
    pub fn new(rgba: RgbaImage, ctx: &Context) -> Self {
        RectSelection {
            texture_handle: ctx.load_texture(
                "screenshot_image",
                ColorImage::from_rgba_unmultiplied(
                    [rgba.width() as usize, rgba.height() as usize],
                    rgba.as_raw(),
                ),
                TextureOptions::default(),
            ),
            rgba,
            start_drag_point: None,
        }
    }

    /// Mostra una finestra full screen e senza barra di controllo, all'interno della quale viene allocato un oggetto painter
    /// sensibile alle operazioni di click e drag.
    /// Lo sfondo di tale componente è lo screenshot fullscreen passato al costruttore di questa istanza, oscurato con un filtro.
    ///
    /// Se viene rilevato click, il punto in cui esso è avvenuto viene memorizzato in <i>self.start_drag_point</i>.<br>
    /// Fino a quando il drag è in corso, utilizzando il painter viene disegnato un rettangolo a partire dai due seguenti vertici:
    /// - <i>self.start_drag_point</i>;
    /// - <i>response.on_hover_pos()</i> (dove <i>response</i> è l'oggetto <i>Response</i> ritornato in seguito all'allocazione del painter).
    ///
    /// Quando viene rilasciato il drag, il rettangolo correntemente disegnato viene salvato per poter essere ritornato dal metodo.<br>
    /// Prima del rilascio del drag, il metodo ritorna <i>None</i>.
    pub fn update(&mut self, ctx: &Context) -> Option<(Rect, RgbaImage)> {
        let mut ret = None;

        egui::Area::new("").show(ctx, |ui| {
            let (response, painter) = ui.allocate_painter(
                Vec2::new(ctx.screen_rect().width(), ctx.screen_rect().height()),
                Sense::click_and_drag(),
            );
            painter.image(
                self.texture_handle.id(),
                painter.clip_rect(),
                Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
                Color32::WHITE,
            );

            ctx.set_cursor_icon(CursorIcon::Crosshair);

            let mut rect_not_to_be_obscured = Rect::from_min_size(Pos2::ZERO, Vec2::ZERO);
            if !response.clicked() {
                if response.drag_started() {
                    self.start_drag_point = response.hover_pos();
                } else if response.dragged() {
                    if let (Some(click_pos), Some(hover_pos)) =
                        (self.start_drag_point, ctx.pointer_hover_pos())
                    {
                        rect_not_to_be_obscured = Rect::from_points(&[click_pos, hover_pos]);
                    }
                } else if response.drag_released() {
                    if let (Some(click_pos), Some(hover_pos)) =
                        (self.start_drag_point, ctx.pointer_hover_pos())
                    {
                        ret = Some((
                            Rect::from_points(&[
                                pos2(
                                    click_pos.x * ctx.pixels_per_point(),
                                    click_pos.y * ctx.pixels_per_point(),
                                ),
                                pos2(
                                    hover_pos.x * ctx.pixels_per_point(),
                                    hover_pos.y * ctx.pixels_per_point(),
                                ),
                            ]),
                            self.rgba.clone(),
                        ));
                    }
                }
            }
            obscure_screen(
                &painter,
                rect_not_to_be_obscured,
                Stroke::new(3.0, Color32::WHITE),
            );
        });
        ret
    }
}
