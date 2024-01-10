use eframe::egui::color_picker::Alpha;
use eframe::egui::{
    color_picker, pos2, Color32, Context, CursorIcon, DragValue, Painter, Pos2, Rect, Rounding,
    Shape, Stroke, Ui, Vec2,
};
use eframe::emath::Rot2;
use eframe::epaint::{CircleShape, RectShape};
use image::{Rgba, RgbaImage};
use imageproc::drawing::{
    draw_filled_circle_mut, draw_filled_rect_mut, draw_hollow_circle_mut, draw_hollow_rect_mut,
    draw_polygon_mut, Blend,
};
use imageproc::point::Point;

/// definisce una direzione (di ridimensionamento, per il ritaglio)
#[derive(PartialEq, Debug)]
pub enum Direction {
    Top,
    Bottom,
    Left,
    Right,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

/// trasforma un punto sulla finestra in un punto rispetto alle dimensioni reali dell'immagine
pub fn unscaled_point(top_left: Pos2, scale_ratio: f32, point: Pos2) -> Pos2 {
    pos2(
        (point.x - top_left.x) / scale_ratio,
        (point.y - top_left.y) / scale_ratio,
    )
}

/// trasforma un rettangolo sulla finestra in un rettangolo rispetto alle dimensioni reali dell'immagine
pub fn unscaled_rect(top_left: Pos2, scale_ratio: f32, rect: Rect) -> Rect {
    Rect::from_two_pos(
        unscaled_point(top_left, scale_ratio, rect.left_top()),
        unscaled_point(top_left, scale_ratio, rect.right_bottom()),
    )
}

/// trasforma un rettangolo rispetto alle dimensioni dell'immagine in un rettangolo scalato sulle dimensioni della
/// finestra
pub fn scaled_rect(top_left: Pos2, scale_ratio: f32, rect: Rect) -> Rect {
    Rect::from_two_pos(
        scaled_point(top_left, scale_ratio, rect.left_top()),
        scaled_point(top_left, scale_ratio, rect.right_bottom()),
    )
}

/// trasforma un punto rispetto alle dimensioni dell'immagine in un punto scalato sulle dimensioni della finestra
pub fn scaled_point(top_left: Pos2, scale_ratio: f32, point: Pos2) -> Pos2 {
    pos2(
        point.x * scale_ratio + top_left.x,
        point.y * scale_ratio + top_left.y,
    )
}

/// dato un segmento definito come due punti nello spazio + uno spessore, la funzione ritorna i 4 punti di un
/// rettangolo che rappresenta lo stesso segmento.
pub fn line_width_to_polygon(points: &[Pos2; 2], width: f32) -> [Point<i32>; 4] {
    // todo: can I obtain this without using sqrt?
    let x1 = points[0].x;
    let x2 = points[1].x;
    let y1 = points[0].y;
    let y2 = points[1].y;

    let segment_length = ((x2 - x1) * (x2 - x1) + (y2 - y1) * (y2 - y1)).sqrt();
    let delta_x = width * (y2 - y1) / segment_length;
    let delta_y = width * (x2 - x1) / segment_length;

    [
        Point::new((x1 + delta_x) as i32, (y1 - delta_y) as i32),
        Point::new((x1 - delta_x) as i32, (y1 + delta_y) as i32),
        Point::new((x2 - delta_x) as i32, (y2 + delta_y) as i32),
        Point::new((x2 + delta_x) as i32, (y2 - delta_y) as i32),
    ]
}

/// la funzione trasforma un rettangolo degenere (con dimensioni negative) nel rettangolo identico ma con dimensioni
/// positive
pub fn make_rect_legal(rect: &mut Rect) {
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

/// oscura lo schermo eccetto l'area ritagliata. Lo spessore e il colore del bordo possono essere personalizzati con il
/// parametro stroke
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

/// disegna l'interfaccia che definisce il colore del tratto e il suo spessore
pub fn stroke_ui_opaque(ui: &mut Ui, stroke: &mut Stroke) {
    let Stroke { width, color } = stroke;
    ui.horizontal(|ui| {
        ui.label("Color:");
        color_picker::color_edit_button_srgba(ui, color, Alpha::Opaque);

        ui.label("Width:");
        ui.add(DragValue::new(width).speed(0.1).clamp_range(1.0..=5.0))
            .on_hover_text("Width");
        // stroke preview:
        let (_id, stroke_rect) = ui.allocate_space(ui.spacing().interact_size);
        ui.painter().line_segment(
            [stroke_rect.left_center(), stroke_rect.right_center()],
            (*width, *color),
        );
    });
}

/// crea un cerchio (o una circonferenza) in scala rispetto alle dimensioni effettive dell'immagine
pub fn create_circle(
    filled: bool,
    scale_ratio: f32,
    stroke: Stroke,
    top_left: Pos2,
    start_drag: Pos2,
    end_drag: Pos2,
) -> Shape {
    let center = start_drag;
    let radius = start_drag.distance(end_drag);
    if filled {
        Shape::Circle(CircleShape::filled(
            unscaled_point(top_left, scale_ratio, center),
            radius / scale_ratio,
            stroke.color,
        ))
    } else {
        Shape::Circle(CircleShape::stroke(
            unscaled_point(top_left, scale_ratio, center),
            radius / scale_ratio,
            Stroke::new(stroke.width / scale_ratio, stroke.color),
        ))
    }
}

/// crea un rettangolo (colorato o solo bordo) in scala rispetto alle dimensioni effettive dell'immagine
pub fn create_rect(
    filled: bool,
    scale_ratio: f32,
    stroke: Stroke,
    top_left: Pos2,
    start_drag: Pos2,
    end_drag: Pos2,
) -> Shape {
    if filled {
        // todo: there is a bug in the width that seems to not be positive.
        Shape::Rect(RectShape::filled(
            unscaled_rect(
                top_left,
                scale_ratio,
                Rect::from_two_pos(start_drag, end_drag),
            ),
            Rounding::none(),
            stroke.color,
        ))
    } else {
        Shape::Rect(RectShape::stroke(
            unscaled_rect(
                top_left,
                scale_ratio,
                Rect::from_two_pos(start_drag, end_drag),
            ),
            Rounding::none(),
            Stroke::new(stroke.width / scale_ratio, stroke.color),
        ))
    }
}

/// crea una freccia in scala rispetto all'immagine, che viene inserita nel gruppo di annotazioni come un insieme di
/// tre segmenti
pub fn push_arrow_into_annotations(
    annotations: &mut Vec<Shape>,
    scale_ratio: f32,
    stroke: Stroke,
    top_left: Pos2,
    start_drag: Pos2,
    end_drag: Pos2,
) {
    let vec = end_drag - start_drag;
    let origin = start_drag;
    let rot = Rot2::from_angle(std::f32::consts::TAU / 10.0);
    let tip_length = vec.length() / 4.0;
    let tip = origin + vec;
    let dir = vec.normalized();
    annotations.push(Shape::LineSegment {
        points: [
            unscaled_point(top_left, scale_ratio, origin),
            unscaled_point(top_left, scale_ratio, tip),
        ],
        stroke: Stroke::new(stroke.width / scale_ratio, stroke.color),
    });
    annotations.push(Shape::LineSegment {
        points: [
            unscaled_point(top_left, scale_ratio, tip),
            unscaled_point(top_left, scale_ratio, tip - tip_length * (rot * dir)),
        ],
        stroke: Stroke::new(stroke.width / scale_ratio, stroke.color),
    });
    annotations.push(Shape::LineSegment {
        points: [
            unscaled_point(top_left, scale_ratio, tip),
            unscaled_point(
                top_left,
                scale_ratio,
                tip - tip_length * (rot.inverse() * dir),
            ),
        ],
        stroke: Stroke::new(stroke.width / scale_ratio, stroke.color),
    });
}

/// ridimensiona il rettangolo in base alla direzione di ridimensionamento. A seconda della direzione in cui si sta
/// ridimensionando il rettangolo, viene settato il bordo giusto alla posizione corrente del cursore.
pub fn resize_rectangle(
    mut rectangle: Rect,
    hover_pos: Pos2,
    scale_ratio: f32,
    top_left: Pos2,
    direction: &mut Direction,
) -> Rect {
    match direction {
        Direction::Top => {
            rectangle.set_top(unscaled_point(top_left, scale_ratio, hover_pos).y);
        }
        Direction::Bottom => {
            rectangle.set_bottom(unscaled_point(top_left, scale_ratio, hover_pos).y);
        }
        Direction::Left => {
            rectangle.set_left(unscaled_point(top_left, scale_ratio, hover_pos).x);
        }
        Direction::Right => {
            rectangle.set_right(unscaled_point(top_left, scale_ratio, hover_pos).x);
        }
        Direction::TopLeft => {
            let point = unscaled_point(top_left, scale_ratio, hover_pos);
            rectangle.set_top(point.y);
            rectangle.set_left(point.x);
        }
        Direction::TopRight => {
            let point = unscaled_point(top_left, scale_ratio, hover_pos);
            rectangle.set_top(point.y);
            rectangle.set_right(point.x);
        }
        Direction::BottomLeft => {
            let point = unscaled_point(top_left, scale_ratio, hover_pos);
            rectangle.set_bottom(point.y);
            rectangle.set_left(point.x);
        }
        Direction::BottomRight => {
            let point = unscaled_point(top_left, scale_ratio, hover_pos);
            rectangle.set_bottom(point.y);
            rectangle.set_right(point.x);
        }
    }
    rectangle
}

/// In base alla direzione, viene impostato il cursore in ridimensionamento
pub fn set_cursor(direction: &Direction, ctx: &Context) {
    match direction {
        Direction::Top | Direction::Bottom => {
            ctx.set_cursor_icon(CursorIcon::ResizeVertical);
        }
        Direction::Left | Direction::Right => {
            ctx.set_cursor_icon(CursorIcon::ResizeHorizontal);
        }
        Direction::TopLeft => {
            ctx.set_cursor_icon(CursorIcon::ResizeNorthWest);
        }
        Direction::TopRight => {
            ctx.set_cursor_icon(CursorIcon::ResizeNorthEast);
        }
        Direction::BottomLeft => {
            ctx.set_cursor_icon(CursorIcon::ResizeSouthWest);
        }
        Direction::BottomRight => {
            ctx.set_cursor_icon(CursorIcon::ResizeSouthEast);
        }
    }
}

/// sulla base di una tolleranza (sarebbe troppo difficile allineare perfettamente il cursore su un bordo di un pixel),
/// se il cursore viene spostato sul bordo del rettangolo, viene modificato per rispecchiare la direzione di
/// ridimensionamento.
pub fn hover_to_direction(
    borders_rect: Rect,
    hover_pos: Pos2,
    cursor_tolerance: f32,
) -> Option<Direction> {
    // top-left corner of the rectangle
    if Rect::from_center_size(borders_rect.left_top(), Vec2::splat(cursor_tolerance * 2.0))
        .contains(hover_pos)
    {
        Some(Direction::TopLeft)
    }
    // top-right corner of the rectangle
    else if Rect::from_center_size(
        borders_rect.right_top(),
        Vec2::splat(cursor_tolerance * 2.0),
    )
    .contains(hover_pos)
    {
        Some(Direction::TopRight)
    }
    // bottom-left corner of the rectangle
    else if Rect::from_center_size(
        borders_rect.left_bottom(),
        Vec2::splat(cursor_tolerance * 2.0),
    )
    .contains(hover_pos)
    {
        Some(Direction::BottomLeft)
    }
    // bottom-right corner of the rectangle
    else if Rect::from_center_size(
        borders_rect.right_bottom(),
        Vec2::splat(cursor_tolerance * 2.0),
    )
    .contains(hover_pos)
    {
        Some(Direction::BottomRight)
    }
    // right segment of the rectangle
    else if hover_pos.x >= borders_rect.right() - cursor_tolerance
        && hover_pos.x <= borders_rect.right() + cursor_tolerance
        && hover_pos.y >= borders_rect.top()
        && hover_pos.y <= borders_rect.bottom()
    {
        Some(Direction::Right)
    }
    // left segment of the rectangle
    else if hover_pos.x >= borders_rect.left() - cursor_tolerance
        && hover_pos.x <= borders_rect.left() + cursor_tolerance
        && hover_pos.y >= borders_rect.top()
        && hover_pos.y <= borders_rect.bottom()
    {
        Some(Direction::Left)
    }
    // top segment of the rectangle
    else if hover_pos.y >= borders_rect.top() - cursor_tolerance
        && hover_pos.y <= borders_rect.top() + cursor_tolerance
        && hover_pos.x >= borders_rect.left()
        && hover_pos.x <= borders_rect.right()
    {
        Some(Direction::Top)
    }
    // bottom segment of the rectangle
    else if hover_pos.y >= borders_rect.bottom() - cursor_tolerance
        && hover_pos.y <= borders_rect.bottom() + cursor_tolerance
        && hover_pos.x >= borders_rect.left()
        && hover_pos.x <= borders_rect.right()
    {
        Some(Direction::Bottom)
    } else {
        None
    }
}

/// scala l'annotazione (cerchio, segmento, quadrato) sulle dimensioni della finestra
pub fn scale_annotation(annotation: &mut Shape, scale_ratio: f32, top_left: Pos2) {
    match annotation {
        Shape::Rect(rect_shape) => {
            rect_shape.rect = scaled_rect(top_left, scale_ratio, rect_shape.rect);
            rect_shape.stroke.width *= scale_ratio;
        }
        Shape::Circle(circle_shape) => {
            circle_shape.center = scaled_point(top_left, scale_ratio, circle_shape.center);
            circle_shape.radius *= scale_ratio;
            circle_shape.stroke.width *= scale_ratio;
        }
        Shape::LineSegment { points, stroke } => {
            for point in points {
                *point = scaled_point(top_left, scale_ratio, *point);
            }
            stroke.width *= scale_ratio;
        }
        Shape::Path(path_shape) => {
            path_shape.stroke.width *= scale_ratio;
            for point in &mut path_shape.points {
                *point = scaled_point(top_left, scale_ratio, *point);
            }
        }
        // todo: set description of reachability
        _ => unreachable!(),
    }
}

/// prima di passare al salvataggio dell'immagine su memoria di massa, le annotazioni vengono scritte sull'immagine, in
/// memoria RAM
pub fn write_annotation_to_image(annotation: &Shape, image_blend: &mut Blend<RgbaImage>) {
    match annotation {
        Shape::Rect(rect_shape) => {
            write_rectangle_with_width(image_blend, rect_shape);
        }
        Shape::Path(path_shape) => {
            for segment in path_shape
                .points
                .iter()
                .zip(path_shape.points.iter().skip(1))
            {
                let polygon_points =
                    line_width_to_polygon(&[*segment.0, *segment.1], path_shape.stroke.width / 2.0);
                if !(polygon_points[0] == polygon_points[polygon_points.len() - 1]) {
                    draw_polygon_mut(
                        image_blend,
                        &polygon_points,
                        Rgba(path_shape.stroke.color.to_array()),
                    );
                }
            }
        }
        Shape::LineSegment { points, stroke } => draw_polygon_mut(
            image_blend,
            &line_width_to_polygon(points, stroke.width / 2.0),
            Rgba(stroke.color.to_array()),
        ),
        Shape::Circle(circle_shape) => {
            write_circle_with_width(image_blend, circle_shape);
        }
        _ => {
            unreachable!("These are the only shapes which have to be used")
        }
    }
}

fn write_rectangle_with_width(image_blend: &mut Blend<RgbaImage>, rect_shape: &RectShape) {
    draw_filled_rect_mut(
        image_blend,
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
    let big_rect = rect_shape.rect.expand(rect_shape.stroke.width / 2.0);
    for delta in 0..rect_shape.stroke.width as i32 {
        let rect = big_rect.shrink(delta as f32);
        draw_hollow_rect_mut(
            image_blend,
            imageproc::rect::Rect::at(rect.left_top().x as i32, rect.left_top().y as i32)
                .of_size(rect.width() as u32, rect.height() as u32),
            Rgba(rect_shape.stroke.color.to_array()),
        );
    }
}

fn write_circle_with_width(image_blend: &mut Blend<RgbaImage>, circle_shape: &CircleShape) {
    draw_filled_circle_mut(
        image_blend,
        (circle_shape.center.x as i32, circle_shape.center.y as i32),
        circle_shape.radius as i32,
        Rgba::from(circle_shape.fill.to_array()),
    );
    let interval = (circle_shape.radius - circle_shape.stroke.width / 2.0) as i32
        ..(circle_shape.radius + circle_shape.stroke.width / 2.0) as i32;
    for radius in interval {
        draw_hollow_circle_mut(
            image_blend,
            (circle_shape.center.x as i32, circle_shape.center.y as i32),
            radius,
            Rgba(circle_shape.stroke.color.to_array()),
        );
    }
}
