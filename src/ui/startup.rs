use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, FontId, Pos2, Rect, Sense, Shape, Stroke, Vec2, pos2, vec2};
use serde::{Deserialize, Serialize};

#[path = "startup_settings.rs"]
mod settings;

#[derive(Clone, Deserialize, Serialize)]
#[serde(default)]
pub(in crate::app) struct Config {
    enabled: bool,
    duration_seconds: f32,
    fade_seconds: f32,
    cursor_count: usize,
    cursor_size: f32,
    show_names: bool,
    show_trails: bool,
    show_guides: bool,
    background: String,
    colors: Vec<String>,
    names: Vec<String>,
    name_choices: Vec<bool>,
    seed: u64,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            enabled: true,
            duration_seconds: 5.5,
            fade_seconds: 0.5,
            cursor_count: 4,
            cursor_size: 15.0,
            show_names: true,
            show_trails: true,
            show_guides: true,
            background: "theme".into(),
            colors: ["#8C69B8", "#ABD3D8"].map(str::to_owned).into(),
            names: ["SaffronCarp1106", "SackBoy19"].map(str::to_owned).into(),
            name_choices: vec![true, true],
            seed: 42,
        }
    }
}

impl Config {
    fn normalize(&mut self) {
        self.duration_seconds = finite(self.duration_seconds, 5.5).clamp(0.8, 20.0);
        self.fade_seconds = finite(self.fade_seconds, 0.5).clamp(0.1, self.duration_seconds * 0.5);
        self.cursor_size = finite(self.cursor_size, 15.0).clamp(12.0, 42.0);
        self.cursor_count = self.cursor_count.clamp(1, 32);
        self.colors.retain(|color| parse_color(color).is_some());
        self.colors.truncate(32);
        if self.colors.is_empty() {
            self.colors = Self::default().colors;
        }
        self.name_choices.resize(self.names.len(), true);
        let names = std::mem::take(&mut self.names);
        let choices = std::mem::take(&mut self.name_choices);
        for (name, selected) in names.into_iter().zip(choices).take(32) {
            let name: String = name.trim().chars().take(24).collect();
            if !name.is_empty() {
                self.names.push(name);
                self.name_choices.push(selected);
            }
        }
        if self.names.is_empty() {
            self.names = Self::default().names;
            self.name_choices = vec![true; self.names.len()];
        }
        if !self.name_choices.iter().any(|selected| *selected) {
            self.name_choices[0] = true;
        }
    }

    fn selected_names(&self) -> Vec<&str> {
        self.names
            .iter()
            .enumerate()
            .filter(|(index, name)| {
                self.name_choices.get(*index).copied().unwrap_or(true) && !name.trim().is_empty()
            })
            .map(|(_, name)| name.as_str())
            .collect()
    }

    fn load() -> Self {
        let path = config_path();
        let source =
            std::fs::read_to_string(&path).unwrap_or_else(|_| include_str!("startup.json").into());
        let mut config: Self = serde_json::from_str(&source).unwrap_or_else(|error| {
            eprintln!(
                "Startup animation config {}: {error}. Using defaults.",
                path.display()
            );
            Self::default()
        });
        config.normalize();
        config
    }
}

fn config_path() -> PathBuf {
    if let Some(path) = std::env::var_os("CODEX_RUST_STARTUP_CONFIG") {
        return path.into();
    }
    if let Ok(executable) = std::env::current_exe() {
        let directory = executable.parent().unwrap_or(Path::new("."));
        let portable = directory.join("startup.json");
        if portable.is_file() {
            return portable;
        }
        for ancestor in directory.ancestors() {
            let path = ancestor.join("src/ui/startup.json");
            if path.is_file() {
                return path;
            }
        }
    }
    PathBuf::from("src/ui/startup.json")
}

pub(in crate::app) struct Startup {
    config: Config,
    started: Option<Instant>,
    finished: bool,
}

impl Startup {
    pub fn new(preferences: Option<Config>) -> Self {
        let mut config = preferences.unwrap_or_else(Config::load);
        config.normalize();
        Self {
            finished: !config.enabled,
            config,
            started: None,
        }
    }

    pub fn is_active(&self) -> bool {
        !self.finished
    }

    pub fn preferences(&self) -> Config {
        let mut config = self.config.clone();
        config.normalize();
        config
    }

    pub fn draw(&mut self, context: &egui::Context) {
        if self.finished {
            return;
        }
        let elapsed = self
            .started
            .get_or_insert_with(Instant::now)
            .elapsed()
            .as_secs_f32();
        let preview = std::env::args()
            .find_map(|arg| {
                arg.strip_prefix("--startup-preview=")
                    .and_then(|value| value.parse::<f32>().ok())
            })
            .filter(|value| value.is_finite());
        let elapsed = preview
            .unwrap_or(elapsed)
            .clamp(0.0, self.config.duration_seconds);
        if elapsed >= self.config.duration_seconds
            || context.input(|input| input.key_pressed(egui::Key::Escape))
        {
            self.finished = true;
            context.request_repaint();
            return;
        }
        let screen = context.content_rect();
        let body = Rect::from_min_max(pos2(screen.left(), screen.top() + 42.0), screen.max);
        let scene = Scene::capture(context, body, self.config.cursor_count, self.config.seed);
        let layer = egui::LayerId::new(egui::Order::Foreground, egui::Id::new("startup_swarm"));
        self.paint(
            context.layer_painter(layer).with_clip_rect(body),
            body,
            elapsed,
            &scene,
        );
        egui::Area::new(egui::Id::new("startup_swarm"))
            .order(egui::Order::Foreground)
            .fixed_pos(body.min)
            .show(context, |ui| {
                let (rect, _) = ui.allocate_exact_size(body.size(), Sense::click_and_drag());
                let skip =
                    Rect::from_min_size(rect.right_bottom() - vec2(108.0, 42.0), vec2(94.0, 28.0));
                if ui.put(skip, egui::Button::new("Skip · Esc")).clicked() {
                    self.finished = true;
                }
            });
        context.request_repaint_after(Duration::from_millis(16));
    }

    fn paint(&self, painter: egui::Painter, rect: Rect, elapsed: f32, scene: &Scene) {
        let config = &self.config;
        let build_duration = config.duration_seconds - config.fade_seconds;
        let progress = (elapsed / build_duration).min(1.0);
        let exit = ((elapsed - build_duration) / config.fade_seconds).clamp(0.0, 1.0);
        let time = progress * scene.duration;
        let background = parse_color(&config.background).unwrap_or(super::theme::background());
        painter.rect_filled(rect, 0, background);
        for (shape, element) in scene.shapes.iter().zip(&scene.owners) {
            let task = &scene.elements[*element];
            let fraction = task.fraction(time);
            if fraction <= 0.0 {
                continue;
            }
            let clip = if fraction >= 1.0 {
                shape.clip_rect
            } else {
                let mut reveal = task.bounds;
                reveal.max.x = reveal.left() + reveal.width() * fraction;
                if task.kind != Kind::Text {
                    reveal.max.y = reveal.top() + reveal.height() * fraction;
                }
                shape.clip_rect.intersect(reveal)
            };
            if clip.is_positive() {
                painter
                    .with_clip_rect(rect.intersect(clip))
                    .add(shape.shape.clone());
            }
        }
        let names = config.selected_names();
        for worker in 0..config.cursor_count {
            let jobs: Vec<_> = scene
                .elements
                .iter()
                .filter(|element| element.worker == worker)
                .collect();
            let Some(task) = jobs
                .iter()
                .find(|element| time < element.end)
                .or_else(|| jobs.last())
            else {
                continue;
            };
            let previous = jobs
                .iter()
                .take_while(|element| element.start < task.start)
                .last();
            let origin = previous.map_or_else(
                || {
                    pos2(
                        rect.center().x + (worker as f32 * 1.7).sin() * rect.width() * 0.22,
                        rect.top() - 40.0,
                    )
                },
                |element| element.bounds.right_bottom(),
            );
            let movement = ((time - task.start) / (task.end - task.start) / 0.25).clamp(0.0, 1.0);
            let fraction = task.fraction(time);
            let target = task.bounds;
            let mut pointer = if movement < 1.0 {
                origin.lerp(target.min, ease(movement))
            } else if task.kind == Kind::Text {
                pos2(target.left() + target.width() * fraction, target.center().y)
            } else {
                target.min.lerp(target.max, fraction)
            };
            let color = parse_color(&config.colors[worker % config.colors.len()])
                .unwrap_or(Color32::LIGHT_BLUE)
                .gamma_multiply(1.0 - exit);
            if config.show_trails && movement < 1.0 && time >= task.start && exit == 0.0 {
                painter.line_segment(
                    [origin.lerp(pointer, 0.72), pointer],
                    Stroke::new(1.2_f32, color.gamma_multiply(0.25)),
                );
            }
            if time >= task.start && time < task.end && movement >= 1.0 && exit == 0.0 {
                if task.kind == Kind::Text {
                    painter.line_segment(
                        [
                            pos2(pointer.x, target.top()),
                            pos2(pointer.x, target.bottom()),
                        ],
                        Stroke::new(1.5_f32, color),
                    );
                } else {
                    let drawn =
                        Rect::from_min_max(target.min, pointer.max(target.min + Vec2::splat(0.1)));
                    painter.rect_stroke(
                        drawn,
                        task.rounding,
                        Stroke::new(1.0_f32, color.gamma_multiply(0.8)),
                        egui::StrokeKind::Inside,
                    );
                    for anchor in [drawn.min, drawn.right_top(), drawn.max, drawn.left_bottom()] {
                        painter.rect_filled(
                            Rect::from_center_size(anchor, Vec2::splat(4.0)),
                            0,
                            color,
                        );
                    }
                }
                if config.show_guides && task.kind == Kind::Box {
                    for x in [target.left(), target.right()] {
                        painter.line_segment(
                            [pos2(x, rect.top()), pos2(x, rect.bottom())],
                            Stroke::new(0.5_f32, color.gamma_multiply(0.18)),
                        );
                    }
                }
            }
            if exit > 0.0 {
                let edge = if worker % 2 == 0 {
                    rect.left() - 90.0
                } else {
                    rect.right() + 90.0
                };
                pointer = pointer.lerp(pos2(edge, rect.top() - 60.0), ease(exit));
            }
            cursor(&painter, pointer, config.cursor_size, color, 1.0 - exit);
            if config.show_names {
                let label = names
                    .get(worker % names.len().max(1))
                    .copied()
                    .unwrap_or("Cursor");
                let label_pos = pointer + vec2(config.cursor_size * 0.75, config.cursor_size * 0.9);
                let galley = painter.layout_no_wrap(
                    label.to_owned(),
                    FontId::proportional(11.0),
                    Color32::WHITE.gamma_multiply(1.0 - exit),
                );
                painter.rect_filled(
                    Rect::from_min_size(
                        label_pos - vec2(5.0, 3.0),
                        galley.size() + vec2(10.0, 6.0),
                    ),
                    4,
                    color,
                );
                painter.galley(label_pos, galley, Color32::WHITE);
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Box,
    Text,
    Detail,
}

struct Element {
    bounds: Rect,
    rounding: egui::CornerRadius,
    kind: Kind,
    worker: usize,
    start: f32,
    end: f32,
}

impl Element {
    fn fraction(&self, time: f32) -> f32 {
        (((time - self.start) / (self.end - self.start) - 0.25) / 0.75).clamp(0.0, 1.0)
    }
}

#[derive(Default)]
struct Scene {
    shapes: Vec<egui::epaint::ClippedShape>,
    owners: Vec<usize>,
    elements: Vec<Element>,
    duration: f32,
}

impl Scene {
    fn capture(context: &egui::Context, body: Rect, workers: usize, seed: u64) -> Self {
        let mut layers = context.memory(|memory| memory.layer_ids().collect::<Vec<_>>());
        if !layers.contains(&egui::LayerId::background()) {
            layers.insert(0, egui::LayerId::background());
        }
        layers.sort_by_key(|layer| layer.order);
        let mut shapes = Vec::new();
        for layer in layers {
            if layer.id == egui::Id::new("startup_swarm") {
                continue;
            }
            let entries = context.graphics(|graphics| {
                graphics
                    .get(layer)
                    .map(|list| list.all_entries().cloned().collect::<Vec<_>>())
                    .unwrap_or_default()
            });
            let transform = context.layer_transform_to_global(layer);
            for mut entry in entries {
                if let Some(transform) = transform {
                    entry.transform(transform);
                }
                flatten(entry.shape, entry.clip_rect.intersect(body), &mut shapes);
            }
        }
        Self::from_shapes(shapes, body, workers, seed)
    }

    fn from_shapes(
        shapes: Vec<egui::epaint::ClippedShape>,
        body: Rect,
        workers: usize,
        seed: u64,
    ) -> Self {
        let mut scene = Self::default();
        for shape in shapes {
            let visible = shape
                .shape
                .visual_bounding_rect()
                .intersect(shape.clip_rect)
                .intersect(body);
            if !visible.is_finite() || visible.width() < 0.5 || visible.height() < 0.5 {
                continue;
            }
            let (kind, bounds, rounding) = match &shape.shape {
                Shape::Rect(rect) => (
                    Kind::Box,
                    rect.rect.intersect(shape.clip_rect).intersect(body),
                    rect.corner_radius,
                ),
                Shape::Text(_) => (Kind::Text, visible, egui::CornerRadius::ZERO),
                _ => (Kind::Detail, visible, egui::CornerRadius::ZERO),
            };
            if !bounds.is_positive() {
                continue;
            }
            let merge = scene.elements.last().is_some_and(|previous| {
                let combined = previous.bounds.union(bounds);
                kind == Kind::Detail
                    && previous.kind == Kind::Detail
                    && previous.bounds.expand(5.0).intersects(bounds)
                    && combined.width() <= 48.0
                    && combined.height() <= 48.0
            });
            let owner = if merge {
                let index = scene.elements.len() - 1;
                scene.elements[index].bounds = scene.elements[index].bounds.union(bounds);
                index
            } else {
                scene.elements.push(Element {
                    bounds,
                    rounding,
                    kind,
                    worker: 0,
                    start: 0.0,
                    end: 0.0,
                });
                scene.elements.len() - 1
            };
            scene.shapes.push(shape);
            scene.owners.push(owner);
        }
        let mut availability = vec![0.0_f32; workers.max(1)];
        for index in 0..scene.elements.len() {
            let bounds = scene.elements[index].bounds;
            let ready = scene.elements[..index]
                .iter()
                .filter(|parent| {
                    parent.kind == Kind::Box
                        && parent.bounds.contains_rect(bounds)
                        && parent.bounds.area() > bounds.area() * 1.05
                })
                .map(|parent| parent.end)
                .fold(0.0_f32, f32::max);
            let offset = (seed.wrapping_add(index as u64) % availability.len() as u64) as usize;
            let worker = (0..availability.len())
                .map(|i| (i + offset) % availability.len())
                .min_by(|a, b| availability[*a].total_cmp(&availability[*b]))
                .unwrap_or(0);
            let element = &mut scene.elements[index];
            element.worker = worker;
            element.start = ready.max(availability[worker]);
            let weight = match element.kind {
                Kind::Box => 1.2,
                Kind::Text => 0.8,
                Kind::Detail => 0.45,
            };
            element.end = element.start + weight;
            availability[worker] = element.end;
            scene.duration = scene.duration.max(element.end);
        }
        scene
    }
}

fn flatten(shape: Shape, clip_rect: Rect, shapes: &mut Vec<egui::epaint::ClippedShape>) {
    match shape {
        Shape::Vec(children) => {
            for child in children {
                flatten(child, clip_rect, shapes);
            }
        }
        Shape::Noop => {}
        shape => shapes.push(egui::epaint::ClippedShape { clip_rect, shape }),
    }
}

fn cursor(painter: &egui::Painter, tip: Pos2, size: f32, color: Color32, alpha: f32) {
    let points = [
        vec2(0.0, 0.0),
        vec2(0.26, 0.94),
        vec2(0.47, 0.59),
        vec2(0.91, 0.43),
    ]
    .map(|point| tip + point * size)
    .to_vec();
    painter.add(Shape::convex_polygon(
        points,
        color,
        Stroke::new(1.2_f32, Color32::WHITE.gamma_multiply(alpha)),
    ));
}

fn parse_color(value: &str) -> Option<Color32> {
    let value = value.strip_prefix('#').unwrap_or(value);
    if value.len() != 6 {
        return None;
    }
    let rgb = u32::from_str_radix(value, 16).ok()?;
    Some(Color32::from_rgb(
        (rgb >> 16) as u8,
        (rgb >> 8) as u8,
        rgb as u8,
    ))
}

fn finite(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}
fn ease(value: f32) -> f32 {
    let t = value.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

#[cfg(test)]
#[path = "../../tests/ui/startup.rs"]
mod tests;
