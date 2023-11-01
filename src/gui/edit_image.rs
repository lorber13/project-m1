// DISCLAIMER: THIS CODE IS MESSY, I KNOW. I STILL HAVE TO MODULARIZE IT
use crate::image_coding::ImageFormat;
use eframe::egui::{
    pos2, stroke_ui, vec2, CentralPanel, Color32, ColorImage, Context, Painter, Pos2, Rect,
    Response, Rounding, Sense, Shape, Stroke, TextureHandle, Ui, Vec2,
};
use eframe::egui::{ComboBox, CursorIcon};
use eframe::emath::Rot2;
use eframe::epaint::{CircleShape, RectShape};
use image::imageops::crop_imm;
use image::{Rgba, RgbaImage};
use imageproc::drawing::{
    draw_filled_circle_mut, draw_filled_rect_mut, draw_hollow_circle_mut, draw_hollow_rect_mut,
    draw_line_segment_mut, Blend,
};
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

#[derive(PartialEq, Debug)]
enum Direction {
    Top,
    Bottom,
    Left,
    Right,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
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
            Default::default(),
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
                    rect_shape.rect = scaled_rect(
                        painter.clip_rect().left_top(),
                        self.scale_ratio,
                        rect_shape.rect,
                    );
                    rect_shape.stroke.width *= self.scale_ratio;
                }
                Shape::Circle(circle_shape) => {
                    circle_shape.center = scaled_point(
                        painter.clip_rect().left_top(),
                        self.scale_ratio,
                        circle_shape.center,
                    );
                    circle_shape.radius *= self.scale_ratio;
                    circle_shape.stroke.width *= self.scale_ratio;
                }
                Shape::LineSegment { points, stroke } => {
                    for point in points {
                        *point =
                            scaled_point(painter.clip_rect().left_top(), self.scale_ratio, *point);
                    }
                    stroke.width *= self.scale_ratio;
                }
                Shape::Path(path_shape) => {
                    path_shape.stroke.width *= self.scale_ratio;
                    for point in path_shape.points.iter_mut() {
                        *point =
                            scaled_point(painter.clip_rect().left_top(), self.scale_ratio, *point);
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
                            line.push(ctx.pointer_hover_pos().expect(
                                "should not panic because while dragging the pointer exists",
                            ));
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
                        obscure_screen(
                            &painter,
                            scaled_rect(
                                painter.clip_rect().left_top(),
                                self.scale_ratio,
                                self.cut_rect,
                            ),
                            Stroke::new(1.0, Color32::WHITE),
                        );
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
                            let center =
                                start_drag.expect("if we are here start_drag should be defined");
                            let radius = start_drag
                                .expect("if we are here start_drag should be defined")
                                .distance(end_drag.expect("the previous assertion"));
                            if self.fill_shape {
                                painter.circle_filled(center, radius, self.stroke.color);
                            } else {
                                painter.circle_stroke(center, radius, self.stroke);
                            }
                        } else if response.drag_released() {
                            let center =
                                start_drag.expect("if we are here start_drag should be defined");
                            let radius = start_drag
                                .expect("if we are here start_drag should be defined")
                                .distance(end_drag.expect("the previous assertion"));
                            if self.fill_shape {
                                painter.circle_filled(center, radius, self.stroke.color);
                                self.annotations.push(Shape::Circle(CircleShape::filled(
                                    unscaled_point(
                                        painter.clip_rect().left_top(),
                                        self.scale_ratio,
                                        center,
                                    ),
                                    radius / self.scale_ratio,
                                    self.stroke.color,
                                )));
                            } else {
                                painter.circle_stroke(center, radius, self.stroke);
                                self.annotations.push(Shape::Circle(CircleShape::stroke(
                                    unscaled_point(
                                        painter.clip_rect().left_top(),
                                        self.scale_ratio,
                                        center,
                                    ),
                                    radius / self.scale_ratio,
                                    Stroke::new(
                                        self.stroke.width / self.scale_ratio,
                                        self.stroke.color,
                                    ),
                                )));
                            }
                        }
                        obscure_screen(
                            &painter,
                            scaled_rect(
                                painter.clip_rect().left_top(),
                                self.scale_ratio,
                                self.cut_rect,
                            ),
                            Stroke::new(1.0, Color32::WHITE),
                        );
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
                            let a =
                                start_drag.expect("if we are here start_drag should be defined");
                            let b = end_drag.expect("the previous assertion");
                            if self.fill_shape {
                                painter.rect_filled(
                                    Rect::from_two_pos(a, b),
                                    Rounding::none(),
                                    self.stroke.color,
                                );
                            } else {
                                painter.rect_stroke(
                                    Rect::from_two_pos(a, b),
                                    Rounding::none(),
                                    self.stroke,
                                );
                            }
                        } else if response.drag_released() {
                            let a =
                                start_drag.expect("if we are here start_drag should be defined");
                            let b = end_drag.expect("the previous assertion");
                            if self.fill_shape {
                                painter.rect_filled(
                                    Rect::from_two_pos(a, b),
                                    Rounding::none(),
                                    self.stroke.color,
                                );
                                self.annotations.push(Shape::Rect(RectShape::filled(
                                    unscaled_rect(
                                        painter.clip_rect().left_top(),
                                        self.scale_ratio,
                                        Rect::from_two_pos(a, b),
                                    ),
                                    Rounding::none(),
                                    self.stroke.color,
                                )));
                            } else {
                                painter.rect_stroke(
                                    Rect::from_two_pos(a, b),
                                    Rounding::none(),
                                    self.stroke,
                                );
                                self.annotations.push(Shape::Rect(RectShape::stroke(
                                    unscaled_rect(
                                        painter.clip_rect().left_top(),
                                        self.scale_ratio,
                                        Rect::from_two_pos(a, b),
                                    ),
                                    Rounding::none(),
                                    Stroke::new(
                                        self.stroke.width / self.scale_ratio,
                                        self.stroke.color,
                                    ),
                                )))
                            }
                        }
                        obscure_screen(
                            &painter,
                            scaled_rect(
                                painter.clip_rect().left_top(),
                                self.scale_ratio,
                                self.cut_rect,
                            ),
                            Stroke::new(1.0, Color32::WHITE),
                        );
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
                            painter.arrow(
                                start_drag.expect("if we are here start_drag should be defined"),
                                end_drag.expect("the previous assertion").sub(
                                    start_drag
                                        .expect("if we are here start_drag should be defined"),
                                ),
                                self.stroke,
                            );
                        } else if response.drag_released() {
                            let vec = end_drag.expect("the previous assertion").sub(
                                start_drag.expect("if we are here start_drag should be defined"),
                            );
                            let origin =
                                start_drag.expect("if we are here start_drag should be defined");
                            painter.arrow(origin, vec, self.stroke);
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
                        obscure_screen(
                            &painter,
                            scaled_rect(
                                painter.clip_rect().left_top(),
                                self.scale_ratio,
                                self.cut_rect,
                            ),
                            Stroke::new(1.0, Color32::WHITE),
                        );
                    }
                    // todo: while dragging, the rectangle must not become a negative rectangle
                    Tool::Cut { modifying } => {
                        obscure_screen(
                            &painter,
                            scaled_rect(
                                painter.clip_rect().left_top(),
                                self.scale_ratio,
                                self.cut_rect,
                            ),
                            Stroke::new(3.0, Color32::YELLOW),
                        );
                        match modifying {
                            ModificationOfRectangle::Move => {
                                if response.dragged() {
                                    ctx.set_cursor_icon(CursorIcon::Grabbing);
                                    // todo: work in painter dimensions, not real dimensions
                                    // todo: refine the function that makes the rectangle not escape borders
                                    let image_rect = Rect::from_min_size(
                                        pos2(0.0, 0.0),
                                        self.texture_handle.size_vec2(),
                                    );
                                    let unscaled_delta = response.drag_delta() / self.scale_ratio;
                                    let translated_rect = self.cut_rect.translate(unscaled_delta);
                                    if image_rect.contains_rect(translated_rect) {
                                        self.cut_rect = translated_rect;
                                    } else {
                                        self.cut_rect = translated_rect.translate({
                                            let mut vec = Vec2::default();
                                            if translated_rect.left() < image_rect.left() {
                                                vec.x = image_rect.left() - translated_rect.left();
                                            }
                                            if translated_rect.top() < image_rect.top() {
                                                vec.y = image_rect.top() - translated_rect.top();
                                            }
                                            if translated_rect.right() > image_rect.right() {
                                                vec.x =
                                                    image_rect.right() - translated_rect.right();
                                            }
                                            if translated_rect.bottom() > image_rect.bottom() {
                                                vec.y =
                                                    image_rect.bottom() - translated_rect.bottom();
                                            }
                                            vec
                                        });
                                    }
                                } else if response.drag_released() {
                                    *modifying = ModificationOfRectangle::NoModification;
                                }
                            }
                            ModificationOfRectangle::Resize { direction } => {
                                if response.dragged() {
                                    let hover_pos = ctx.pointer_hover_pos().expect(
                                        "while dragging the pointer position is never None",
                                    );
                                    match direction {
                                        Direction::Top => {
                                            ctx.set_cursor_icon(CursorIcon::ResizeVertical);
                                            self.cut_rect.set_top(
                                                unscaled_point(
                                                    painter.clip_rect().left_top(),
                                                    self.scale_ratio,
                                                    hover_pos,
                                                )
                                                .y,
                                            );
                                        }
                                        Direction::Bottom => {
                                            ctx.set_cursor_icon(CursorIcon::ResizeVertical);
                                            self.cut_rect.set_bottom(
                                                unscaled_point(
                                                    painter.clip_rect().left_top(),
                                                    self.scale_ratio,
                                                    hover_pos,
                                                )
                                                .y,
                                            );
                                        }
                                        Direction::Left => {
                                            ctx.set_cursor_icon(CursorIcon::ResizeHorizontal);
                                            self.cut_rect.set_left(
                                                unscaled_point(
                                                    painter.clip_rect().left_top(),
                                                    self.scale_ratio,
                                                    hover_pos,
                                                )
                                                .x,
                                            );
                                        }
                                        Direction::Right => {
                                            ctx.set_cursor_icon(CursorIcon::ResizeHorizontal);
                                            self.cut_rect.set_right(
                                                unscaled_point(
                                                    painter.clip_rect().left_top(),
                                                    self.scale_ratio,
                                                    hover_pos,
                                                )
                                                .x,
                                            );
                                        }
                                        Direction::TopLeft => {
                                            let point = unscaled_point(
                                                painter.clip_rect().left_top(),
                                                self.scale_ratio,
                                                hover_pos,
                                            );
                                            ctx.set_cursor_icon(CursorIcon::ResizeNorthWest);
                                            self.cut_rect.set_top(point.y);
                                            self.cut_rect.set_left(point.x);
                                        }
                                        Direction::TopRight => {
                                            let point = unscaled_point(
                                                painter.clip_rect().left_top(),
                                                self.scale_ratio,
                                                hover_pos,
                                            );
                                            ctx.set_cursor_icon(CursorIcon::ResizeNorthEast);
                                            self.cut_rect.set_top(point.y);
                                            self.cut_rect.set_right(point.x);
                                        }
                                        Direction::BottomLeft => {
                                            let point = unscaled_point(
                                                painter.clip_rect().left_top(),
                                                self.scale_ratio,
                                                hover_pos,
                                            );
                                            ctx.set_cursor_icon(CursorIcon::ResizeSouthWest);
                                            self.cut_rect.set_bottom(point.y);
                                            self.cut_rect.set_left(point.x);
                                        }
                                        Direction::BottomRight => {
                                            let point = unscaled_point(
                                                painter.clip_rect().left_top(),
                                                self.scale_ratio,
                                                hover_pos,
                                            );
                                            ctx.set_cursor_icon(CursorIcon::ResizeSouthEast);
                                            self.cut_rect.set_bottom(point.y);
                                            self.cut_rect.set_right(point.x);
                                        }
                                    }
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
                                        painter.clip_rect().left_top(),
                                        self.scale_ratio,
                                        self.cut_rect,
                                    );
                                    let cursor_tolerance = 10.0; // todo: should the tolerance be calculated depending on the display's number of pixels?

                                    // top-left corner of the rectangle
                                    if Rect::from_center_size(
                                        rect.left_top(),
                                        Vec2::splat(cursor_tolerance * 2.0),
                                    )
                                    .contains(pos)
                                    {
                                        ctx.set_cursor_icon(CursorIcon::ResizeNorthWest);
                                        if response.drag_started() {
                                            *modifying = ModificationOfRectangle::Resize {
                                                direction: Direction::TopLeft,
                                            };
                                        }
                                    }
                                    // top-right corner of the rectangle
                                    else if Rect::from_center_size(
                                        rect.right_top(),
                                        Vec2::splat(cursor_tolerance * 2.0),
                                    )
                                    .contains(pos)
                                    {
                                        ctx.set_cursor_icon(CursorIcon::ResizeNorthEast);
                                        if response.drag_started() {
                                            *modifying = ModificationOfRectangle::Resize {
                                                direction: Direction::TopRight,
                                            };
                                        }
                                    }
                                    // bottom-left corner of the rectangle
                                    else if Rect::from_center_size(
                                        rect.left_bottom(),
                                        Vec2::splat(cursor_tolerance * 2.0),
                                    )
                                    .contains(pos)
                                    {
                                        ctx.set_cursor_icon(CursorIcon::ResizeSouthWest);
                                        if response.drag_started() {
                                            *modifying = ModificationOfRectangle::Resize {
                                                direction: Direction::BottomLeft,
                                            };
                                        }
                                    }
                                    // bottom-right corner of the rectangle
                                    else if Rect::from_center_size(
                                        rect.right_bottom(),
                                        Vec2::splat(cursor_tolerance * 2.0),
                                    )
                                    .contains(pos)
                                    {
                                        ctx.set_cursor_icon(CursorIcon::ResizeSouthEast);
                                        if response.drag_started() {
                                            *modifying = ModificationOfRectangle::Resize {
                                                direction: Direction::BottomRight,
                                            };
                                        }
                                    }
                                    // right segment of the rectangle
                                    else if pos.x >= rect.right() - cursor_tolerance
                                        && pos.x <= rect.right() + cursor_tolerance
                                        && pos.y >= rect.top()
                                        && pos.y <= rect.bottom()
                                    {
                                        // todo: manage equivalence between f32. Is round() sufficient?
                                        ctx.set_cursor_icon(CursorIcon::ResizeHorizontal);
                                        if response.drag_started() {
                                            *modifying = ModificationOfRectangle::Resize {
                                                direction: Direction::Right,
                                            };
                                        }
                                    }
                                    // left segment of the rectangle
                                    else if pos.x >= rect.left() - cursor_tolerance
                                        && pos.x <= rect.left() + cursor_tolerance
                                        && pos.y >= rect.top()
                                        && pos.y <= rect.bottom()
                                    {
                                        ctx.set_cursor_icon(CursorIcon::ResizeHorizontal);
                                        if response.drag_started() {
                                            *modifying = ModificationOfRectangle::Resize {
                                                direction: Direction::Left,
                                            };
                                        }
                                    }
                                    // top segment of the rectangle
                                    else if pos.y >= rect.top() - cursor_tolerance
                                        && pos.y <= rect.top() + cursor_tolerance
                                        && pos.x >= rect.left()
                                        && pos.x <= rect.right()
                                    {
                                        ctx.set_cursor_icon(CursorIcon::ResizeVertical);
                                        if response.drag_started() {
                                            *modifying = ModificationOfRectangle::Resize {
                                                direction: Direction::Top,
                                            };
                                        }
                                    }
                                    // bottom segment of the rectangle
                                    else if pos.y >= rect.bottom() - cursor_tolerance
                                        && pos.y <= rect.bottom() + cursor_tolerance
                                        && pos.x >= rect.left()
                                        && pos.x <= rect.right()
                                    {
                                        ctx.set_cursor_icon(CursorIcon::ResizeVertical);
                                        if response.drag_started() {
                                            *modifying = ModificationOfRectangle::Resize {
                                                direction: Direction::Bottom,
                                            };
                                        }
                                    }
                                    // moving of the rectangle
                                    else if rect.contains(pos) {
                                        ctx.set_cursor_icon(CursorIcon::Grab);
                                        if response.drag_started() {
                                            *modifying = ModificationOfRectangle::Move;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            });
        });
        ret
    }

    fn draw_menu_buttons(&mut self, ret: &mut EditImageEvent, ui: &mut Ui) {
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
                    ui.selectable_value(&mut self.format, ImageFormat::Png, "Png");
                    ui.selectable_value(&mut self.format, ImageFormat::JPEG, "Jpeg");
                    ui.selectable_value(&mut self.format, ImageFormat::GIF, "Gif");
                });
            if ui.button("Save").clicked() {
                /*
                todo: there are a number of problems:
                      1. the stroke in the imageproc crate is difficult to be defined
                      2. the circle, when using a transparent color (i.e. alpha not 255) appears striped
                      3. the color of annotations in the painter is a little different from the color of the image saved
                 */
                for annotation in &self.annotations {
                    match annotation {
                        Shape::Rect(rect_shape) => {
                            draw_filled_rect_mut(
                                &mut self.image_blend,
                                imageproc::rect::Rect::at(
                                    rect_shape.rect.left_top().x as i32,
                                    rect_shape.rect.left_top().y as i32,
                                )
                                .of_size(
                                    rect_shape.rect.width() as u32,
                                    rect_shape.rect.height() as u32,
                                ),
                                Rgba(rect_shape.fill.to_array()),
                            );
                            draw_hollow_rect_mut(
                                &mut self.image_blend,
                                imageproc::rect::Rect::at(
                                    rect_shape.rect.left_top().x as i32,
                                    rect_shape.rect.left_top().y as i32,
                                )
                                .of_size(
                                    rect_shape.rect.width() as u32,
                                    rect_shape.rect.height() as u32,
                                ),
                                Rgba(rect_shape.stroke.color.to_array()),
                            );
                        }
                        Shape::Path(path_shape) => {
                            for segment in path_shape
                                .points
                                .iter()
                                .zip(path_shape.points.iter().skip(1))
                            {
                                draw_line_segment_mut(
                                    &mut self.image_blend,
                                    (segment.0.x, segment.0.y),
                                    (segment.1.x, segment.1.y),
                                    Rgba(path_shape.stroke.color.to_array()),
                                )
                            }
                        }
                        Shape::LineSegment { points, stroke } => draw_line_segment_mut(
                            &mut self.image_blend,
                            (points[0].x, points[0].y),
                            (points[1].x, points[1].y),
                            Rgba(stroke.color.to_array()),
                        ),
                        Shape::Circle(circle_shape) => {
                            draw_filled_circle_mut(
                                &mut self.image_blend,
                                (circle_shape.center.x as i32, circle_shape.center.y as i32),
                                circle_shape.radius as i32,
                                Rgba::from(circle_shape.fill.to_array()),
                            );
                            draw_hollow_circle_mut(
                                &mut self.image_blend,
                                (circle_shape.center.x as i32, circle_shape.center.y as i32),
                                circle_shape.radius as i32,
                                Rgba(circle_shape.stroke.color.to_array()),
                            )
                        }
                        _ => {
                            unreachable!("These are the only shapes which have to be used")
                        }
                    }
                }
                *ret = EditImageEvent::Saved {
                    image: crop_imm(
                        &self.image_blend.0,
                        self.cut_rect.left_top().x as u32,
                        self.cut_rect.left_top().y as u32,
                        self.cut_rect.width() as u32,
                        self.cut_rect.height() as u32,
                    )
                    .to_image(),
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

fn scaled_rect(top_left: Pos2, scale_ratio: f32, rect: Rect) -> Rect {
    Rect::from_two_pos(
        scaled_point(top_left, scale_ratio, rect.left_top()),
        scaled_point(top_left, scale_ratio, rect.right_bottom()),
    )
}

fn scaled_point(top_left: Pos2, scale_ratio: f32, point: Pos2) -> Pos2 {
    pos2(
        point.x * scale_ratio + top_left.x,
        point.y * scale_ratio + top_left.y,
    )
}

fn make_rect_legal(rect: &mut Rect) {
    let width = rect.width();
    let height = rect.height();
    if width < 0.0 {
        rect.set_left(rect.left() + width);
        rect.set_right(rect.right() - width);
    }
    if height < 0.0 {
        rect.set_top(rect.top() + height);
        rect.set_bottom(rect.bottom() - height);
    }
}

pub fn obscure_screen(painter: &Painter, except_rectangle: Rect, stroke: Stroke) {
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
    painter.rect_stroke(except_rectangle, Rounding::none(), stroke);
}
