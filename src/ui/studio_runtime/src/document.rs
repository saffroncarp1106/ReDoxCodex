use crate::{License, Skin};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fs::File,
    io::{Read, Write},
    path::Path,
};

pub const MAX_DOCUMENT_BYTES: usize = 16 * 1024 * 1024;
pub const MAX_NODES: usize = 1000;
pub const MAX_PAGES: usize = 12;
pub const MAX_TEXT: usize = 2048;
pub const MAX_DEPTH: usize = 16;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Document {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<crate::scenes::Source>,
    pub version: u8,
    pub name: String,
    pub author: String,
    pub license: License,
    pub pages: Vec<Page>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub textures: Vec<TextureAsset>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TextureAsset {
    pub png_base64: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtVertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
    pub color: [u8; 4],
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtMesh {
    pub texture: usize,
    pub vertices: Vec<ArtVertex>,
    pub indices: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum VectorIcon {
    FolderBack,
    FolderFront,
    SplitPanel,
    SplitDivider,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceTextLayout {
    pub width: Option<f32>,
    pub max_rows: usize,
    pub break_anywhere: bool,
    pub overflow_character: Option<char>,
    pub align: TextAlign,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Artwork {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text_layout: Option<SourceTextLayout>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub font_face: Option<String>,
    #[serde(default)]
    pub text_origin: [f32; 2],
    #[serde(default, skip_serializing_if = "Font::is_sans")]
    pub original_font: Font,
    #[serde(default, skip_serializing_if = "TextAlign::is_left")]
    pub original_text_align: TextAlign,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub vector_icon: Option<VectorIcon>,
    pub size: [f32; 2],
    pub meshes: Vec<ArtMesh>,
    pub clip: [f32; 4],
    pub original_text: String,
    pub original_color: [u8; 4],
    pub original_font_size: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Page {
    pub name: String,
    pub background: [u8; 3],
    pub nodes: Vec<Node>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Artwork,
    Frame,
    Group,
    Rectangle,
    Ellipse,
    Text,
    Button,
    Input,
    Line,
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Artwork => "Artwork",
            Self::Frame => "Frame",
            Self::Group => "Group",
            Self::Rectangle => "Rectangle",
            Self::Ellipse => "Ellipse",
            Self::Text => "Text",
            Self::Button => "Button",
            Self::Input => "Input",
            Self::Line => "Line",
        }
    }
    pub fn container(self) -> bool {
        matches!(self, Self::Frame | Self::Group)
    }
    pub fn has_text(self) -> bool {
        matches!(self, Self::Text | Self::Button | Self::Input)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Flow {
    Free,
    Horizontal,
    Vertical,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextAlign {
    #[default]
    Left,
    Center,
    Right,
}

impl TextAlign {
    pub fn is_left(&self) -> bool {
        *self == Self::Left
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Font {
    #[default]
    Sans,
    Mono,
}

impl Font {
    pub fn is_sans(&self) -> bool {
        *self == Self::Sans
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Node {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ui_script: Vec<crate::actions::UiAction>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub artwork: Option<Artwork>,
    pub id: u32,
    pub parent: Option<u32>,
    pub kind: Kind,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub visible: bool,
    pub locked: bool,
    pub fill: [u8; 4],
    pub stroke: [u8; 4],
    pub stroke_width: f32,
    pub radius: f32,
    pub opacity: f32,
    pub text: String,
    pub text_color: [u8; 4],
    pub font_size: f32,
    pub font: Font,
    pub text_align: TextAlign,
    pub clip_content: bool,
    pub flow: Flow,
    pub gap: f32,
    pub padding: f32,
}

impl Node {
    pub fn new(id: u32, kind: Kind, x: f32, y: f32) -> Self {
        let (width, height) = match kind {
            Kind::Frame => (960., 640.),
            Kind::Text => (240., 40.),
            Kind::Button => (140., 44.),
            Kind::Input => (280., 44.),
            Kind::Line => (200., 1.),
            _ => (160., 120.),
        };
        let fill = match kind {
            Kind::Frame | Kind::Input => [255, 255, 255, 255],
            Kind::Button => [63, 104, 235, 255],
            Kind::Text | Kind::Group | Kind::Line => [0, 0, 0, 0],
            _ => [217, 222, 234, 255],
        };
        Self {
            ui_script: vec![],
            artwork: None,
            id,
            parent: None,
            kind,
            name: format!("{} {id}", kind.label()),
            x,
            y,
            width,
            height,
            visible: true,
            locked: false,
            fill,
            stroke: [160, 168, 184, 255],
            stroke_width: if matches!(kind, Kind::Input | Kind::Line) {
                1.
            } else {
                0.
            },
            radius: if matches!(kind, Kind::Button | Kind::Input) {
                8.
            } else {
                0.
            },
            opacity: 1.,
            text: match kind {
                Kind::Text => "Your text".into(),
                Kind::Button => "Button".into(),
                Kind::Input => "Placeholder".into(),
                _ => String::new(),
            },
            text_color: if kind == Kind::Button {
                [255, 255, 255, 255]
            } else {
                [32, 37, 49, 255]
            },
            font_size: if kind == Kind::Text { 24. } else { 14. },
            font: Font::Sans,
            text_align: if kind == Kind::Button {
                TextAlign::Center
            } else {
                TextAlign::Left
            },
            clip_content: kind == Kind::Frame,
            flow: Flow::Free,
            gap: 16.,
            padding: 16.,
        }
    }
}

impl Page {
    pub fn blank(name: &str) -> Self {
        Self {
            name: name.into(),
            background: [30, 30, 33],
            nodes: vec![],
        }
    }
    pub fn node(&self, id: u32) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == id)
    }
    pub fn node_mut(&mut self, id: u32) -> Option<&mut Node> {
        self.nodes.iter_mut().find(|n| n.id == id)
    }
    pub fn children(&self, parent: Option<u32>) -> Vec<u32> {
        self.nodes
            .iter()
            .filter(|n| n.parent == parent)
            .map(|n| n.id)
            .collect()
    }
    pub fn ancestors(&self, id: u32) -> Vec<u32> {
        let mut result = vec![];
        let mut parent = self.node(id).and_then(|n| n.parent);
        while let Some(id) = parent {
            if result.len() >= MAX_DEPTH {
                break;
            }
            result.push(id);
            parent = self.node(id).and_then(|n| n.parent);
        }
        result
    }
    pub fn world_rect(&self, id: u32) -> Option<[f32; 4]> {
        let node = self.node(id)?;
        let (mut x, mut y) = (node.x, node.y);
        for id in self.ancestors(id) {
            let parent = self.node(id)?;
            x += parent.x;
            y += parent.y;
        }
        Some([x, y, node.width, node.height])
    }
    pub fn editable(&self, id: u32) -> bool {
        self.node(id).is_some_and(|n| !n.locked && n.visible)
            && self
                .ancestors(id)
                .iter()
                .all(|id| self.node(*id).is_some_and(|n| !n.locked && n.visible))
    }
    pub fn scene_order(&self) -> Vec<u32> {
        fn visit(page: &Page, parent: Option<u32>, output: &mut Vec<u32>, depth: usize) {
            if depth > MAX_DEPTH {
                return;
            }
            for id in page.children(parent) {
                output.push(id);
                visit(page, Some(id), output, depth + 1);
            }
        }
        let mut output = vec![];
        visit(self, None, &mut output, 0);
        output
    }
    pub fn relayout(&mut self) {
        for id in self.scene_order().into_iter().rev() {
            let Some(parent) = self.node(id) else {
                continue;
            };
            let (flow, gap, padding) = (parent.flow, parent.gap, parent.padding);
            if flow == Flow::Free {
                continue;
            }
            let mut cursor = padding;
            for child in self.children(Some(id)) {
                if let Some(node) = self.node_mut(child) {
                    if !node.visible {
                        continue;
                    }
                    match flow {
                        Flow::Horizontal => {
                            node.x = cursor;
                            node.y = padding;
                            cursor += node.width + gap;
                        }
                        Flow::Vertical => {
                            node.x = padding;
                            node.y = cursor;
                            cursor += node.height + gap;
                        }
                        Flow::Free => {}
                    }
                }
            }
        }
    }
}

impl Document {
    pub fn blank() -> Self {
        Self {
            source: None,
            version: 2,
            name: "Untitled".into(),
            author: "Creator".into(),
            license: License::Cc0,
            pages: vec![Page::blank("Page 1")],
            textures: vec![],
        }
    }
    pub fn next_id(&self) -> u32 {
        let used: HashSet<_> = self
            .pages
            .iter()
            .flat_map(|p| &p.nodes)
            .map(|n| n.id)
            .collect();
        (1..).find(|id| !used.contains(id)).unwrap()
    }
    pub fn node_count(&self) -> usize {
        self.pages.iter().map(|p| p.nodes.len()).sum()
    }

    pub fn validate(&self) -> Result<(), String> {
        use base64::Engine;
        if self.textures.len() > 8 {
            return Err("A design supports up to 8 embedded textures".into());
        }
        let mut total_pixels = 0;
        for texture in &self.textures {
            if texture.png_base64.len() > 6 * 1024 * 1024 {
                return Err("Embedded texture exceeds its byte limit".into());
            }
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(&texture.png_base64)
                .map_err(|e| e.to_string())?;
            let reader = image::ImageReader::with_format(
                std::io::Cursor::new(bytes),
                image::ImageFormat::Png,
            );
            let (w, h) = reader.into_dimensions().map_err(|e| e.to_string())?;
            if w == 0 || h == 0 || w > 4096 || h > 4096 {
                return Err("Unsupported embedded texture dimensions".into());
            }
            total_pixels += w as u64 * h as u64;
            if total_pixels > 4 * 1024 * 1024 {
                return Err("Embedded textures exceed 4 million pixels".into());
            }
        }
        let mut total_vertices = 0;
        if self.version != 2 {
            return Err("Unsupported design version".into());
        }
        if let Some(source) = &self.source
            && !crate::scenes::THEMES.contains(&source.theme.as_str())
        {
            return Err("Unknown source theme".into());
        }
        valid_label(&self.name, "Document name")?;
        valid_label(&self.author, "Creator")?;
        if self.pages.is_empty() || self.pages.len() > MAX_PAGES || self.node_count() > MAX_NODES {
            return Err("A design supports 1–12 pages and up to 1000 layers".into());
        }
        let mut all_ids = HashSet::new();
        for page in &self.pages {
            valid_label(&page.name, "Page name")?;
            for node in &page.nodes {
                if let Some(art) = &node.artwork {
                    if art.text_layout.as_ref().is_some_and(|layout| {
                        layout.max_rows == 0
                            || layout.max_rows > 64
                            || layout.width.is_some_and(|width| {
                                !width.is_finite() || !(1. ..=16000.).contains(&width)
                            })
                    }) {
                        return Err("Invalid source text layout".into());
                    }
                    if art.font_face.as_ref().is_some_and(|name| {
                        !crate::presentation::typography::FACES
                            .iter()
                            .any(|(face, _)| face == name)
                    }) || art
                        .text_origin
                        .iter()
                        .any(|v| !v.is_finite() || v.abs() > 32000.)
                    {
                        return Err("Invalid source typography".into());
                    }
                    if art
                        .size
                        .iter()
                        .any(|v| !v.is_finite() || *v < 1. || *v > 16000.)
                    {
                        return Err("Invalid artwork dimensions".into());
                    }
                    if art.meshes.len() > 16
                        || art.clip.iter().any(|v| !v.is_finite() || v.abs() > 32000.)
                        || art.original_text.len() > MAX_TEXT
                        || !art.original_font_size.is_finite()
                    {
                        return Err("Invalid source artwork".into());
                    }
                    for mesh in &art.meshes {
                        total_vertices += mesh.vertices.len();
                        if total_vertices > 200_000
                            || mesh.indices.len() > 600_000
                            || mesh.indices.len() % 3 != 0
                            || mesh.texture >= self.textures.len()
                            || mesh
                                .indices
                                .iter()
                                .any(|i| *i as usize >= mesh.vertices.len())
                        {
                            return Err(
                                "Artwork exceeds mesh limits or references missing data".into()
                            );
                        }
                        if mesh.vertices.iter().any(|v| {
                            v.position
                                .iter()
                                .any(|n| !n.is_finite() || n.abs() > 32000.)
                                || v.uv
                                    .iter()
                                    .any(|n| !n.is_finite() || !(0.0..=1.0).contains(n))
                        }) {
                            return Err("Invalid artwork vertex".into());
                        }
                    }
                }
                if node.id == 0 || node.id > 1_000_000_000 || !all_ids.insert(node.id) {
                    return Err("Layer IDs must be unique positive numbers".into());
                }
                valid_label(&node.name, "Layer name")?;
                if node.ui_script.len() > 16 {
                    return Err("UI scripts support at most 16 calls".into());
                }
                if node.ui_script.iter().any(|a| a.is_control()) && node.ui_script.len() != 1 {
                    return Err(
                        "Composer, model, effort and menus must be standalone controls".into(),
                    );
                }
                if node.text.len() > MAX_TEXT
                    || node
                        .text
                        .chars()
                        .any(|c| c.is_control() && c != '\n' && c != '\t')
                {
                    return Err(
                        "Layer text exceeds its limit or contains control characters".into(),
                    );
                }
                for (value, min, max) in [
                    (node.x, -16000., 16000.),
                    (node.y, -16000., 16000.),
                    (node.width, 1., 16000.),
                    (node.height, 1., 16000.),
                    (node.stroke_width, 0., 32.),
                    (node.radius, 0., 200.),
                    (node.opacity, 0., 1.),
                    (node.font_size, 8., 160.),
                    (node.gap, 0., 256.),
                    (node.padding, 0., 256.),
                ] {
                    if !value.is_finite() || value < min || value > max {
                        return Err(format!(
                            "Layer {} has a value outside the supported range",
                            node.id
                        ));
                    }
                }
                if !node.kind.container() && node.flow != Flow::Free {
                    return Err("Only frames and groups support auto layout".into());
                }
                let mut visited = HashSet::from([node.id]);
                let mut parent = node.parent;
                while let Some(id) = parent {
                    if !visited.insert(id) || visited.len() > MAX_DEPTH {
                        return Err("Layer hierarchy contains a cycle or is too deep".into());
                    }
                    let ancestor = page
                        .node(id)
                        .ok_or("Layer parent is missing from its page")?;
                    if !ancestor.kind.container() {
                        return Err("Layer parent must be a frame or group".into());
                    }
                    parent = ancestor.parent;
                }
            }
        }
        Ok(())
    }
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > MAX_DOCUMENT_BYTES {
            return Err("Design exceeds 16 MiB".into());
        }
        #[derive(Deserialize)]
        struct Header {
            version: u8,
        }
        let header: Header = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
        let mut doc = match header.version {
            1 => Self::from_skin(&Skin::parse(bytes)?),
            2 => serde_json::from_slice(bytes).map_err(|e| format!("Invalid design: {e}"))?,
            _ => return Err("Unsupported skin version".into()),
        };
        doc.validate()?;
        for page in &mut doc.pages {
            page.relayout();
        }
        Ok(doc)
    }
    pub fn encode(&self) -> Result<Vec<u8>, String> {
        self.validate()?;
        let bytes = if self.source.is_some() {
            serde_json::to_vec(self)
        } else {
            serde_json::to_vec_pretty(self)
        }
        .map_err(|e| e.to_string())?;
        if bytes.len() > MAX_DOCUMENT_BYTES {
            return Err("Design exceeds 16 MiB".into());
        }
        Ok(bytes)
    }
    pub fn load(path: &Path) -> Result<Self, String> {
        if path.extension().and_then(|s| s.to_str()) != Some("redoxskin") {
            return Err("Choose a .redoxskin file".into());
        }
        let mut bytes = vec![];
        File::open(path)
            .map_err(|e| e.to_string())?
            .take((MAX_DOCUMENT_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        Self::parse(&bytes)
    }
    pub fn save(&self, path: &Path) -> Result<(), String> {
        let bytes = self.encode()?;
        let parent = path.parent().ok_or("Choose a file in a local folder")?;
        let mut temp = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
        temp.write_all(&bytes)
            .and_then(|_| temp.as_file().sync_all())
            .map_err(|e| e.to_string())?;
        temp.persist(path).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn from_skin(skin: &Skin) -> Self {
        let mut doc = Self::blank();
        doc.name = skin.name.clone();
        doc.author = skin.author.clone();
        let p = &skin.palette;
        let rgba = |c: [u8; 3]| [c[0], c[1], c[2], 255];
        let page = &mut doc.pages[0];
        let mut frame = Node::new(1, Kind::Frame, 0., 0.);
        frame.name = "ReDox workspace".into();
        frame.fill = rgba(p.surface);
        frame.radius = skin.layout.rounding as f32;
        page.nodes.push(frame);
        let mut title = Node::new(2, Kind::Rectangle, 0., 0.);
        title.parent = Some(1);
        title.width = 960.;
        title.height = 40.;
        title.fill = rgba(p.accent);
        title.name = "Title bar".into();
        page.nodes.push(title);
        let mut caption = Node::new(3, Kind::Text, 18., 10.);
        caption.parent = Some(1);
        caption.text = "ReDox — Your workspace".into();
        caption.text_color = rgba(p.accent_text);
        caption.font_size = 15.;
        caption.width = 600.;
        caption.height = 24.;
        page.nodes.push(caption);
        let width = skin.layout.sidebar_width as f32;
        let sx = if skin.layout.sidebar_right {
            960. - width
        } else {
            0.
        };
        let cx = if skin.layout.sidebar_right {
            40.
        } else {
            width + 40.
        };
        let mut side = Node::new(4, Kind::Frame, sx, 40.);
        side.parent = Some(1);
        side.width = width;
        side.height = 600.;
        side.fill = rgba(p.background);
        side.name = "Sidebar".into();
        side.clip_content = true;
        page.nodes.push(side);
        for (i, text) in [
            "My workspace",
            "New conversation",
            "Recent messages",
            "Design explorations",
        ]
        .iter()
        .enumerate()
        {
            let mut node = Node::new(
                5 + i as u32,
                Kind::Text,
                16.,
                28. + i as f32 * (40. + skin.layout.spacing as f32),
            );
            node.parent = Some(4);
            node.text = (*text).into();
            node.name = (*text).into();
            node.width = width - 32.;
            node.font_size = skin.layout.text_size as f32;
            node.text_color = rgba(p.text);
            page.nodes.push(node);
        }
        let mut heading = Node::new(9, Kind::Text, cx, 160.);
        heading.parent = Some(1);
        heading.text = "Make room for your next idea.".into();
        heading.width = 650. - width / 2.;
        heading.height = 80.;
        heading.font_size = 30.;
        heading.text_color = rgba(p.text);
        page.nodes.push(heading);
        let mut body = Node::new(10, Kind::Text, cx, 260.);
        body.parent = Some(1);
        body.text =
            "A workspace designed by you.\nEvery element here can be moved, resized or replaced."
                .into();
        body.width = 640. - width / 2.;
        body.height = 100.;
        body.font_size = skin.layout.text_size as f32;
        body.text_color = rgba(p.text);
        page.nodes.push(body);
        let mut input = Node::new(11, Kind::Input, cx, 500.);
        input.parent = Some(1);
        input.width = 960. - width - 80.;
        input.height = 94.;
        input.text = "Ask ReDox anything…".into();
        input.fill = rgba(p.surface);
        input.text_color = rgba(p.text);
        input.radius = skin.layout.rounding as f32;
        input.stroke_width = if skin.layout.bevel { 3. } else { 1. };
        page.nodes.push(input);
        let mut button = Node::new(12, Kind::Button, 960. - width - 172., 548.);
        button.x = if skin.layout.sidebar_right {
            960. - width - 132.
        } else {
            828.
        };
        button.parent = Some(1);
        button.width = 92.;
        button.height = 32.;
        button.text = "Send".into();
        button.fill = rgba(p.accent);
        button.text_color = rgba(p.accent_text);
        button.radius = skin.layout.rounding as f32;
        page.nodes.push(button);
        doc
    }
}

fn valid_label(value: &str, field: &str) -> Result<(), String> {
    if value.trim().is_empty() || value.len() > 128 || value.chars().any(char::is_control) {
        return Err(format!("{field} must contain 1–128 bytes of visible text"));
    }
    Ok(())
}
