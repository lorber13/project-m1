use crate::image_coding::ImageFormat;
use eframe::egui::{
    pos2, stroke_ui, vec2, CentralPanel, Color32, ColorImage, Context, Painter, Pos2, Rect,
    Response, Rounding, Sense, Shape, Stroke, TextureHandle, Ui,
};
use eframe::egui::{ComboBox, CursorIcon};
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

#[derive(PartialEq, Debug)]
enum Tool {
    Pen {
        line: Vec<Pos2>,
    },
    Circle {
        start_drag: Option<Pos2>,
    },
    Rect {
        start_drag: Option<Pos2>,
    },
    Arrow {
        start_drag: Option<Pos2>,
    },
    Cut {
        state_of_current_rectangle: CuttingRectangle,
    },
    /* todo:
       text, very difficult
       rubber, not mandatory but recommended
    */
}

#[derive(PartialEq, Debug)]
enum CuttingRectangle {
    NonExistent,
    Creation {
        start_drag: Pos2,
    },
    Existent {
        rect: Rect,
        resizing: ResizeDirection,
    },
}

#[derive(PartialEq, Debug)]
enum ResizeDirection {
    Top,
    Bottom,
    Left,
    Right,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
    NoResize,
}

pub struct EditImage {
    current_tool: Tool,
    stroke: Stroke,
    fill_shape: bool,
    image: RgbaImage,
    format: ImageFormat,
    texture_handle: TextureHandle,
    annotations: Vec<Shape>,
    scale_ratio: f32,
}

impl EditImage {
    pub fn new(rgba: RgbaImage, ctx: &Context) -> EditImage {
        EditImage {
            current_tool: Tool::Pen { line: Vec::new() },
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
                    rect_shape.rect.min.x += painter.clip_rect().left_top().x;
                    rect_shape.rect.min.y *= self.scale_ratio;
                    rect_shape.rect.min.y += painter.clip_rect().left_top().y;
                    rect_shape.rect.max.x *= self.scale_ratio;
                    rect_shape.rect.max.x += painter.clip_rect().left_top().x;
                    rect_shape.rect.max.y *= self.scale_ratio;
                    rect_shape.rect.max.y += painter.clip_rect().left_top().y;
                    rect_shape.stroke.width *= self.scale_ratio;
                }
                Shape::Circle(circle_shape) => {
                    circle_shape.center.x *= self.scale_ratio;
                    circle_shape.center.x += painter.clip_rect().left_top().x;
                    circle_shape.center.y *= self.scale_ratio;
                    circle_shape.center.y += painter.clip_rect().left_top().y;
                    circle_shape.radius *= self.scale_ratio;
                    circle_shape.stroke.width *= self.scale_ratio;
                }
                Shape::LineSegment { points, stroke } => {
                    points[0].x *= self.scale_ratio;
                    points[0].x += painter.clip_rect().left_top().x;
                    points[0].y *= self.scale_ratio;
                    points[0].y += painter.clip_rect().left_top().y;
                    points[1].x *= self.scale_ratio;
                    points[1].x += painter.clip_rect().left_top().x;
                    points[1].y *= self.scale_ratio;
                    points[1].y += painter.clip_rect().left_top().y;
                    stroke.width *= self.scale_ratio;
                }
                Shape::Path(path_shape) => {
                    path_shape.stroke.width *= self.scale_ratio;
                    for point in path_shape.points.iter_mut() {
                        point.x *= self.scale_ratio;
                        point.x += painter.clip_rect().left_top().x;
                        point.y *= self.scale_ratio;
                        point.y += painter.clip_rect().left_top().y;
                    }
                }
                _ => unreachable!(),
            }
        }
        painter.extend(new_annotations); // Do I need to redraw annotations every single frame? Yes because every frame the scaling ratio can change
        (response, painter)
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
                self.draw_menu_buttons(&mut ret, ui);
                ui.separator();
                let (response, painter) = self.display_window(ui);
                match &mut self.current_tool {
                    Tool::Pen { line } => {
                        if response.drag_started() {
                            line.push(response.hover_pos().expect(
                                "should not panic because the pointer should be on the widget",
                            ));
                        } else if response.dragged() {
                            line.push(response.hover_pos().unwrap()); // todo: manage hover outside the response
                            painter.add(Shape::line(line.clone(), self.stroke));
                        // todo: check if clone is necessary
                        } else if response.drag_released() {
                            // no need to push current hover pos, since this frame drag is released
                            painter.add(Shape::line(line.clone(), self.stroke)); // todo: check if necessary clone
                            self.annotations.push(Shape::line(
                                line.clone()
                                    .iter_mut()
                                    .map(|point| {
                                        unscaled_point(
                                            painter.clip_rect().left_top(),
                                            self.scale_ratio,
                                            *point,
                                        )
                                    })
                                    .collect(),
                                Stroke::new(
                                    self.stroke.width / self.scale_ratio,
                                    self.stroke.color,
                                ),
                            ));
                            *line = Vec::new();
                        }
                    }
                    Tool::Circle { start_drag } => {
                        if response.drag_started() {
                            *start_drag = response.hover_pos();
                        } else if response.dragged() {
                            if self.fill_shape {
                                painter.circle_filled(
                                    start_drag.unwrap(),
                                    response.hover_pos().unwrap().distance(start_drag.unwrap()),
                                    self.stroke.color,
                                );
                            } else {
                                painter.circle_stroke(
                                    start_drag.unwrap(),
                                    response.hover_pos().unwrap().distance(start_drag.unwrap()),
                                    self.stroke,
                                );
                            }
                        } else if response.drag_released() {
                            if self.fill_shape {
                                painter.circle_filled(
                                    start_drag.unwrap(),
                                    response.hover_pos().unwrap().distance(start_drag.unwrap()),
                                    self.stroke.color,
                                );
                                self.annotations.push(Shape::Circle(CircleShape::filled(
                                    unscaled_point(
                                        painter.clip_rect().left_top(),
                                        self.scale_ratio,
                                        start_drag.unwrap(),
                                    ),
                                    response.hover_pos().unwrap().distance(start_drag.unwrap())
                                        / self.scale_ratio, // todo: manage hover outside the response
                                    self.stroke.color,
                                )));
                            } else {
                                painter.circle_stroke(
                                    start_drag.unwrap(),
                                    response.hover_pos().unwrap().distance(start_drag.unwrap()),
                                    self.stroke,
                                );
                                self.annotations.push(Shape::Circle(CircleShape::stroke(
                                    unscaled_point(
                                        painter.clip_rect().left_top(),
                                        self.scale_ratio,
                                        start_drag.unwrap(),
                                    ),
                                    response.hover_pos().unwrap().distance(start_drag.unwrap())
                                        / self.scale_ratio, // todo: manage hover outside the response
                                    Stroke::new(
                                        self.stroke.width / self.scale_ratio,
                                        self.stroke.color,
                                    ),
                                )));
                            }
                        }
                    }
                    Tool::Rect { start_drag } => {
                        if response.drag_started() {
                            *start_drag = response.hover_pos();
                        } else if response.dragged() {
                            if self.fill_shape {
                                painter.rect_filled(
                                    Rect::from_two_pos(
                                        start_drag.unwrap(),
                                        response.hover_pos().unwrap(),
                                    ), // todo: manage hover outside the response
                                    Rounding::none(),
                                    self.stroke.color,
                                );
                            } else {
                                painter.rect_stroke(
                                    Rect::from_two_pos(
                                        start_drag.unwrap(),
                                        response.hover_pos().unwrap(),
                                    ), // todo: manage hover outside the response
                                    Rounding::none(),
                                    self.stroke,
                                );
                            }
                        } else if response.drag_released() {
                            if self.fill_shape {
                                painter.rect_filled(
                                    Rect::from_two_pos(
                                        start_drag.unwrap(),
                                        response.hover_pos().unwrap(),
                                    ), // todo: manage hover outside the response
                                    Rounding::none(),
                                    self.stroke.color,
                                );
                                self.annotations.push(Shape::Rect(RectShape::filled(
                                    unscaled_rect(
                                        painter.clip_rect().left_top(),
                                        self.scale_ratio,
                                        Rect::from_two_pos(
                                            start_drag.unwrap(),
                                            response.hover_pos().unwrap(),
                                        ), // todo: manage hover outside the response
                                    ),
                                    Rounding::none(),
                                    self.stroke.color,
                                )));
                            } else {
                                painter.rect_stroke(
                                    Rect::from_two_pos(
                                        start_drag.unwrap(),
                                        response.hover_pos().unwrap(),
                                    ), // todo: manage hover outside the response
                                    Rounding::none(),
                                    self.stroke,
                                );
                                self.annotations.push(Shape::Rect(RectShape::stroke(
                                    unscaled_rect(
                                        painter.clip_rect().left_top(),
                                        self.scale_ratio,
                                        Rect::from_two_pos(
                                            start_drag.unwrap(),
                                            response.hover_pos().unwrap(),
                                        ), // todo: manage hover outside the response
                                    ),
                                    Rounding::none(),
                                    Stroke::new(
                                        self.stroke.width / self.scale_ratio,
                                        self.stroke.color,
                                    ),
                                )))
                            }
                        }
                    }
                    Tool::Arrow { start_drag } => {
                        if response.drag_started() {
                            *start_drag = response.hover_pos();
                        } else if response.dragged() {
                            painter.arrow(
                                start_drag.unwrap(),
                                response.hover_pos().unwrap().sub(start_drag.unwrap()),
                                self.stroke,
                            );
                        } else if response.drag_released() {
                            painter.arrow(
                                start_drag.unwrap(),
                                response.hover_pos().unwrap().sub(start_drag.unwrap()),
                                self.stroke,
                            );
                            let vec = response.hover_pos().unwrap().sub(start_drag.unwrap());
                            let origin = start_drag.unwrap();
                            let rot = Rot2::from_angle(std::f32::consts::TAU / 10.0);
                            let tip_length = vec.length() / 4.0;
                            let tip = origin + vec;
                            let dir = vec.normalized();
                            self.annotations.push(Shape::LineSegment {
                                points: [
                                    unscaled_point(
                                        painter.clip_rect().left_top(),
                                        self.scale_ratio,
                                        origin,
                                    ),
                                    unscaled_point(
                                        painter.clip_rect().left_top(),
                                        self.scale_ratio,
                                        tip,
                                    ),
                                ],
                                stroke: Stroke::new(
                                    self.stroke.width / self.scale_ratio,
                                    self.stroke.color,
                                ),
                            });
                            self.annotations.push(Shape::LineSegment {
                                points: [
                                    unscaled_point(
                                        painter.clip_rect().left_top(),
                                        self.scale_ratio,
                                        tip,
                                    ),
                                    unscaled_point(
                                        painter.clip_rect().left_top(),
                                        self.scale_ratio,
                                        tip - tip_length * (rot * dir),
                                    ),
                                ],
                                stroke: Stroke::new(
                                    self.stroke.width / self.scale_ratio,
                                    self.stroke.color,
                                ),
                            });
                            self.annotations.push(Shape::LineSegment {
                                points: [
                                    unscaled_point(
                                        painter.clip_rect().left_top(),
                                        self.scale_ratio,
                                        tip,
                                    ),
                                    unscaled_point(
                                        painter.clip_rect().left_top(),
                                        self.scale_ratio,
                                        tip - tip_length * (rot.inverse() * dir),
                                    ),
                                ],
                                stroke: Stroke::new(
                                    self.stroke.width / self.scale_ratio,
                                    self.stroke.color,
                                ),
                            });
                        }
                    }
                    Tool::Cut {
                        ref mut state_of_current_rectangle,
                    } => match state_of_current_rectangle {
                        // todo: why response.clicked() always returns false? should I embed states in the input? I cannot
                        CuttingRectangle::NonExistent => {
                            painter.rect_filled(
                                painter.clip_rect(),
                                Rounding::none(),
                                Color32::from_black_alpha(200),
                            );
                            ctx.set_cursor_icon(CursorIcon::Crosshair);
                            if response.drag_started() {
                                *state_of_current_rectangle = CuttingRectangle::Creation {
                                    start_drag: response.hover_pos().unwrap(),
                                }
                            }
                        }
                        CuttingRectangle::Creation { start_drag } => {
                            let rectangle_drawn_until_now =
                                Rect::from_two_pos(*start_drag, response.hover_pos().unwrap());
                            if response.dragged() {
                                ctx.set_cursor_icon(CursorIcon::Crosshair);
                                obscure_screen(&painter, rectangle_drawn_until_now);
                            } else if response.drag_released() {
                                if rectangle_drawn_until_now.area() > 0.0 {
                                    // todo: this piece of code is a workaround. It is not meant to be in the final release.
                                    *state_of_current_rectangle = CuttingRectangle::Existent {
                                        rect: unscaled_rect(
                                            painter.clip_rect().left_top(),
                                            self.scale_ratio,
                                            rectangle_drawn_until_now,
                                        ),
                                        resizing: ResizeDirection::NoResize,
                                    }
                                } else {
                                    *state_of_current_rectangle = CuttingRectangle::NonExistent;
                                }
                                obscure_screen(&painter, rectangle_drawn_until_now);
                            } else {
                                unreachable!()
                            }
                        }
                        CuttingRectangle::Existent { rect, resizing } => {
                            obscure_screen(
                                &painter,
                                scaled_rect(
                                    painter.clip_rect().left_top(),
                                    self.scale_ratio,
                                    *rect,
                                ),
                            );
                            match resizing {
                                ResizeDirection::Top => {
                                    if response.dragged() {
                                        ctx.set_cursor_icon(CursorIcon::ResizeVertical);
                                        rect.set_top(
                                            unscaled_point(
                                                painter.clip_rect().left_top(),
                                                self.scale_ratio,
                                                response.hover_pos().unwrap_or_else(|| todo!()),
                                            )
                                            .y,
                                        );
                                    } else if response.drag_released() {
                                        *resizing = ResizeDirection::NoResize;
                                    }
                                }
                                ResizeDirection::Bottom => {
                                    if response.dragged() {
                                        ctx.set_cursor_icon(CursorIcon::ResizeVertical);
                                        rect.set_bottom(
                                            unscaled_point(
                                                painter.clip_rect().left_top(),
                                                self.scale_ratio,
                                                response.hover_pos().unwrap_or_else(|| todo!()),
                                            )
                                            .y,
                                        );
                                    } else if response.drag_released() {
                                        *resizing = ResizeDirection::NoResize;
                                    }
                                }
                                ResizeDirection::Left => {
                                    if response.dragged() {
                                        ctx.set_cursor_icon(CursorIcon::ResizeHorizontal);
                                        rect.set_left(
                                            unscaled_point(
                                                painter.clip_rect().left_top(),
                                                self.scale_ratio,
                                                response.hover_pos().unwrap_or_else(|| todo!()),
                                            )
                                            .x,
                                        );
                                    } else if response.drag_released() {
                                        *resizing = ResizeDirection::NoResize;
                                    }
                                }
                                ResizeDirection::Right => {
                                    if response.dragged() {
                                        ctx.set_cursor_icon(CursorIcon::ResizeHorizontal);
                                        rect.set_right(
                                            unscaled_point(
                                                painter.clip_rect().left_top(),
                                                self.scale_ratio,
                                                response.hover_pos().unwrap_or_else(|| todo!()),
                                            )
                                            .x,
                                        );
                                    } else if response.drag_released() {
                                        *resizing = ResizeDirection::NoResize;
                                    }
                                }
                                ResizeDirection::TopLeft => {
                                    if response.dragged() {
                                        let point = unscaled_point(
                                            painter.clip_rect().left_top(),
                                            self.scale_ratio,
                                            response.hover_pos().unwrap_or_else(|| todo!()),
                                        );
                                        ctx.set_cursor_icon(CursorIcon::ResizeNorthWest);
                                        rect.set_top(point.y);
                                        rect.set_left(point.x);
                                    } else if response.drag_released() {
                                        *resizing = ResizeDirection::NoResize;
                                    }
                                }
                                ResizeDirection::TopRight => {
                                    if response.dragged() {
                                        let point = unscaled_point(
                                            painter.clip_rect().left_top(),
                                            self.scale_ratio,
                                            response.hover_pos().unwrap_or_else(|| todo!()),
                                        );
                                        ctx.set_cursor_icon(CursorIcon::ResizeNorthEast);
                                        rect.set_top(point.y);
                                        rect.set_right(point.x);
                                    } else if response.drag_released() {
                                        *resizing = ResizeDirection::NoResize;
                                    }
                                }
                                ResizeDirection::BottomLeft => {
                                    if response.dragged() {
                                        let point = unscaled_point(
                                            painter.clip_rect().left_top(),
                                            self.scale_ratio,
                                            response.hover_pos().unwrap_or_else(|| todo!()),
                                        );
                                        ctx.set_cursor_icon(CursorIcon::ResizeSouthWest);
                                        rect.set_bottom(point.y);
                                        rect.set_left(point.x);
                                    } else if response.drag_released() {
                                        *resizing = ResizeDirection::NoResize;
                                    }
                                }
                                ResizeDirection::BottomRight => {
                                    let point = unscaled_point(
                                        painter.clip_rect().left_top(),
                                        self.scale_ratio,
                                        response.hover_pos().unwrap_or_else(|| todo!()),
                                    );
                                    if response.dragged() {
                                        ctx.set_cursor_icon(CursorIcon::ResizeSouthEast);
                                        rect.set_bottom(point.y);
                                        rect.set_right(point.x);
                                    } else if response.drag_released() {
                                        *resizing = ResizeDirection::NoResize;
                                    }
                                }
                                ResizeDirection::NoResize => {
                                    if let Some(mut pos) = response.hover_pos() {
                                        pos = unscaled_point(
                                            painter.clip_rect().left_top(),
                                            self.scale_ratio,
                                            pos,
                                        );
                                        // top-left corner of the rectangle
                                        if rect.left_top().round() == pos.round() {
                                            ctx.set_cursor_icon(CursorIcon::ResizeNorthWest);
                                            if response.drag_started() {
                                                *resizing = ResizeDirection::TopLeft;
                                            }
                                        }
                                        // top-right corner of the rectangle
                                        else if rect.right_top().round() == pos.round() {
                                            ctx.set_cursor_icon(CursorIcon::ResizeNorthEast);
                                            if response.drag_started() {
                                                *resizing = ResizeDirection::TopRight;
                                            }
                                        }
                                        // bottom-left corner of the rectangle
                                        else if rect.left_bottom().round() == pos.round() {
                                            ctx.set_cursor_icon(CursorIcon::ResizeSouthWest);
                                            if response.drag_started() {
                                                *resizing = ResizeDirection::BottomLeft;
                                            }
                                        }
                                        // bottom-right corner of the rectangle
                                        else if rect.right_bottom().round() == pos.round() {
                                            ctx.set_cursor_icon(CursorIcon::ResizeSouthEast);
                                            if response.drag_started() {
                                                *resizing = ResizeDirection::BottomRight;
                                            }
                                        }
                                        // right segment of the rectangle
                                        else if rect.right().round() == pos.x.round()
                                            && pos.y >= rect.top()
                                            && pos.y <= rect.bottom()
                                        {
                                            // todo: manage equivalence between f32. Is round() sufficient?
                                            ctx.set_cursor_icon(CursorIcon::ResizeHorizontal);
                                            if response.drag_started() {
                                                *resizing = ResizeDirection::Right;
                                            }
                                        }
                                        // left segment of the rectangle
                                        else if rect.left().round() == pos.x.round()
                                            && pos.y >= rect.top()
                                            && pos.y <= rect.bottom()
                                        {
                                            ctx.set_cursor_icon(CursorIcon::ResizeHorizontal);
                                            if response.drag_started() {
                                                *resizing = ResizeDirection::Left;
                                            }
                                        }
                                        // top segment of the rectangle
                                        else if rect.top().round() == pos.y.round()
                                            && pos.x >= rect.left()
                                            && pos.x <= rect.right()
                                        {
                                            ctx.set_cursor_icon(CursorIcon::ResizeVertical);
                                            if response.drag_started() {
                                                *resizing = ResizeDirection::Top;
                                            }
                                        }
                                        // bottom segment of the rectangle
                                        else if rect.bottom().round() == pos.y.round()
                                            && pos.x >= rect.left()
                                            && pos.x <= rect.right()
                                        {
                                            ctx.set_cursor_icon(CursorIcon::ResizeVertical);
                                            if response.drag_started() {
                                                *resizing = ResizeDirection::Bottom;
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    },
                }
            });
        });
        ret
    }

    fn draw_menu_buttons(&mut self, ret: &mut EditImageEvent, ui: &mut Ui) {
        ui.horizontal_top(|ui| {
            // todo: when the button is pressed, the enum is initialized, but the button does not keep being selected when the internal state of the enum changes
            if ui
                .selectable_label(
                    if let Tool::Rect { .. } = self.current_tool {
                        true
                    } else {
                        false
                    },
                    "rectangle",
                )
                .clicked()
            {
                self.current_tool = Tool::Rect { start_drag: None };
            }
            if ui
                .selectable_label(
                    if let Tool::Circle { .. } = self.current_tool {
                        true
                    } else {
                        false
                    },
                    "circle",
                )
                .clicked()
            {
                self.current_tool = Tool::Circle { start_drag: None };
            }
            if ui
                .selectable_label(
                    if let Tool::Pen { .. } = self.current_tool {
                        true
                    } else {
                        false
                    },
                    "pen",
                )
                .clicked()
            {
                self.current_tool = Tool::Pen { line: Vec::new() };
            }
            if ui
                .selectable_label(
                    if let Tool::Arrow { .. } = self.current_tool {
                        true
                    } else {
                        false
                    },
                    "arrow",
                )
                .clicked()
            {
                self.current_tool = Tool::Arrow { start_drag: None };
            }
            if ui
                .selectable_label(
                    if let Tool::Cut { .. } = self.current_tool {
                        true
                    } else {
                        false
                    },
                    "cut",
                )
                .clicked()
            {
                self.current_tool = Tool::Cut {
                    state_of_current_rectangle: CuttingRectangle::NonExistent,
                };
            }
            if let Tool::Rect { .. } | Tool::Circle { .. } = self.current_tool {
                ui.selectable_value(&mut self.fill_shape, true, "filled");
                ui.selectable_value(&mut self.fill_shape, false, "border");
            }
            match (&self.current_tool, self.fill_shape) {
                (Tool::Rect { .. } | Tool::Circle { .. }, true) => {
                    ui.color_edit_button_srgba(&mut self.stroke.color);
                }
                (Tool::Rect { .. } | Tool::Circle { .. }, false)
                | (Tool::Pen { .. }, _)
                | (Tool::Arrow { .. }, _) => {
                    stroke_ui(ui, &mut self.stroke, "Stroke");
                }
                (Tool::Cut { .. }, _) => {}
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
                *ret = EditImageEvent::Saved {
                    image: self.image.clone(), // todo: ugly clone
                    format: self.format,
                };
            }
            if ui.button("Abort").clicked() {
                *ret = EditImageEvent::Aborted;
            }
        });
    }
}

fn unscaled_point(top_left: Pos2, scale_ratio: f32, point: Pos2) -> Pos2 {
    pos2(
        (point.x - top_left.x) / scale_ratio,
        (point.y - top_left.y) / scale_ratio,
    )
}
fn unscaled_rect(top_left: Pos2, scale_ratio: f32, rect: Rect) -> Rect {
    Rect::from_two_pos(
        unscaled_point(top_left, scale_ratio, rect.left_top()),
        unscaled_point(top_left, scale_ratio, rect.right_bottom()),
    )
}

fn scaled_rect(top_left: Pos2, scale_ratio: f32, mut rect: Rect) -> Rect {
    rect.min = scaled_point(top_left, scale_ratio, rect.min);
    rect.max = scaled_point(top_left, scale_ratio, rect.max);
    rect
}

fn scaled_point(top_left: Pos2, scale_ratio: f32, point: Pos2) -> Pos2 {
    pos2(
        point.x * scale_ratio + top_left.x,
        point.y * scale_ratio + top_left.y,
    )
}

pub fn obscure_screen(painter: &Painter, except_rectangle: Rect) {
    // todo: there are two white vertical lines to be removed
    painter.rect_filled(
        {
            let mut rect = painter.clip_rect();
            rect.set_right(except_rectangle.left());
            rect
        },
        Rounding::none(),
        Color32::from_black_alpha(200),
    );
    painter.rect_filled(
        {
            let mut rect = painter.clip_rect();
            rect.set_bottom(except_rectangle.top());
            rect.set_left(except_rectangle.left());
            rect.set_right(except_rectangle.right());
            rect
        },
        Rounding::none(),
        Color32::from_black_alpha(200),
    );
    painter.rect_filled(
        {
            let mut rect = painter.clip_rect();
            rect.set_left(except_rectangle.right());
            rect
        },
        Rounding::none(),
        Color32::from_black_alpha(200),
    );
    painter.rect_filled(
        {
            let mut rect = painter.clip_rect();
            rect.set_top(except_rectangle.bottom());
            rect.set_left(except_rectangle.left());
            rect.set_right(except_rectangle.right());
            rect
        },
        Rounding::none(),
        Color32::from_black_alpha(200),
    );
    painter.rect_stroke(
        except_rectangle,
        Rounding::none(),
        Stroke::new(3.0, Color32::RED),
    );
}
