use super::egui::ComboBox;
use crate::image_coding::ImageFormat;
use eframe::egui::{
    pos2, stroke_ui, vec2, CentralPanel, Color32, ColorImage, Context, Painter, Pos2, Rect,
    Response, Rounding, Sense, Shape, Stroke, TextureHandle, Ui,
};
use eframe::epaint::{CircleShape, RectShape};
use image::RgbaImage;

pub enum EditImageEvent {
    Saved {
        image: RgbaImage,
        format: ImageFormat,
    },
    Aborted,
    Nil,
}

#[derive(PartialEq)]
enum Tool {
    Line,
    Circle,
    Rect,
    Arrow,
    /* todo:
       text, very difficult
       rubber, not mandatory but recommended
    */
}

pub struct EditImage {
    current_shape: Tool,
    stroke: Stroke,
    fill_shape: bool,
    start_drag: Option<Pos2>,
    image: RgbaImage,
    format: ImageFormat,
    texture_handle: TextureHandle,
    annotations: Vec<Shape>,
    scale_ratio: f32,
}

impl EditImage {
    pub fn new(rgba: RgbaImage, ctx: &Context) -> EditImage {
        EditImage {
            current_shape: Tool::Rect,
            start_drag: None,
            texture_handle: ctx.load_texture(
                "screenshot_image",
                ColorImage::from_rgba_unmultiplied(
                    [rgba.width() as usize, rgba.height() as usize],
                    rgba.as_raw(),
                ),
                Default::default(),
            ),
            image: rgba,
            format: ImageFormat::Png,
            annotations: Vec::new(),
            scale_ratio: Default::default(),
            stroke: Stroke {
                width: 1.0,
                color: Color32::GREEN.gamma_multiply(0.5),
            },
            fill_shape: false,
        }
    }

    fn display_window(&mut self, ui: &mut Ui) -> (Response, Painter) {
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
        let scaled_dimensions = vec2(
            self.texture_handle.size()[0] as f32 * self.scale_ratio,
            self.texture_handle.size()[1] as f32 * self.scale_ratio,
        );
        let (response, painter) = ui.allocate_painter(scaled_dimensions, Sense::click_and_drag());
        painter.image(
            self.texture_handle.id(),
            Rect::from_min_size(painter.clip_rect().left_top(), scaled_dimensions),
            Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        let mut new_annotations = self.annotations.clone();
        for annotation in new_annotations.iter_mut() {
            match annotation {
                Shape::Rect(rect_shape) => {
                    rect_shape.rect.min.x *= self.scale_ratio;
                    rect_shape.rect.min.x += painter.clip_rect().min.x;
                    rect_shape.rect.min.y *= self.scale_ratio;
                    rect_shape.rect.min.y += painter.clip_rect().min.y;
                    rect_shape.rect.max.x *= self.scale_ratio;
                    rect_shape.rect.max.x += painter.clip_rect().min.x;
                    rect_shape.rect.max.y *= self.scale_ratio;
                    rect_shape.rect.max.y += painter.clip_rect().min.y;
                },
                Shape::Circle(circle_shape) => {
                    circle_shape.center.x *= self.scale_ratio;
                    circle_shape.center.x += painter.clip_rect().min.x;
                    circle_shape.center.y *= self.scale_ratio;
                    circle_shape.center.y += painter.clip_rect().min.y;
                    circle_shape.radius *= self.scale_ratio;
                }
                _ => {}
            }
        }
        painter.extend(new_annotations); // Do I need to redraw annotations every single frame? Yes because every frame the scaling ratio can change
        (response, painter)
    }
    fn draw_shape(&mut self, painter: &Painter, response: &Response) {
        match (&self.current_shape, self.fill_shape) {
            (Tool::Rect, true) => {
                painter.rect_filled(
                    Rect::from_two_pos(self.start_drag.unwrap(), response.hover_pos().unwrap()), // todo: manage hover outside the response
                    Rounding::none(),
                    self.stroke.color,
                );
            }
            (Tool::Rect, false) => {
                painter.rect_stroke(
                    Rect::from_two_pos(self.start_drag.unwrap(), response.hover_pos().unwrap()), // todo: manage hover outside the response
                    Rounding::none(),
                    self.stroke,
                );
            }
            (Tool::Circle, true) => {
                painter.circle_filled(
                    self.start_drag.unwrap(),
                    response
                        .hover_pos()
                        .unwrap()
                        .distance(self.start_drag.unwrap()),
                    self.stroke.color,
                );
            }
            (Tool::Circle, false) => {
                painter.circle_stroke(
                    self.start_drag.unwrap(),
                    response
                        .hover_pos()
                        .unwrap()
                        .distance(self.start_drag.unwrap()),
                    self.stroke,
                );
            }
            (Tool::Line, _) => {
                todo!()
            }
            (Tool::Arrow, _) => {
                todo!()
            }
        }
    }
    fn scaled_rect(&self, painter: &Painter, response: &Response) -> Rect {
        Rect::from_two_pos(
            pos2(
                (self.start_drag.unwrap().x - painter.clip_rect().min.x) / self.scale_ratio,
                (self.start_drag.unwrap().y - painter.clip_rect().min.y) / self.scale_ratio,
            ),
            pos2(
                (response.hover_pos().unwrap().x - painter.clip_rect().min.x) / self.scale_ratio,
                (response.hover_pos().unwrap().y - painter.clip_rect().min.y) / self.scale_ratio,
            ),
        ) // todo: manage hover outside the response
    }
    fn push_shape(&mut self, painter: &Painter, response: &Response) {
        self.annotations
            .push(match (&self.current_shape, self.fill_shape) {
                (Tool::Rect, true) => Shape::Rect(RectShape::filled(
                    self.scaled_rect(painter, response),
                    Rounding::none(),
                    self.stroke.color,
                )),
                (Tool::Rect, false) => Shape::Rect(RectShape::stroke(
                    self.scaled_rect(painter, response),
                    Rounding::none(),
                    self.stroke,
                )),
                (Tool::Circle, false) => Shape::Circle(CircleShape::stroke(
                    pos2(
                        (self.start_drag.unwrap().x - painter.clip_rect().min.x) / self.scale_ratio,
                        (self.start_drag.unwrap().y - painter.clip_rect().min.y) / self.scale_ratio,
                    ),
                    response
                        .hover_pos()
                        .unwrap()
                        .distance(self.start_drag.unwrap())
                        / self.scale_ratio, // todo: manage hover outside the response
                    self.stroke,
                )),
                (Tool::Circle, true) => Shape::Circle(CircleShape::filled(
                    pos2(
                        (self.start_drag.unwrap().x - painter.clip_rect().min.x) / self.scale_ratio,
                        (self.start_drag.unwrap().y - painter.clip_rect().min.y) / self.scale_ratio,
                    ),
                    response
                        .hover_pos()
                        .unwrap()
                        .distance(self.start_drag.unwrap())
                        / self.scale_ratio, // todo: manage hover outside the response
                    self.stroke.color,
                )),
                _ => todo!(),
            });
    }
    pub fn update(
        &mut self,
        ctx: &Context,
        _frame: &mut eframe::Frame,
        enabled: bool,
    ) -> EditImageEvent {
        let mut ret = EditImageEvent::Nil;
        CentralPanel::default().show(ctx, |ui| {
            ui.add_enabled_ui(enabled, |ui| {
                ui.horizontal_top(|ui| {
                    ComboBox::from_label("") //menù a tendina per la scelta del formato di output
                        .selected_text(format!("{:?}", self.format))
                        .show_ui(ui, |ui| {
                            ui.style_mut().wrap = Some(false);
                            ui.set_min_width(60.0);
                            ui.selectable_value(&mut self.format, ImageFormat::Png, "Png");
                            ui.selectable_value(&mut self.format, ImageFormat::JPEG, "Jpeg");
                            ui.selectable_value(&mut self.format, ImageFormat::GIF, "Gif");
                        });
                    ui.selectable_value(&mut self.current_shape, Tool::Rect, "rectangle");
                    ui.selectable_value(&mut self.current_shape, Tool::Circle, "circle");
                    ui.selectable_value(&mut self.current_shape, Tool::Line, "line");
                    ui.selectable_value(&mut self.current_shape, Tool::Arrow, "arrow");
                    if let Tool::Rect | Tool::Circle = self.current_shape {
                        ui.selectable_value(&mut self.fill_shape, true, "filled");
                        ui.selectable_value(&mut self.fill_shape, false, "border");
                    }
                    match (&self.current_shape, self.fill_shape) {
                        (Tool::Rect | Tool::Circle, true) => {
                            ui.color_edit_button_srgba(&mut self.stroke.color);
                        }
                        (Tool::Rect | Tool::Circle, false) | (Tool::Line, _) => {
                            stroke_ui(ui, &mut self.stroke, "Stroke");
                        }
                        _ => {}
                    }

                    if ui.button("Save").clicked() {
                        ret = EditImageEvent::Saved {
                            image: self.image.clone(), // todo: ugly clone
                            format: self.format.clone(),
                        };
                    }
                    if ui.button("Abort").clicked() {
                        ret = EditImageEvent::Aborted;
                    }
                });
                ui.separator();
                let (response, painter) = self.display_window(ui);
                if response.drag_started() {
                    self.start_drag = response.hover_pos();
                } else if response.dragged() {
                    self.draw_shape(&painter, &response);
                } else if response.drag_released() {
                    self.draw_shape(&painter, &response);
                    self.push_shape(&painter, &response);
                }
            });
        });
        ret
    }
}
