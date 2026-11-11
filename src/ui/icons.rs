use eframe::egui::{self, Color32, Pos2, Rect, Response, Sense, Shape, Stroke, Ui, pos2, vec2};

#[derive(Clone, Copy)]
pub enum Icon {
    Back,
    Forward,
    Panel,
    Home,
    Codex,
    Chat,
    Clock,
    Books,
    Grid,
    NewChat,
    Folder,
    Bell,
    Search,
    More,
    Agent,
    Bolt,
    Shield,
    Mic,
    ArrowUp,
    Copy,
    Minimize,
    Maximize,
    Close,
    Chevron,
    Settings,
    Reticle,
}

pub fn button(ui: &mut Ui, icon: Icon, tooltip: &str) -> Response {
    let response = button_with_sense(ui, icon, Sense::click()).on_hover_text(tooltip);
    #[cfg(feature = "design-export")]
    if let Some(action) = redox_studio::actions::UiAction::from_label(tooltip) {
        super::design_export::record(ui.ctx(), response.rect, action);
    }
    response
}

pub(super) fn button_with_sense(ui: &mut Ui, icon: Icon, sense: Sense) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(30.0, 30.0), sense);
    if super::theme::is_wii(ui.ctx()) {
        super::wii::paint_surface(ui, &response, false);
    } else if response.hovered() && ui.is_enabled() {
        ui.painter().rect_filled(rect, 7, super::theme::surface());
    }
    let color = if !ui.is_enabled() {
        Color32::from_gray(85)
    } else if response.hovered() {
        super::theme::text()
    } else {
        super::theme::muted()
    };
    paint(ui.painter(), rect.shrink(7.0), icon, color);
    response
}

pub fn paint(painter: &egui::Painter, rect: Rect, icon: Icon, color: Color32) {
    let point = |x: f32, y: f32| {
        pos2(
            rect.left() + x / 24.0 * rect.width(),
            rect.top() + y / 24.0 * rect.height(),
        )
    };
    let stroke = Stroke::new(1.25_f32, color);
    let path = |points: &[(f32, f32)]| {
        painter.add(Shape::line(
            points
                .iter()
                .map(|(x, y)| point(*x, *y))
                .collect::<Vec<Pos2>>(),
            stroke,
        ));
    };
    let box_at = |a: (f32, f32), b: (f32, f32), rounding| {
        painter.rect_stroke(
            Rect::from_min_max(point(a.0, a.1), point(b.0, b.1)),
            rounding,
            stroke,
            egui::StrokeKind::Inside,
        );
    };

    match icon {
        Icon::Reticle => {
            for quadrant in 0..4 {
                let start = quadrant as f32 * std::f32::consts::FRAC_PI_2 + 0.2;
                let points = (0..14)
                    .map(|step| {
                        let angle = start + step as f32 / 13.0 * 1.15;
                        point(12.0 + angle.cos() * 10.0, 12.0 + angle.sin() * 10.0)
                    })
                    .collect();
                painter.add(Shape::line(points, stroke));
            }
            painter.add(Shape::convex_polygon(
                vec![
                    point(12., 6.),
                    point(18., 12.),
                    point(12., 18.),
                    point(6., 12.),
                ],
                color,
                Stroke::NONE,
            ));
        }
        Icon::Back => {
            path(&[(14., 5.), (7., 12.), (14., 19.)]);
            path(&[(7., 12.), (22., 12.)]);
        }
        Icon::Forward => {
            path(&[(10., 5.), (17., 12.), (10., 19.)]);
            path(&[(2., 12.), (17., 12.)]);
        }
        Icon::Panel => {
            box_at((3., 4.), (21., 20.), 3);
            path(&[(10., 4.), (10., 20.)]);
        }
        Icon::Codex => {
            let points = (0..=96)
                .map(|index| {
                    let angle = index as f32 / 96.0 * std::f32::consts::TAU;
                    let radius = 9.0 + 0.8 * (angle * 6.0).cos();
                    point(12.0 + radius * angle.cos(), 12.0 + radius * angle.sin())
                })
                .collect();
            painter.add(Shape::line(points, Stroke::new(2.0, color)));
            path(&[(7., 8.), (10., 12.), (7., 16.)]);
            path(&[(13., 16.), (18., 16.)]);
        }
        Icon::Home => {
            painter.add(Shape::convex_polygon(
                vec![
                    point(2., 10.),
                    point(12., 2.),
                    point(22., 10.),
                    point(20., 10.),
                    point(20., 21.),
                    point(14., 21.),
                    point(14., 14.),
                    point(10., 14.),
                    point(10., 21.),
                    point(4., 21.),
                    point(4., 10.),
                ],
                color,
                Stroke::NONE,
            ));
        }
        Icon::Chat => {
            box_at((3., 3.), (18., 17.), 3);
            path(&[(7., 17.), (4., 21.), (4., 17.)]);
            box_at((9., 8.), (23., 22.), 3);
        }
        Icon::Clock => {
            painter.circle_stroke(point(12., 12.), rect.width() * 0.43, stroke);
            path(&[(12., 6.), (12., 12.), (8., 15.)]);
        }
        Icon::Books => {
            box_at((2., 3.), (7., 21.), 1);
            box_at((9., 3.), (14., 21.), 1);
            path(&[(16., 4.), (20., 3.), (24., 20.), (20., 21.), (16., 4.)]);
        }
        Icon::Grid => {
            for (x, y) in [(3., 3.), (14., 3.), (3., 14.), (14., 14.)] {
                box_at((x, y), (x + 7., y + 7.), 2);
            }
        }
        Icon::NewChat => {
            path(&[
                (9., 4.),
                (5., 4.),
                (3., 6.),
                (3., 20.),
                (19., 20.),
                (21., 18.),
                (21., 13.),
            ]);
            path(&[
                (9., 15.),
                (10., 10.),
                (19., 1.),
                (23., 5.),
                (14., 14.),
                (9., 15.),
            ]);
        }
        Icon::Folder => {
            path(&[
                (2., 8.),
                (2., 5.),
                (4., 3.),
                (10., 3.),
                (13., 6.),
                (20., 6.),
                (22., 8.),
            ]);
            path(&[(2., 8.), (22., 8.), (19., 20.), (2., 20.), (2., 8.)]);
        }
        Icon::Bell => {
            path(&[
                (4., 17.),
                (7., 14.),
                (7., 8.),
                (9., 4.),
                (15., 4.),
                (17., 8.),
                (17., 14.),
                (20., 17.),
                (4., 17.),
            ]);
            path(&[(9., 20.), (15., 20.)]);
        }
        Icon::Search => {
            painter.circle_stroke(point(10., 10.), rect.width() * 0.32, stroke);
            path(&[(16., 16.), (22., 22.)]);
        }
        Icon::More => {
            for x in [5., 12., 19.] {
                painter.circle_filled(point(x, 12.), 1.25, color);
            }
        }
        Icon::Agent => {
            box_at((4., 7.), (20., 20.), 3);
            path(&[(12., 7.), (12., 3.)]);
            painter.circle_filled(point(12., 2.), 1.2, color);
            painter.circle_filled(point(8., 12.), 1.2, color);
            painter.circle_filled(point(16., 12.), 1.2, color);
            path(&[(8., 16.), (16., 16.)]);
            path(&[(1., 11.), (1., 16.)]);
            path(&[(23., 11.), (23., 16.)]);
        }
        Icon::Bolt => {
            path(&[
                (13., 2.),
                (5., 13.),
                (11., 13.),
                (10., 22.),
                (20., 9.),
                (13., 9.),
                (13., 2.),
            ]);
        }
        Icon::Shield => {
            path(&[
                (12., 2.),
                (21., 6.),
                (20., 16.),
                (12., 22.),
                (4., 16.),
                (3., 6.),
                (12., 2.),
            ]);
            path(&[(12., 7.), (12., 13.)]);
            painter.circle_filled(point(12., 17.), 0.8, color);
        }
        Icon::Mic => {
            box_at((9., 2.), (15., 15.), 3);
            path(&[
                (5., 10.),
                (5., 15.),
                (9., 19.),
                (15., 19.),
                (19., 15.),
                (19., 10.),
            ]);
            path(&[(12., 19.), (12., 23.)]);
        }
        Icon::ArrowUp => {
            path(&[(5., 11.), (12., 4.), (19., 11.)]);
            path(&[(12., 4.), (12., 21.)]);
        }
        Icon::Copy => {
            box_at((7., 3.), (21., 17.), 3);
            path(&[(6., 8.), (3., 8.), (3., 22.), (16., 22.), (16., 19.)]);
        }
        Icon::Minimize => path(&[(5., 12.), (19., 12.)]),
        Icon::Maximize => box_at((5., 5.), (19., 19.), 1),
        Icon::Close => {
            path(&[(5., 5.), (19., 19.)]);
            path(&[(19., 5.), (5., 19.)]);
        }
        Icon::Chevron => path(&[(6., 9.), (12., 15.), (18., 9.)]),
        Icon::Settings => {
            painter.circle_stroke(point(12., 12.), rect.width() * 0.35, stroke);
            painter.circle_stroke(point(12., 12.), rect.width() * 0.12, stroke);
            for (a, b) in [
                ((12., 0.), (12., 4.)),
                ((12., 20.), (12., 24.)),
                ((0., 12.), (4., 12.)),
                ((20., 12.), (24., 12.)),
            ] {
                path(&[a, b]);
            }
        }
    }
}
