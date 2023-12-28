use super::screens_manager::ScreensManager;
use eframe::egui::{self, ScrollArea};
extern crate image;
use super::super::itc::{Delay, ScreenshotDim};
use eframe::egui::ColorImage;
use std::sync::Arc;

///Stato della parte di interfaccia con la funzione di selezionare la modalità di cattura e avviarla.
pub struct CaptureMode {
    area: ScreenshotDim,
    delay: Delay,
    screens_mgr: Arc<ScreensManager>,
}
impl CaptureMode {
    pub fn new(screens_mgr: Arc<ScreensManager>) -> Self {
        Self {
            area: ScreenshotDim::Fullscreen,
            delay: Delay {
                delayed: false,
                scalar: 0.0,
            },
            screens_mgr,
        }
    }

    ///Ritorna Some(ScreenshotDim, f64) se l'utente ha premuto il bottone "Acquire"
    /// - ScreenshotDim è la modalità di selezione dell'area coinvolta nello screenshot;<br>
    /// - f64 sono i secondi di delay impostati.<br>
    /// Non è necessario che il metodo ritorni anche indicazione sullo schermo selezionato,
    /// perchè l'informazione viene già memorizzata dentro alla variabile di tipo Arc<ScreensManager>.
    pub fn update(
        &mut self,
        ui: &mut egui::Ui,
        ctx: &egui::Context,
        _frame: &mut eframe::Frame,
    ) -> Option<(ScreenshotDim, f64)> {
        let mut ret = None;

        ScrollArea::new([true, false]).show(ui, |ui| {
            ui.label("Capture Mode");
            ui.separator();
            egui::ComboBox::from_label("Area") //menù a tendina per scegliere se fare uno screen di tutto, oppure per selezionare un rettangolo
                .selected_text(format!("{:?}", self.area))
                .show_ui(ui, |ui| {
                    ui.style_mut().wrap = Some(false);
                    ui.set_min_width(60.0);
                    ui.selectable_value(&mut self.area, ScreenshotDim::Fullscreen, "Full Screen");
                    ui.selectable_value(&mut self.area, ScreenshotDim::Rectangle, "Rectangle");
                })
                .response
                .on_hover_text("Choose the desired area");

            ui.separator();

            self.screens_combobox(ui, self.screens_mgr.clone(), ctx);

            ui.separator();

            //checkbox con spinner per attivare e impostare delay
            ui.add(egui::Checkbox::new(&mut self.delay.delayed, "Timer"))
                .on_hover_text("To take a delayed screenshot");
            if self.delay.delayed {
                ui.add(egui::Slider::new(&mut self.delay.scalar, 0.0..=5.0));
            }

            ui.separator();
            // gestione della pressione del pulsante "Acquire": la funzione ritorna Some(..) al posto di None
            if ui
                .button("Acquire")
                .on_hover_text(
                    "After acquisition, the image is automatically copied to the clipboard",
                )
                .clicked()
            {
                ret = Some((self.area.clone(), self.delay.scalar));
            }
        });
        ret
    }

    /// Combobox che mostra l'elenco di screen messo a disposizione dallo screen manager.<br>
    /// Si itera su ogni schermo, ottenendo le info da visualizzare ed eseguendo try_lock()
    /// sul mutex che contiene l'icona dello screen.<br>
    /// Se l'icona è ancora in caricamento (la rispettiva Option contiene None), oppure try_lock()
    /// fallisce, allora viene mostrato uno spinner al posto dell'icona nella corrispondente
    /// entry della combobox.<br>
    /// Una selezione su questa combobox scatena la modifica dello screen che lo screen manager
    /// etichetta come "selected".<br>
    /// Esiste un bottone per chiedere il refresh dell'intera lista di screen allo screen manager.
    fn screens_combobox(
        &self,
        ui: &mut egui::Ui,
        screens_manager: Arc<ScreensManager>,
        ctx: &egui::Context,
    ) {
        ui.horizontal(|ui| {
            egui::ComboBox::from_label("Screen") //prova di menù a tendina per scegliere se fare uno screen di tutto, oppure per selezionare un rettangolo
                .selected_text(format!(
                    "{:?}",
                    screens_manager.get_current_screen_index() + 1
                ))
                .show_ui(ui, |ui| {
                    ui.style_mut().wrap = Some(false);
                    ui.set_min_width(60.0);
                    for (i, s) in screens_manager.get_screens().iter().enumerate() {
                        let di = s.0.display_info;
                        let str = format!("{} ({}x{})", i + 1, di.width, di.height);

                        ui.horizontal(|ui| {
                            if let Ok(guard) = s.1.try_lock() {
                                if let Some(rgba) = guard.clone() {
                                    let txt = ctx.load_texture(
                                        "icon",
                                        ColorImage::from_rgba_unmultiplied(
                                            [rgba.width() as usize, rgba.height() as usize],
                                            rgba.as_raw(),
                                        ),
                                        Default::default(),
                                    );
                                    ui.image(txt.id(), txt.size_vec2());
                                } else {
                                    ui.spinner();
                                }
                            } else {
                                ui.spinner();
                            }

                            let mut curr = screens_manager.get_current_screen_index();
                            ui.selectable_value(&mut curr, i, &str);
                            screens_manager.select_screen(curr);
                        });
                    }
                });

            if ui.button("↺").on_hover_text("Refresh").clicked() {
                screens_manager.update_available_screens();
            }
        });
    }
}
