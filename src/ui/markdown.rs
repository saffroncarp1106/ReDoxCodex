use std::cell::RefCell;
use std::collections::HashMap;

use eframe::egui::{self, Color32, RichText, TextureHandle, Vec2, vec2};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use latex_rust::{Dim, MathFont, MathStyle, PngOptions};
use pulldown_cmark::{Event, Parser, Tag};

use super::theme;

thread_local! {
    static MARKDOWN: RefCell<CommonMarkCache> = RefCell::new(CommonMarkCache::default());
    static MATH: RefCell<MathCache> = RefCell::new(MathCache::default());
}

#[cfg(test)]
pub(super) fn show(ui: &mut egui::Ui, source: &str) {
    show_in_workspace(ui, source, None);
}

pub(super) fn show_in_workspace(
    ui: &mut egui::Ui,
    source: &str,
    workspace: Option<&std::path::Path>,
) {
    let source = normalize_delimiters(source);
    let error_id = ui.next_auto_id().with("file_link_error");
    let command_start = ui.output_mut(|output| output.commands.len());
    let mut offset = 0;
    for reference in super::media::references(&source) {
        render(ui, &source[offset..reference.range.start]);
        super::media::show(ui, &reference.destination, reference.video, workspace);
        offset = reference.range.end;
    }
    render(ui, &source[offset..]);
    let file_links = ui.output_mut(|output| {
        let mut file_links = Vec::new();
        let new_commands = output.commands.split_off(command_start);
        for command in new_commands {
            match command {
                egui::OutputCommand::OpenUrl(link) if redox::media::is_file_link(&link.url) => {
                    file_links.push(link.url);
                }
                command => output.commands.push(command),
            }
        }
        file_links
    });
    for link in file_links {
        let destination = Parser::new(&source)
            .into_offset_iter()
            .find_map(|(event, range)| {
                if let Event::Start(Tag::Link { dest_url, .. }) = event
                    && dest_url.as_ref() == link
                {
                    Some(super::media::preserve_windows_path(&source[range], &link))
                } else {
                    None
                }
            })
            .unwrap_or(link);
        let error = redox::media::open_file_link(&destination, workspace).err();
        ui.data_mut(|data| data.insert_temp(error_id, error));
    }
    if let Some(Some(error)) = ui.data(|data| data.get_temp::<Option<String>>(error_id)) {
        ui.colored_label(Color32::LIGHT_RED, format!("Could not open file: {error}"));
    }
}

fn render(ui: &mut egui::Ui, source: &str) {
    let mut offset = 0;
    let mut block_start = None;
    let mut code = String::new();
    for (event, range) in Parser::new(source).into_offset_iter() {
        match event {
            Event::Start(Tag::CodeBlock(_)) => {
                render_prose(ui, &source[offset..range.start]);
                block_start = Some(range.start);
                code.clear();
            }
            Event::Text(text) if block_start.is_some() => code.push_str(&text),
            Event::End(pulldown_cmark::TagEnd::CodeBlock) if block_start.is_some() => {
                egui::Frame::group(ui.style()).show(ui, |ui| {
                    let width = ui.available_width();
                    ui.horizontal_top(|ui| {
                        ui.allocate_ui_with_layout(
                            vec2((width - 36.0).max(1.0), 24.0),
                            egui::Layout::top_down(egui::Align::Min),
                            |ui| {
                                super::scroll::ScrollArea::horizontal().show(ui, |ui| {
                                    ui.label(
                                        RichText::new(code.trim_end_matches('\n')).monospace(),
                                    );
                                });
                            },
                        );
                        if ui
                            .add_sized([28.0, 24.0], egui::Button::new("Copy"))
                            .on_hover_text("Copy code")
                            .clicked()
                        {
                            ui.ctx().copy_text(code.clone());
                        }
                    });
                });
                offset = range.end;
                block_start = None;
            }
            _ => {}
        }
    }
    render_prose(ui, &source[offset..]);
}

fn render_prose(ui: &mut egui::Ui, source: &str) {
    MARKDOWN.with(|cache| {
        CommonMarkViewer::new()
            .render_math_fn(Some(&render_math))
            .show(ui, &mut cache.borrow_mut(), source);
    });
}
fn render_math(ui: &mut egui::Ui, source: &str, inline: bool) {
    MATH.with(|cache| cache.borrow_mut().show(ui, source, inline));
}

pub(super) fn normalize_delimiters(source: &str) -> String {
    if !source.contains(r"\[") && !source.contains(r"\(") {
        return source.to_owned();
    }
    let mut protected: Vec<_> = Parser::new(source)
        .into_offset_iter()
        .filter_map(|(event, range)| {
            matches!(event, Event::Code(_) | Event::Start(Tag::CodeBlock(_))).then_some(range)
        })
        .collect();
    protected.sort_by_key(|range| range.start);
    let mut output = String::with_capacity(source.len());
    let mut offset = 0;
    for range in protected {
        if range.start < offset {
            continue;
        }
        normalize_text(&source[offset..range.start], &mut output);
        output.push_str(&source[range.clone()]);
        offset = range.end;
    }
    normalize_text(&source[offset..], &mut output);
    output
}

fn normalize_text(source: &str, output: &mut String) {
    let mut offset = 0;
    while offset < source.len() {
        let rest = &source[offset..];
        if rest.starts_with(r"\\") {
            output.push_str(r"\\");
            offset += 2;
            continue;
        }
        let (open, close, block) = if rest.starts_with(r"\[") {
            (r"\[", r"\]", true)
        } else if rest.starts_with(r"\(") {
            (r"\(", r"\)", false)
        } else {
            let ch = rest.chars().next().unwrap();
            output.push(ch);
            offset += ch.len_utf8();
            continue;
        };
        if let Some(end) = rest[open.len()..].find(close) {
            let math = &rest[open.len()..open.len() + end];
            if block {
                output.push_str("\n\n$$\n");
                output.push_str(math.trim());
                output.push_str("\n$$\n\n");
            } else {
                output.push('$');
                output.push_str(math.trim());
                output.push('$');
            }
            offset += open.len() + end + close.len();
        } else {
            output.push_str(rest);
            break;
        }
    }
}

#[derive(Hash, PartialEq, Eq)]
struct MathKey {
    source: String,
    inline: bool,
    color: Color32,
}

#[derive(Clone)]
struct Formula {
    texture: TextureHandle,
    size: Vec2,
}

struct MathCache {
    font: Result<MathFont, String>,
    entries: HashMap<MathKey, Result<Formula, String>>,
    bytes: usize,
}

impl Default for MathCache {
    fn default() -> Self {
        Self {
            font: MathFont::stix_two_math().map_err(|error| error.to_string()),
            entries: HashMap::new(),
            bytes: 0,
        }
    }
}

impl MathCache {
    fn show(&mut self, ui: &mut egui::Ui, source: &str, inline: bool) {
        let color = theme::text();
        let key = MathKey {
            source: source.into(),
            inline,
            color,
        };
        if !self.entries.contains_key(&key) {
            if self.entries.len() >= 128 || self.bytes >= 16 * 1024 * 1024 {
                self.entries.clear();
                self.bytes = 0;
            }
            let rendered = self.render(ui.ctx(), source, inline, color);
            if let Ok(formula) = &rendered {
                let [width, height] = formula.texture.size();
                self.bytes += width * height * 4;
            }
            self.entries.insert(key, rendered);
        }
        let key = MathKey {
            source: source.into(),
            inline,
            color,
        };
        match self.entries.get(&key).cloned().unwrap() {
            Ok(formula) => {
                let draw = |ui: &mut egui::Ui| {
                    let response = ui
                        .add(egui::Image::new((formula.texture.id(), formula.size)))
                        .on_hover_text(source);
                    response.context_menu(|ui| {
                        if ui.button("Copy LaTeX").clicked() {
                            ui.ctx().copy_text(source.into());
                            ui.close();
                        }
                    });
                };
                if inline {
                    draw(ui);
                } else {
                    ui.add_space(6.0);
                    super::scroll::ScrollArea::horizontal()
                        .id_salt(("equation", source))
                        .max_width(ui.available_width())
                        .auto_shrink([false, true])
                        .show(ui, draw);
                    ui.add_space(6.0);
                }
            }
            Err(error) => {
                ui.add(
                    egui::Label::new(RichText::new(source).monospace())
                        .wrap()
                        .selectable(true),
                )
                .on_hover_text(format!("Equation could not be formatted: {error}"));
            }
        }
    }

    fn render(
        &self,
        context: &egui::Context,
        source: &str,
        inline: bool,
        color: Color32,
    ) -> Result<Formula, String> {
        let font = self.font.as_ref().map_err(Clone::clone)?;
        let bytes = math_png(source, inline, color, font)?;
        let image = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)
            .map_err(|error| error.to_string())?
            .to_rgba8();
        let size = vec2(image.width() as f32 * 0.5, image.height() as f32 * 0.5);
        let pixels = egui::ColorImage::from_rgba_unmultiplied(
            [image.width() as usize, image.height() as usize],
            image.as_raw(),
        );
        let texture =
            context.load_texture("formatted_equation", pixels, egui::TextureOptions::LINEAR);
        Ok(Formula { texture, size })
    }
}

fn math_png(
    source: &str,
    inline: bool,
    color: Color32,
    font: &MathFont,
) -> Result<Vec<u8>, String> {
    if source.len() > 4096 {
        return Err("Equation exceeds the rendering size limit.".into());
    }
    let (ast, _) =
        latex_rust::parse_with_options(source, &latex_rust::ParseOptions::new().with_max_depth(32))
            .map_err(|error| error.to_string())?;
    let options = PngOptions {
        font_size_pt: Dim::from_i64(if inline { 14 } else { 17 }),
        display: !inline,
        color: latex_rust::Color::rgb(color.r(), color.g(), color.b()),
        ..PngOptions::default()
    };
    let layout = latex_rust::layout_with_em_size_pt(
        &ast,
        font,
        if inline {
            MathStyle::Text
        } else {
            MathStyle::Display
        },
        &options.font_size_pt,
    )
    .map_err(|error| error.to_string())?;
    let height = &layout.height + &layout.depth;
    if layout.width.cmp(&Dim::from_i64(96)) != Some(std::cmp::Ordering::Less)
        || height.cmp(&Dim::from_i64(48)) != Some(std::cmp::Ordering::Less)
    {
        return Err("Equation is too large to render.".into());
    }
    latex_rust::render_png(&layout, font, &options).map_err(|error| error.to_string())
}

#[cfg(test)]
#[path = "../../tests/ui/markdown.rs"]
mod tests;
