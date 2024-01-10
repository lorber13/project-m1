pub mod utils;

use crate::gui::edit_image::utils::{
    handle_arrow_usage, handle_circle_usage, handle_cut, handle_pen_usage, handle_rect_usage,
};
use crate::gui::loading::show_loading;
use crate::image_coding::ImageFormat;
use eframe::egui::color_picker::Alpha;
use eframe::egui::ComboBox;
use eframe::egui::{
    color_picker, pos2, vec2, Align, CentralPanel, Color32, ColorImage, Context, Key, Layout,
    Painter, Pos2, Rect, Response, Rounding, Sense, Shape, Stroke, TextureHandle, TextureOptions,
    Ui,
};
use image::imageops::crop_imm;
use image::RgbaImage;
use imageproc::drawing::Blend;
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::thread;
use utils::{
    obscure_screen, scale_annotation, scaled_rect, stroke_ui_opaque, write_annotation_to_image,
    Direction,
};

/// indica se e' stato premuto uno dei pulsanti Save o Abort.
/// Lo stato `Nil` indica che non e' stato premuto nessuno dei due pulsanti
/// Lo stato Aborted indica che e' stato premuto il pulsante Abort
/// Lo stato Saved indica che e' stato premuto il pulsante Save. In questo caso, verra' ritornata l'immagine da salvare
/// (`RgbaImage`), e il suo formato (`ImageFormat`)
pub enum FrameEvent {
    Saved {
        image: RgbaImage,
        format: ImageFormat,
    },
    Aborted,
    Nil,
}

/// Rappresenta il tool attualmente in uso. Per ogni tool ci sono dei campi che servono ad indicare lo stato della
/// forma che si sta disegnando in un certo istante. Per esempio, se siamo nello stato Rect, ci saranno due valori che
/// indicano la posizione di partenza e di arrivo del trascinamento del cursore (questi due valori determinano
/// univocamente un rettangolo sullo schermo).
#[derive(PartialEq, Debug)]
enum Tool {
    Pen {
        line: Vec<Pos2>,
    },
    Circle {
        start_drag: Option<Pos2>,
        end_drag: Option<Pos2>,
    },
    Rect {
        start_drag: Option<Pos2>,
        end_drag: Option<Pos2>,
    },
    Arrow {
        start_drag: Option<Pos2>,
        end_drag: Option<Pos2>,
    },
    Cut {
        modifying: ModificationOfRectangle,
    },
    /* todo:
       text, very difficult
       rubber, not mandatory but recommended
    */
}

/// rappresenta lo stato interno al Tool di ritaglio. Se siamo in ritaglio, per ogni frame,
/// il rettangolo di ritaglio puo' essere mosso, ridimensionato, oppure puo' non essere modificato. Se sta venendo
/// ridimensionato, viene anche indicata una direzione.
#[derive(PartialEq, Debug)]
pub enum ModificationOfRectangle {
    Move,
    Resize { direction: Direction },
    NoModification,
}

/// Rappresenta la schermata di modifica dello screenshot acquisito
pub struct EditImage {
    current_tool: Tool,
    cut_rect: Rect,
    stroke: Stroke,
    fill_shape: bool,
    image: RgbaImage,
    format: ImageFormat,
    texture_handle: TextureHandle,
    annotations: Vec<Shape>,
    scale_ratio: f32,
    receive_thread: Receiver<RgbaImage>,
}

impl EditImage {
    /// crea una nuova istanza della schermata di modifica dello screenshot. Lo screenshot acquisito viene passato come
    /// parametro.
    pub fn new(rgba: RgbaImage, ctx: &Context) -> EditImage {
        let texture_handle = ctx.load_texture(
            "screenshot_image",
            ColorImage::from_rgba_unmultiplied(
                [rgba.width() as usize, rgba.height() as usize],
                rgba.as_raw(),
            ),
            TextureOptions::default(),
        );
        let (_, rx) = channel();
        EditImage {
            cut_rect: Rect::from_min_size(pos2(0.0, 0.0), texture_handle.size_vec2()),
            current_tool: Tool::Pen { line: Vec::new() },
            texture_handle,
            image: rgba,
            format: ImageFormat::Png,
            annotations: Vec::new(),
            scale_ratio: Default::default(),
            stroke: Stroke {
                width: 1.0,
                color: Color32::GREEN,
            },
            fill_shape: false,
            receive_thread: rx,
        }
    }

    /// crea un oggetto Painter, scalato in base alle dimensioni della finestra al frame corrente
    fn allocate_scaled_painter(&mut self, ui: &mut Ui) -> (Response, Painter) {
        self.update_scale_ratio(ui);
        let scaled_dimensions = vec2(
            self.texture_handle.size()[0] as f32 * self.scale_ratio,
            self.texture_handle.size()[1] as f32 * self.scale_ratio,
        );
        ui.allocate_painter(scaled_dimensions, Sense::click_and_drag())
    }

    /// visualizza le varie annotazioni dell'utente sull'immagine. In particolare, disegna sulla finestra le
    /// annotazioni precedenti (gia' salvate), l'annotazione in corso (quella che sta venendo disegnata al frame
    /// corrente), e infine la regione di ritaglio.
    fn display_annotations(&mut self, painter: &Painter) {
        painter.image(
            self.texture_handle.id(),
            painter.clip_rect(),
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        self.draw_previous_annotations(painter);
        self.draw_current_annotation(painter);
        self.draw_cutting_region(painter);
    }

    /// disegna la regione di ritaglio. Puo' essere bianca o gialla a seconda che stia venendo modificata o no.
    fn draw_cutting_region(&mut self, painter: &Painter) {
        if let Tool::Cut { .. } = self.current_tool {
            obscure_screen(
                painter,
                scaled_rect(
                    painter.clip_rect().left_top(),
                    self.scale_ratio,
                    self.cut_rect,
                ),
                Stroke::new(3.0, Color32::RED),
            );
        } else {
            obscure_screen(
                painter,
                scaled_rect(
                    painter.clip_rect().left_top(),
                    self.scale_ratio,
                    self.cut_rect,
                ),
                Stroke::new(1.0, Color32::WHITE),
            );
        }
    }

    /// disegna l'annotazione corrente (rettangolo, cerchio, linea, freccia)
    fn draw_current_annotation(&mut self, painter: &Painter) {
        match &self.current_tool {
            Tool::Pen { line } => {
                painter.add(Shape::line(line.clone(), self.stroke));
            }
            Tool::Circle {
                start_drag,
                end_drag,
            } => {
                if let (Some(start), Some(end)) = (start_drag, end_drag) {
                    let radius = start.distance(*end);
                    if self.fill_shape {
                        painter.circle_filled(*start, radius, self.stroke.color);
                    } else {
                        painter.circle_stroke(*start, radius, self.stroke);
                    }
                }
            }
            Tool::Rect {
                start_drag,
                end_drag,
            } => {
                if let (Some(a), Some(b)) = (start_drag, end_drag) {
                    if self.fill_shape {
                        painter.rect_filled(
                            Rect::from_two_pos(*a, *b),
                            Rounding::none(),
                            self.stroke.color,
                        );
                    } else {
                        painter.rect_stroke(
                            Rect::from_two_pos(*a, *b),
                            Rounding::none(),
                            self.stroke,
                        );
                    }
                }
            }
            Tool::Arrow {
                start_drag,
                end_drag,
            } => {
                if let (Some(start), Some(end)) = (start_drag, end_drag) {
                    painter.arrow(*start, *end - *start, self.stroke);
                }
            }
            Tool::Cut { .. } => {}
        }
    }

    /// disegna le annotazioni precedenti (tutte quelle che non stanno venendo disegnate al frame corrente)
    fn draw_previous_annotations(&mut self, painter: &Painter) {
        let mut annotations = self.annotations.clone();
        for annotation in &mut annotations {
            scale_annotation(annotation, self.scale_ratio, painter.clip_rect().left_top());
        }
        painter.extend(annotations);
    }

    /// ad ogni frame ricalcola il fattore di scalatura dell'immagine (ad ogni frame la dimensione della finestra puo'
    /// variare)
    fn update_scale_ratio(&mut self, ui: &mut Ui) {
        let available_size = ui.available_size_before_wrap();
        let image_size = self.texture_handle.size_vec2();
        self.scale_ratio = {
            let mut ratio = if image_size.x / available_size.x > image_size.y / available_size.y {
                available_size.x / image_size.x
            } else {
                available_size.y / image_size.y
            };
            if ratio > 1.0 {
                ratio = 1.0;
            }
            ratio
        };
    }

    /// questa e' la funzione di ingresso. Ad ogni frame viene chiamata questa funzione che determina che cosa va
    /// disegnato sulla finestra
    pub fn update(
        &mut self,
        ctx: &Context,
        _frame: &mut eframe::Frame,
        enabled: bool,
    ) -> FrameEvent {
        CentralPanel::default()
            .show(ctx, |ui| match self.receive_thread.try_recv() {
                Ok(image) => FrameEvent::Saved {
                    image,
                    format: self.format,
                },
                Err(error) => match error {
                    TryRecvError::Empty => {
                        show_loading(ctx);
                        FrameEvent::Nil
                    }
                    TryRecvError::Disconnected => {
                        ui.add_enabled_ui(enabled, |ui| {
                            let ret = self.draw_menu_buttons(ui);
                            ui.separator();
                            let (response, painter) = self.allocate_scaled_painter(ui);
                            self.handle_events(ctx, &response, painter.clip_rect());
                            self.display_annotations(&painter);
                            ret
                        })
                        .inner
                    }
                },
            })
            .inner
    }

    /// gestisce lo stato dell'applicazione sulla base degli eventi che accadono al frame corrente. In base al tool in
    /// uso, viene aggiornato lo stato dell'annotazione che sta venendo disegnata. Se si tratta per esempio di una
    /// linea, viene allungata aggiungendo la posizione del cursore al frame corrente.
    fn handle_events(&mut self, ctx: &Context, response: &Response, painter_rect: Rect) {
        self.handle_ctrl_z(ctx);
        match &mut self.current_tool {
            Tool::Pen { line } => {
                handle_pen_usage(
                    &mut self.annotations,
                    self.stroke,
                    self.scale_ratio,
                    ctx,
                    response,
                    painter_rect.left_top(),
                    line,
                );
            }
            Tool::Circle {
                start_drag,
                end_drag,
            } => {
                handle_circle_usage(
                    &mut self.annotations,
                    ctx,
                    response,
                    painter_rect.left_top(),
                    self.fill_shape,
                    self.scale_ratio,
                    self.stroke,
                    start_drag,
                    end_drag,
                );
            }
            Tool::Rect {
                start_drag,
                end_drag,
            } => {
                handle_rect_usage(
                    &mut self.annotations,
                    ctx,
                    response,
                    painter_rect.left_top(),
                    self.stroke,
                    self.scale_ratio,
                    self.fill_shape,
                    start_drag,
                    end_drag,
                );
            }
            Tool::Arrow {
                start_drag,
                end_drag,
            } => {
                handle_arrow_usage(
                    &mut self.annotations,
                    ctx,
                    response,
                    painter_rect.left_top(),
                    self.scale_ratio,
                    self.stroke,
                    start_drag,
                    end_drag,
                );
            }
            // todo: while dragging, the rectangle must not become a negative rectangle
            Tool::Cut { modifying } => {
                handle_cut(
                    modifying,
                    ctx,
                    response,
                    self.scale_ratio,
                    painter_rect.left_top(),
                    &mut self.cut_rect,
                    &self.texture_handle,
                );
            }
        }
    }

    fn handle_ctrl_z(&mut self, ctx: &Context) {
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(Key::Z)) {
            self.remove_annotation();
        }
    }

    fn remove_annotation(&mut self) {
        let annotation = self.annotations.pop();
        if let Some(Shape::LineSegment { .. }) = annotation {
            self.annotations.pop();
            self.annotations.pop();
        }
    }

    /// disegna i bottoni principali dell'interfaccia
    fn draw_menu_buttons(&mut self, ui: &mut Ui) -> FrameEvent {
        ui.horizontal(|ui| {
            ui.label("Tool:");
            if ui
                .selectable_label(matches!(self.current_tool, Tool::Rect { .. }), "rectangle")
                .clicked()
            {
                self.current_tool = Tool::Rect {
                    start_drag: None,
                    end_drag: None,
                };
            }
            if ui
                .selectable_label(matches!(self.current_tool, Tool::Circle { .. }), "circle")
                .clicked()
            {
                self.current_tool = Tool::Circle {
                    start_drag: None,
                    end_drag: None,
                };
            }
            if ui
                .selectable_label(matches!(self.current_tool, Tool::Pen { .. }), "pen")
                .clicked()
            {
                self.current_tool = Tool::Pen { line: Vec::new() };
            }
            if ui
                .selectable_label(matches!(self.current_tool, Tool::Arrow { .. }), "arrow")
                .clicked()
            {
                self.current_tool = Tool::Arrow {
                    start_drag: None,
                    end_drag: None,
                };
            }
            if ui
                .selectable_label(matches!(self.current_tool, Tool::Cut { .. }), "cut")
                .clicked()
            {
                self.current_tool = Tool::Cut {
                    modifying: ModificationOfRectangle::NoModification,
                };
            }
            ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                if ui.button("undo").clicked() {
                    self.remove_annotation();
                }
                if ui.button("clear").clicked() {
                    self.annotations = Vec::new();
                }
            });
        });
        if let Tool::Rect { .. } | Tool::Circle { .. } = self.current_tool {
            ui.horizontal(|ui| {
                ui.label("Shape:");
                ui.selectable_value(&mut self.fill_shape, true, "filled");
                ui.selectable_value(&mut self.fill_shape, false, "border");
            });
        }
        match (&self.current_tool, self.fill_shape) {
            (Tool::Rect { .. } | Tool::Circle { .. }, true) => {
                ui.horizontal(|ui| {
                    ui.label("Color:");
                    color_picker::color_edit_button_srgba(
                        ui,
                        &mut self.stroke.color,
                        Alpha::Opaque,
                    );
                });
            }
            (Tool::Rect { .. } | Tool::Circle { .. }, false)
            | (Tool::Pen { .. } | Tool::Arrow { .. }, _) => {
                stroke_ui_opaque(ui, &mut self.stroke);
            }
            (Tool::Cut { .. }, _) => {}
        }
        ui.horizontal(|ui| {
            ui.label("Format:");
            ComboBox::from_label("") //menù a tendina per la scelta del formato di output
                .selected_text(format!("{:?}", self.format))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.format, ImageFormat::Png, "Png");
                    ui.selectable_value(&mut self.format, ImageFormat::JPEG, "Jpeg");
                    ui.selectable_value(&mut self.format, ImageFormat::GIF, "Gif");
                });
            ui.with_layout(Layout::right_to_left(Align::TOP), |ui| {
                if ui.button("Abort").clicked() {
                    FrameEvent::Aborted
                } else if ui.button("Save").clicked() {
                    let (tx, rx) = channel();
                    self.receive_thread = rx;
                    let annotations = self.annotations.clone();
                    let image = self.image.clone();
                    let cut_rect = self.cut_rect;
                    thread::spawn(move || {
                        let mut image_blend = Blend(image);
                        for annotation in annotations {
                            write_annotation_to_image(&annotation, &mut image_blend);
                        }
                        tx.send(
                            crop_imm(
                                &image_blend.0,
                                cut_rect.left_top().x as u32,
                                cut_rect.left_top().y as u32,
                                cut_rect.width() as u32,
                                cut_rect.height() as u32,
                            )
                            .to_image(),
                        )
                    });
                    FrameEvent::Nil
                } else {
                    FrameEvent::Nil
                }
            })
            .inner
        })
        .inner
    }
}
