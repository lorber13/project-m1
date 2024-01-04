use crate::edit_image_utils::{
    create_circle, create_rect, hover_to_direction, make_rect_legal, obscure_screen,
    push_arrow_into_annotations, resize_rectangle, scale_annotation, scaled_rect, set_cursor,
    stroke_ui_opaque, unscaled_point, write_annotation_to_image, Direction,
};
use crate::image_coding::ImageFormat;
use eframe::egui::color_picker::Alpha;
use eframe::egui::{
    color_picker, pos2, vec2, CentralPanel, Color32, ColorImage, Context, Painter, Pos2, Rect,
    Response, Rounding, Sense, Shape, Stroke, TextureHandle, TextureOptions, Ui, Vec2,
};
use eframe::egui::{ComboBox, CursorIcon};
use image::imageops::crop_imm;
use image::RgbaImage;
use imageproc::drawing::Blend;

pub enum EditImageEvent {
    Saved {
        image: RgbaImage,
        format: ImageFormat,
    },
    Aborted,
    Nil,
}

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

#[derive(PartialEq, Debug)]
enum ModificationOfRectangle {
    Move,
    Resize { direction: Direction },
    NoModification,
}

pub struct EditImage {
    current_tool: Tool,
    cut_rect: Rect,
    stroke: Stroke,
    fill_shape: bool,
    image_blend: Blend<RgbaImage>,
    format: ImageFormat,
    texture_handle: TextureHandle,
    annotations: Vec<Shape>,
    scale_ratio: f32,
}

impl EditImage {
    pub fn new(rgba: RgbaImage, ctx: &Context) -> EditImage {
        let texture_handle = ctx.load_texture(
            "screenshot_image",
            ColorImage::from_rgba_unmultiplied(
                [rgba.width() as usize, rgba.height() as usize],
                rgba.as_raw(),
            ),
            TextureOptions::default(),
        );
        EditImage {
            cut_rect: Rect::from_min_size(pos2(0.0, 0.0), texture_handle.size_vec2()),
            current_tool: Tool::Pen { line: Vec::new() },
            texture_handle,
            image_blend: Blend(rgba),
            format: ImageFormat::Png,
            annotations: Vec::new(),
            scale_ratio: Default::default(),
            stroke: Stroke {
                width: 1.0,
                color: Color32::GREEN,
            },
            fill_shape: false,
        }
    }

    fn allocate_scaled_painter(&mut self, ui: &mut Ui) -> (Response, Painter) {
        self.update_scale_ratio(ui);
        let scaled_dimensions = vec2(
            self.texture_handle.size()[0] as f32 * self.scale_ratio,
            self.texture_handle.size()[1] as f32 * self.scale_ratio,
        );
        ui.allocate_painter(scaled_dimensions, Sense::click_and_drag())
    }

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

    fn draw_cutting_region(&mut self, painter: &Painter) {
        if let Tool::Cut { .. } = self.current_tool {
            obscure_screen(
                painter,
                scaled_rect(
                    painter.clip_rect().left_top(),
                    self.scale_ratio,
                    self.cut_rect,
                ),
                Stroke::new(3.0, Color32::YELLOW),
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

    fn draw_previous_annotations(&mut self, painter: &Painter) {
        let mut annotations = self.annotations.clone();
        for annotation in annotations.iter_mut() {
            scale_annotation(annotation, self.scale_ratio, painter.clip_rect().left_top());
        }
        painter.extend(annotations);
    }

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
    pub fn update(
        &mut self,
        ctx: &Context,
        _frame: &mut eframe::Frame,
        enabled: bool,
    ) -> EditImageEvent {
        CentralPanel::default()
            .show(ctx, |ui| {
                ui.add_enabled_ui(enabled, |ui| {
                    let ret = self.draw_menu_buttons(ui);
                    ui.separator();
                    let (response, painter) = self.allocate_scaled_painter(ui);
                    self.handle_events(ctx, response, painter.clip_rect());
                    self.display_annotations(&painter);
                    ret
                })
                .inner
            })
            .inner
    }

    fn handle_events(&mut self, ctx: &Context, response: Response, painter_rect: Rect) {
        match &mut self.current_tool {
            Tool::Pen { line } => {
                if response.drag_started() {
                    line.push(
                        response
                            .hover_pos()
                            .expect("should not panic because the pointer should be on the widget"),
                    );
                } else if response.dragged() {
                    line.push(
                        ctx.pointer_hover_pos()
                            .expect("should not panic because while dragging the pointer exists"),
                    );
                } else if response.drag_released() {
                    // no need to push current hover pos, since this frame drag is released
                    self.annotations.push(Shape::line(
                        line.clone()
                            .iter_mut()
                            .map(|point| {
                                unscaled_point(painter_rect.left_top(), self.scale_ratio, *point)
                            })
                            .collect(),
                        Stroke::new(self.stroke.width / self.scale_ratio, self.stroke.color),
                    ));
                    *line = Vec::new();
                }
            }
            Tool::Circle {
                start_drag,
                end_drag,
            } => {
                if response.drag_started() {
                    *start_drag = response.hover_pos();
                } else if response.dragged() {
                    assert!(ctx.pointer_hover_pos().is_some());
                    *end_drag = ctx.pointer_hover_pos();
                } else if response.drag_released() {
                    self.annotations.push(create_circle(
                        self.fill_shape,
                        self.scale_ratio,
                        self.stroke,
                        painter_rect.left_top(),
                        start_drag.expect("should be defined"),
                        end_drag.expect("should be defined"),
                    ));
                    *start_drag = None;
                    *end_drag = None;
                }
            }
            Tool::Rect {
                start_drag,
                end_drag,
            } => {
                if response.drag_started() {
                    *start_drag = response.hover_pos();
                } else if response.dragged() {
                    assert!(ctx.pointer_hover_pos().is_some());
                    *end_drag = ctx.pointer_hover_pos();
                } else if response.drag_released() {
                    self.annotations.push(create_rect(
                        self.fill_shape,
                        self.scale_ratio,
                        self.stroke,
                        painter_rect.left_top(),
                        start_drag.expect("should be defined"),
                        end_drag.expect("should be defined"),
                    ));
                    *start_drag = None;
                    *end_drag = None;
                }
            }
            Tool::Arrow {
                start_drag,
                end_drag,
            } => {
                if response.drag_started() {
                    *start_drag = response.hover_pos();
                } else if response.dragged() {
                    assert!(ctx.pointer_hover_pos().is_some());
                    *end_drag = ctx.pointer_hover_pos();
                } else if response.drag_released() {
                    push_arrow_into_annotations(
                        &mut self.annotations,
                        self.scale_ratio,
                        self.stroke,
                        painter_rect.left_top(),
                        start_drag.expect("should be defined"),
                        end_drag.expect("should be defined"),
                    );
                    *start_drag = None;
                    *end_drag = None;
                }
            }
            // todo: while dragging, the rectangle must not become a negative rectangle
            Tool::Cut { modifying } => {
                match modifying {
                    ModificationOfRectangle::Move => {
                        if response.dragged() {
                            ctx.set_cursor_icon(CursorIcon::Grabbing);
                            // todo: work in painter dimensions, not real dimensions
                            // todo: refine the function that makes the rectangle not escape borders
                            self.translate_rect(&response);
                        } else if response.drag_released() {
                            *modifying = ModificationOfRectangle::NoModification;
                        }
                    }
                    ModificationOfRectangle::Resize { direction } => {
                        if response.dragged() {
                            set_cursor(direction, ctx);
                            self.cut_rect = resize_rectangle(
                                self.cut_rect,
                                ctx.pointer_hover_pos().expect("should be defined"),
                                self.scale_ratio,
                                painter_rect.left_top(),
                                direction,
                            );
                        } else if response.drag_released() {
                            make_rect_legal(&mut self.cut_rect);
                            self.cut_rect = self.cut_rect.intersect(Rect::from_min_size(
                                pos2(0.0, 0.0),
                                self.texture_handle.size_vec2(),
                            ));
                            *modifying = ModificationOfRectangle::NoModification;
                        }
                    }
                    ModificationOfRectangle::NoModification => {
                        if let Some(pos) = response.hover_pos() {
                            let rect = scaled_rect(
                                painter_rect.left_top(),
                                self.scale_ratio,
                                self.cut_rect,
                            );
                            match hover_to_direction(rect, pos, 10.0) {
                                None => {
                                    // the cursor is not on the border of the cutting rectangle
                                    if rect.contains(pos) {
                                        ctx.set_cursor_icon(CursorIcon::Grab);
                                        if response.drag_started() {
                                            *modifying = ModificationOfRectangle::Move;
                                        }
                                    }
                                }
                                Some(direction) => {
                                    set_cursor(&direction, ctx);
                                    if response.drag_started() {
                                        *modifying = ModificationOfRectangle::Resize { direction }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    fn translate_rect(&mut self, response: &Response) {
        let image_rect = Rect::from_min_size(pos2(0.0, 0.0), self.texture_handle.size_vec2());
        let unscaled_delta = response.drag_delta() / self.scale_ratio;
        let translated_rect = self.cut_rect.translate(unscaled_delta);
        if image_rect.contains_rect(translated_rect) {
            self.cut_rect = translated_rect;
        } else {
            self.align_rect_to_borders(image_rect, translated_rect);
        }
    }

    fn align_rect_to_borders(&mut self, image_rect: Rect, translated_rect: Rect) {
        self.cut_rect = translated_rect.translate({
            let mut vec = Vec2::default();
            if translated_rect.left() < image_rect.left() {
                vec.x = image_rect.left() - translated_rect.left();
            }
            if translated_rect.top() < image_rect.top() {
                vec.y = image_rect.top() - translated_rect.top();
            }
            if translated_rect.right() > image_rect.right() {
                vec.x = image_rect.right() - translated_rect.right();
            }
            if translated_rect.bottom() > image_rect.bottom() {
                vec.y = image_rect.bottom() - translated_rect.bottom();
            }
            vec
        });
    }

    fn draw_menu_buttons(&mut self, ui: &mut Ui) -> EditImageEvent {
        ui.horizontal_top(|ui| {
            // todo: when the button is pressed, the enum is initialized, but the button does not keep being selected when the internal state of the enum changes
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
            if let Tool::Rect { .. } | Tool::Circle { .. } = self.current_tool {
                ui.selectable_value(&mut self.fill_shape, true, "filled");
                ui.selectable_value(&mut self.fill_shape, false, "border");
            }
            match (&self.current_tool, self.fill_shape) {
                (Tool::Rect { .. } | Tool::Circle { .. }, true) => {
                    color_picker::color_edit_button_srgba(
                        ui,
                        &mut self.stroke.color,
                        Alpha::Opaque,
                    );
                }
                (Tool::Rect { .. } | Tool::Circle { .. }, false)
                | (Tool::Pen { .. } | Tool::Arrow { .. }, _) => {
                    stroke_ui_opaque(ui, &mut self.stroke);
                }
                (Tool::Cut { .. }, _) => {}
            }

            ComboBox::from_label("") //menù a tendina per la scelta del formato di output
                .selected_text(format!("{:?}", self.format))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.format, ImageFormat::Png, "Png");
                    ui.selectable_value(&mut self.format, ImageFormat::JPEG, "Jpeg");
                    ui.selectable_value(&mut self.format, ImageFormat::GIF, "Gif");
                });
            if ui.button("Save").clicked() {
                for annotation in &self.annotations {
                    write_annotation_to_image(annotation, &mut self.image_blend);
                }
                EditImageEvent::Saved {
                    image: crop_imm(
                        &self.image_blend.0,
                        self.cut_rect.left_top().x as u32,
                        self.cut_rect.left_top().y as u32,
                        self.cut_rect.width() as u32,
                        self.cut_rect.height() as u32,
                    )
                    .to_image(),
                    format: self.format,
                }
            } else if ui.button("Abort").clicked() {
                EditImageEvent::Aborted
            } else {
                EditImageEvent::Nil
            }
        })
        .inner
    }
}
