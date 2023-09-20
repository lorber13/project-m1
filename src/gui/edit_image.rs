use super::egui::ComboBox;
use crate::image_coding::ImageFormat;
use eframe::egui::{
    pos2, stroke_ui, vec2, CentralPanel, Color32, ColorImage, Context, Painter, Pos2, Rect,
    Response, Rounding, Sense, Shape, Stroke, TextureHandle, Ui,
};
use eframe::emath::Rot2;
use eframe::epaint::{CircleShape, RectShape};
use image::RgbaImage;
use std::ops::Sub;

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
    Pen,
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
    line: Vec<Pos2>,
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
            line: Vec::new(),
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
                    rect_shape.stroke.width *= self.scale_ratio;
                }
                Shape::Circle(circle_shape) => {
                    circle_shape.center.x *= self.scale_ratio;
                    circle_shape.center.x += painter.clip_rect().min.x;
                    circle_shape.center.y *= self.scale_ratio;
                    circle_shape.center.y += painter.clip_rect().min.y;
                    circle_shape.radius *= self.scale_ratio;
                    circle_shape.stroke.width *= self.scale_ratio;
                }
                Shape::LineSegment { points, stroke } => {
                    points[0].x *= self.scale_ratio;
                    points[0].x += painter.clip_rect().min.x;
                    points[0].y *= self.scale_ratio;
                    points[0].y += painter.clip_rect().min.y;
                    points[1].x *= self.scale_ratio;
                    points[1].x += painter.clip_rect().min.x;
                    points[1].y *= self.scale_ratio;
                    points[1].y += painter.clip_rect().min.y;
                    stroke.width *= self.scale_ratio;
                }
                Shape::Path(path_shape) => {
                    path_shape.stroke.width *= self.scale_ratio;
                    for point in path_shape.points.iter_mut() {
                        point.x *= self.scale_ratio;
                        point.x += painter.clip_rect().min.x;
                        point.y *= self.scale_ratio;
                        point.y += painter.clip_rect().min.y;
                    }
                }
                _ => unreachable!(),
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
            (Tool::Pen, _) => {
                painter.add(Shape::line(self.line.clone(), self.stroke));
            }
            (Tool::Arrow, _) => {
                painter.arrow(
                    self.start_drag.unwrap(),
                    response.hover_pos().unwrap().sub(self.start_drag.unwrap()),
                    self.stroke,
                );
            }
        }
    }
    fn scaled_rect(&self, painter: &Painter, response: &Response) -> Rect {
        Rect::from_two_pos(
            scaled_point(
                painter.clip_rect().left_top(),
                self.scale_ratio,
                self.start_drag.unwrap(),
            ),
            scaled_point(
                painter.clip_rect().left_top(),
                self.scale_ratio,
                response.hover_pos().unwrap(),
            ),
        ) // todo: manage hover outside the response
    }
    fn push_shape(&mut self, painter: &Painter, response: &Response) {
        match (&self.current_shape, self.fill_shape) {
            (Tool::Rect, true) => self.annotations.push(Shape::Rect(RectShape::filled(
                self.scaled_rect(painter, response),
                Rounding::none(),
                self.stroke.color,
            ))),
            (Tool::Rect, false) => self.annotations.push(Shape::Rect(RectShape::stroke(
                self.scaled_rect(painter, response),
                Rounding::none(),
                Stroke::new(self.stroke.width / self.scale_ratio, self.stroke.color),
            ))),
            (Tool::Circle, false) => self.annotations.push(Shape::Circle(CircleShape::stroke(
                scaled_point(
                    painter.clip_rect().left_top(),
                    self.scale_ratio,
                    self.start_drag.unwrap(),
                ),
                response
                    .hover_pos()
                    .unwrap()
                    .distance(self.start_drag.unwrap())
                    / self.scale_ratio, // todo: manage hover outside the response
                Stroke::new(self.stroke.width / self.scale_ratio, self.stroke.color),
            ))),
            (Tool::Circle, true) => self.annotations.push(Shape::Circle(CircleShape::filled(
                scaled_point(
                    painter.clip_rect().left_top(),
                    self.scale_ratio,
                    self.start_drag.unwrap(),
                ),
                response
                    .hover_pos()
                    .unwrap()
                    .distance(self.start_drag.unwrap())
                    / self.scale_ratio, // todo: manage hover outside the response
                self.stroke.color,
            ))),
            (Tool::Arrow, _) => {
                let vec = response.hover_pos().unwrap().sub(self.start_drag.unwrap());
                let origin = self.start_drag.unwrap();
                let rot = Rot2::from_angle(std::f32::consts::TAU / 10.0);
                let tip_length = vec.length() / 4.0;
                let tip = origin + vec;
                let dir = vec.normalized();
                self.annotations.push(Shape::LineSegment {
                    points: [
                        scaled_point(painter.clip_rect().left_top(), self.scale_ratio, origin),
                        scaled_point(painter.clip_rect().left_top(), self.scale_ratio, tip),
                    ],
                    stroke: Stroke::new(self.stroke.width / self.scale_ratio, self.stroke.color),
                });
                self.annotations.push(Shape::LineSegment {
                    points: [
                        scaled_point(painter.clip_rect().left_top(), self.scale_ratio, tip),
                        scaled_point(
                            painter.clip_rect().left_top(),
                            self.scale_ratio,
                            tip - tip_length * (rot * dir),
                        ),
                    ],
                    stroke: Stroke::new(self.stroke.width / self.scale_ratio, self.stroke.color),
                });
                self.annotations.push(Shape::LineSegment {
                    points: [
                        scaled_point(painter.clip_rect().left_top(), self.scale_ratio, tip),
                        scaled_point(
                            painter.clip_rect().left_top(),
                            self.scale_ratio,
                            tip - tip_length * (rot.inverse() * dir),
                        ),
                    ],
                    stroke: Stroke::new(self.stroke.width / self.scale_ratio, self.stroke.color),
                });
            }
            (Tool::Pen, _) => {
                self.annotations.push(Shape::line(
                    self.line
                        .clone()
                        .iter_mut()
                        .map(|point| {
                            scaled_point(painter.clip_rect().left_top(), self.scale_ratio, *point)
                        })
                        .collect(),
                    Stroke::new(self.stroke.width / self.scale_ratio, self.stroke.color),
                ));
                self.line = Vec::new();
            }
        }
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
                    ui.selectable_value(&mut self.current_shape, Tool::Rect, "rectangle");
                    ui.selectable_value(&mut self.current_shape, Tool::Circle, "circle");
                    ui.selectable_value(&mut self.current_shape, Tool::Pen, "pen");
                    ui.selectable_value(&mut self.current_shape, Tool::Arrow, "arrow");
                    if let Tool::Rect | Tool::Circle = self.current_shape {
                        ui.selectable_value(&mut self.fill_shape, true, "filled");
                        ui.selectable_value(&mut self.fill_shape, false, "border");
                    }
                    match (&self.current_shape, self.fill_shape) {
                        (Tool::Rect | Tool::Circle, true) => {
                            ui.color_edit_button_srgba(&mut self.stroke.color);
                        }
                        (Tool::Rect | Tool::Circle, false) | (Tool::Pen, _) | (Tool::Arrow, _) => {
                            stroke_ui(ui, &mut self.stroke, "Stroke");
                        }
                    }

                    ComboBox::from_label("") //menù a tendina per la scelta del formato di output
                        .selected_text(format!("{:?}", self.format))
                        .show_ui(ui, |ui| {
                            ui.style_mut().wrap = Some(false);
                            ui.set_min_width(60.0);
                            ui.selectable_value(&mut self.format, ImageFormat::Png, "Png");
                            ui.selectable_value(&mut self.format, ImageFormat::JPEG, "Jpeg");
                            ui.selectable_value(&mut self.format, ImageFormat::GIF, "Gif");
                        });
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
                    if let Tool::Pen = self.current_shape {
                        self.line.push(response.hover_pos().expect(
                            "should not panic because the pointer should be on the widget",
                        ));
                    }
                } else if response.dragged() {
                    if let Tool::Pen = self.current_shape {
                        self.line.push(response.hover_pos().unwrap()); // todo: manage hover outside the response
                    }
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

fn scaled_point(top_left: Pos2, scale_ratio: f32, point: Pos2) -> Pos2 {
    pos2(
        (point.x - top_left.x) / scale_ratio,
        (point.y - top_left.y) / scale_ratio,
    )
}
