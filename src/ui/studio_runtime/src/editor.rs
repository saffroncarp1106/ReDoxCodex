use crate::document::{Document, Kind, MAX_DEPTH, MAX_NODES, MAX_PAGES, Node, Page};
use std::collections::{HashMap, HashSet};

#[derive(Clone)]
struct Snapshot {
    document: Document,
    page: usize,
    selection: Vec<u32>,
}

pub struct Editor {
    pub document: Document,
    pub page: usize,
    pub selection: Vec<u32>,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    pending: Option<Snapshot>,
    clipboard: Vec<Node>,
}

#[derive(Clone, Copy)]
pub enum Alignment {
    Left,
    Center,
    Right,
    Top,
    Middle,
    Bottom,
}

impl Editor {
    pub fn new(document: Document) -> Self {
        Self {
            document,
            page: 0,
            selection: vec![],
            undo: vec![],
            redo: vec![],
            pending: None,
            clipboard: vec![],
        }
    }
    pub fn page(&self) -> &Page {
        &self.document.pages[self.page]
    }
    pub fn page_mut(&mut self) -> &mut Page {
        &mut self.document.pages[self.page]
    }
    pub fn single(&self) -> Option<u32> {
        if self.selection.len() == 1 {
            self.selection.first().copied()
        } else {
            None
        }
    }
    fn snapshot(&self) -> Snapshot {
        Snapshot {
            document: self.document.clone(),
            page: self.page,
            selection: self.selection.clone(),
        }
    }

    pub fn begin(&mut self) {
        if self.pending.is_none() {
            self.pending = Some(self.snapshot());
        }
    }
    pub fn commit(&mut self) {
        if let Some(snapshot) = self.pending.take()
            && snapshot.document != self.document
        {
            self.undo.push(snapshot);
            if self.undo.len() > 80 {
                self.undo.remove(0);
            }
            self.redo.clear();
        }
        self.clean_selection();
    }
    pub fn cancel(&mut self) {
        if let Some(snapshot) = self.pending.take() {
            self.restore(snapshot);
        }
    }
    fn restore(&mut self, snapshot: Snapshot) {
        self.document = snapshot.document;
        self.page = snapshot.page.min(self.document.pages.len() - 1);
        self.selection = snapshot.selection;
        self.clean_selection();
    }
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
            || self
                .pending
                .as_ref()
                .is_some_and(|p| p.document != self.document)
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
    pub fn undo(&mut self) {
        self.commit();
        if let Some(snapshot) = self.undo.pop() {
            self.redo.push(self.snapshot());
            self.restore(snapshot);
        }
    }
    pub fn redo(&mut self) {
        self.commit();
        if let Some(snapshot) = self.redo.pop() {
            self.undo.push(self.snapshot());
            self.restore(snapshot);
        }
    }
    pub fn clean_selection(&mut self) {
        let page = &self.document.pages[self.page];
        self.selection.retain(|id| page.node(*id).is_some());
    }
    pub fn select(&mut self, id: u32, toggle: bool) {
        if toggle {
            if self.selection.contains(&id) {
                self.selection.retain(|n| *n != id);
            } else {
                self.selection.push(id);
            }
        } else {
            self.selection = vec![id];
        }
    }
    pub fn roots(&self) -> Vec<u32> {
        self.selection
            .iter()
            .copied()
            .filter(|id| {
                self.page().editable(*id)
                    && !self
                        .page()
                        .ancestors(*id)
                        .iter()
                        .any(|a| self.selection.contains(a))
            })
            .collect()
    }
    pub fn selection_bounds(&self) -> Option<[f32; 4]> {
        let roots = self.roots();
        let selection = if roots.is_empty() {
            &self.selection
        } else {
            &roots
        };
        union(
            selection
                .iter()
                .filter_map(|id| self.page().world_rect(*id)),
        )
    }
    pub fn add(&mut self, kind: Kind, world: [f32; 4], parent: Option<u32>) -> Result<u32, String> {
        if self.document.node_count() >= MAX_NODES {
            return Err("Design has reached the 1000 layer limit".into());
        }
        if parent.is_some_and(|id| {
            !self.page().editable(id) || !self.page().node(id).is_some_and(|n| n.kind.container())
        }) {
            return Err("Choose an unlocked frame or group as the parent".into());
        }
        if parent.is_some_and(|id| self.page().ancestors(id).len() + 2 > MAX_DEPTH) {
            return Err("The design supports up to 16 levels of nesting".into());
        }
        self.begin();
        let id = self.document.next_id();
        let offset = parent
            .and_then(|id| self.page().world_rect(id))
            .unwrap_or([0.; 4]);
        let mut node = Node::new(id, kind, world[0] - offset[0], world[1] - offset[1]);
        if !node.x.is_finite()
            || !node.y.is_finite()
            || node.x.abs() > 16000.
            || node.y.abs() > 16000.
            || !world[2].is_finite()
            || !world[3].is_finite()
        {
            return Err("Place the element within 16000 px of its parent origin".into());
        }
        node.parent = parent;
        node.width = world[2].clamp(1., 16000.);
        node.height = world[3].clamp(1., 16000.);
        self.page_mut().nodes.push(node);
        self.selection = vec![id];
        self.page_mut().relayout();
        Ok(id)
    }
    pub fn component_size(action: crate::actions::UiAction) -> [f32; 2] {
        match action {
            crate::actions::UiAction::Composer => [280., 44.],
            crate::actions::UiAction::Conversation => [640., 360.],
            _ => [140., 44.],
        }
    }
    pub fn add_component(
        &mut self,
        action: crate::actions::UiAction,
        world: [f32; 2],
        parent: Option<u32>,
    ) -> Result<u32, String> {
        use crate::actions::UiAction;
        let kind = match action {
            UiAction::Composer => Kind::Input,
            UiAction::Conversation => Kind::Frame,
            _ => Kind::Button,
        };
        let [width, height] = Self::component_size(action);
        let id = self.add(kind, [world[0], world[1], width, height], parent)?;
        let node = self.page_mut().node_mut(id).unwrap();
        node.name = action.label().into();
        node.text = match action {
            UiAction::Composer => "Ask anything".into(),
            UiAction::Conversation => "Send a message to start the conversation".into(),
            _ => action.label().into(),
        };
        node.ui_script = vec![action];
        if action == UiAction::Conversation {
            node.radius = 8.;
        }
        Ok(id)
    }
    pub fn add_page(&mut self) -> Result<(), String> {
        if self.document.pages.len() >= MAX_PAGES {
            return Err("Design has reached the 12 page limit".into());
        }
        self.begin();
        self.document.pages.push(Page::blank(&format!(
            "Page {}",
            self.document.pages.len() + 1
        )));
        self.page = self.document.pages.len() - 1;
        self.selection.clear();
        Ok(())
    }
    pub fn delete_page(&mut self) {
        if self.document.pages.len() <= 1 {
            return;
        }
        self.begin();
        self.document.pages.remove(self.page);
        self.page = self.page.min(self.document.pages.len() - 1);
        self.selection.clear();
    }
    pub fn delete(&mut self) {
        let roots = self.roots();
        if roots.is_empty() {
            return;
        }
        self.begin();
        let removed: HashSet<_> = self
            .page()
            .nodes
            .iter()
            .filter(|n| {
                roots.contains(&n.id)
                    || self
                        .page()
                        .ancestors(n.id)
                        .iter()
                        .any(|id| roots.contains(id))
            })
            .map(|n| n.id)
            .collect();
        self.page_mut().nodes.retain(|n| !removed.contains(&n.id));
        self.selection.clear();
        self.page_mut().relayout();
    }
    pub fn copy(&mut self) {
        let roots = self.roots();
        self.clipboard = self
            .page()
            .scene_order()
            .iter()
            .filter_map(|id| {
                let node = self.page().node(*id)?;
                if !roots.contains(id)
                    && !self.page().ancestors(*id).iter().any(|p| roots.contains(p))
                {
                    return None;
                }
                let mut node = node.clone();
                if roots.contains(id) {
                    let rect = self.page().world_rect(*id)?;
                    node.x = rect[0];
                    node.y = rect[1];
                    node.parent = None;
                }
                Some(node)
            })
            .collect();
    }
    pub fn paste(&mut self) -> Result<(), String> {
        if self.clipboard.is_empty() {
            return Ok(());
        }
        if self.document.node_count() + self.clipboard.len() > MAX_NODES {
            return Err("Pasting would exceed the 1000 layer limit".into());
        }
        self.begin();
        let mut used: HashSet<_> = self
            .document
            .pages
            .iter()
            .flat_map(|p| &p.nodes)
            .map(|n| n.id)
            .collect();
        let mut ids = HashMap::new();
        let mut next = 1;
        for node in &self.clipboard {
            while used.contains(&next) {
                next += 1;
            }
            ids.insert(node.id, next);
            used.insert(next);
        }
        let mut roots = vec![];
        for mut node in self.clipboard.clone() {
            node.id = ids[&node.id];
            node.parent = node.parent.and_then(|id| ids.get(&id).copied());
            if node.parent.is_none() {
                node.x = (node.x + 24.).min(16000.);
                node.y = (node.y + 24.).min(16000.);
                roots.push(node.id);
            }
            self.page_mut().nodes.push(node);
        }
        self.selection = roots;
        Ok(())
    }
    pub fn duplicate(&mut self) -> Result<(), String> {
        let parents: Vec<_> = self
            .roots()
            .iter()
            .map(|id| self.page().node(*id).unwrap().parent)
            .collect();
        let clipboard = self.clipboard.clone();
        self.copy();
        let result = self.paste();
        self.clipboard = clipboard;
        if result.is_ok() {
            for (id, parent) in self.selection.clone().into_iter().zip(parents) {
                self.reparent(id, parent)?;
            }
        }
        result
    }
    pub fn group(&mut self) -> Result<(), String> {
        let roots = self.roots();
        if roots.is_empty() {
            return Ok(());
        }
        let rect = union(roots.iter().filter_map(|id| self.page().world_rect(*id))).unwrap();
        let positions: Vec<_> = roots
            .iter()
            .map(|id| (*id, self.page().world_rect(*id).unwrap()))
            .collect();
        let common_parent = self.page().node(roots[0]).and_then(|n| n.parent);
        let group_index = self
            .page()
            .nodes
            .iter()
            .enumerate()
            .filter(|(_, n)| roots.contains(&n.id))
            .map(|(i, _)| i)
            .max()
            .unwrap();
        let parent = if roots.iter().all(|id| {
            self.page()
                .node(*id)
                .is_some_and(|n| n.parent == common_parent)
        }) {
            common_parent
        } else {
            None
        };
        let id = self.add(Kind::Group, rect, parent)?;
        for (child, position) in positions {
            if let Some(node) = self.page_mut().node_mut(child) {
                node.parent = Some(id);
                node.x = position[0] - rect[0];
                node.y = position[1] - rect[1];
            }
        }
        if let Err(error) = self.document.validate() {
            self.cancel();
            return Err(error);
        }
        let group = self.page_mut().nodes.pop().unwrap();
        self.page_mut().nodes.insert(group_index, group);
        Ok(())
    }
    pub fn ungroup(&mut self) {
        let groups: Vec<_> = self
            .roots()
            .into_iter()
            .filter(|id| self.page().node(*id).is_some_and(|n| n.kind == Kind::Group))
            .collect();
        self.begin();
        let mut children = vec![];
        for id in groups {
            let group = self.page().node(id).unwrap().clone();
            let mut extracted = vec![];
            for child in self.page().children(Some(id)) {
                let position = self
                    .page()
                    .nodes
                    .iter()
                    .position(|n| n.id == child)
                    .unwrap();
                let mut node = self.page_mut().nodes.remove(position);
                node.parent = group.parent;
                node.x += group.x;
                node.y += group.y;
                children.push(node.id);
                extracted.push(node);
            }
            let index = self.page().nodes.iter().position(|n| n.id == id).unwrap();
            self.page_mut().nodes.remove(index);
            let index = index.min(self.page().nodes.len());
            self.page_mut().nodes.splice(index..index, extracted);
        }
        self.selection = children;
        self.page_mut().relayout();
    }
    pub fn reparent(&mut self, id: u32, parent: Option<u32>) -> Result<(), String> {
        if !self.page().editable(id) {
            return Err("Unlock and show the layer before moving it".into());
        }
        if parent == Some(id) || parent.is_some_and(|p| self.page().ancestors(p).contains(&id)) {
            return Err("A layer cannot contain itself".into());
        }
        if parent.is_some_and(|p| {
            !self
                .page()
                .node(p)
                .is_some_and(|n| n.kind.container() && self.page().editable(p))
        }) {
            return Err("Choose an unlocked frame or group".into());
        }
        self.begin();
        let world = self.page().world_rect(id).ok_or("Layer is missing")?;
        let offset = parent
            .and_then(|p| self.page().world_rect(p))
            .unwrap_or([0.; 4]);
        let before = self.page().node(id).unwrap().clone();
        if let Some(node) = self.page_mut().node_mut(id) {
            node.parent = parent;
            node.x = world[0] - offset[0];
            node.y = world[1] - offset[1];
        }
        if let Err(error) = self.document.validate() {
            *self.page_mut().node_mut(id).unwrap() = before;
            return Err(error);
        }
        self.page_mut().relayout();
        Ok(())
    }
    pub fn reorder(&mut self, front: bool, extreme: bool) {
        self.begin();
        let selected: HashSet<_> = self.roots().into_iter().collect();
        let parents: HashSet<_> = selected
            .iter()
            .filter_map(|id| self.page().node(*id).map(|n| n.parent))
            .collect();
        for parent in parents {
            let slots: Vec<_> = self
                .page()
                .nodes
                .iter()
                .enumerate()
                .filter(|(_, n)| n.parent == parent)
                .map(|(index, _)| index)
                .collect();
            let mut siblings: Vec<_> = slots
                .iter()
                .map(|index| self.page().nodes[*index].clone())
                .collect();
            if extreme {
                siblings.sort_by_key(|node| selected.contains(&node.id) == front);
            } else if front {
                for index in (0..siblings.len().saturating_sub(1)).rev() {
                    if selected.contains(&siblings[index].id)
                        && !selected.contains(&siblings[index + 1].id)
                    {
                        siblings.swap(index, index + 1);
                    }
                }
            } else {
                for index in 1..siblings.len() {
                    if selected.contains(&siblings[index].id)
                        && !selected.contains(&siblings[index - 1].id)
                    {
                        siblings.swap(index, index - 1);
                    }
                }
            }
            for (slot, node) in slots.into_iter().zip(siblings) {
                self.page_mut().nodes[slot] = node;
            }
        }
        self.page_mut().relayout();
    }
    pub fn align(&mut self, alignment: Alignment) {
        let roots = self.roots();
        let Some(mut target) = union(roots.iter().filter_map(|id| self.page().world_rect(*id)))
        else {
            return;
        };
        if roots.len() == 1 {
            let Some(parent) = self
                .page()
                .node(roots[0])
                .and_then(|n| n.parent)
                .and_then(|id| self.page().world_rect(id))
            else {
                return;
            };
            target = parent;
        }
        self.begin();
        for id in roots {
            let r = self.page().world_rect(id).unwrap();
            let (dx, dy) = match alignment {
                Alignment::Left => (target[0] - r[0], 0.),
                Alignment::Center => (target[0] + (target[2] - r[2]) * 0.5 - r[0], 0.),
                Alignment::Right => (target[0] + target[2] - r[0] - r[2], 0.),
                Alignment::Top => (0., target[1] - r[1]),
                Alignment::Middle => (0., target[1] + (target[3] - r[3]) * 0.5 - r[1]),
                Alignment::Bottom => (0., target[1] + target[3] - r[1] - r[3]),
            };
            if let Some(node) = self.page_mut().node_mut(id) {
                node.x = (node.x + dx).clamp(-16000., 16000.);
                node.y = (node.y + dy).clamp(-16000., 16000.);
            }
        }
        self.page_mut().relayout();
    }
    pub fn nudge(&mut self, dx: f32, dy: f32) {
        self.begin();
        for id in self.roots() {
            if let Some(node) = self.page_mut().node_mut(id) {
                node.x = (node.x + dx).clamp(-16000., 16000.);
                node.y = (node.y + dy).clamp(-16000., 16000.);
            }
        }
        self.page_mut().relayout();
    }
}

pub fn union(rects: impl Iterator<Item = [f32; 4]>) -> Option<[f32; 4]> {
    let mut output: Option<[f32; 4]> = None;
    for rect in rects {
        output = Some(match output {
            None => rect,
            Some(previous) => {
                let x = previous[0].min(rect[0]);
                let y = previous[1].min(rect[1]);
                [
                    x,
                    y,
                    (previous[0] + previous[2]).max(rect[0] + rect[2]) - x,
                    (previous[1] + previous[3]).max(rect[1] + rect[3]) - y,
                ]
            }
        });
    }
    output
}
