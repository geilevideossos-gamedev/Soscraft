use crate::args::{Arg, ArgKind};
use crate::reader::{CommandError, StringReader};
use crate::server::Ctx;

pub type Exec = fn(&mut Ctx) -> Result<i32, CommandError>;
pub type Modifier = fn(&mut Ctx) -> Result<(), CommandError>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Redirect {
    Root,
    Command(&'static str),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeKind {
    Literal(&'static str),
    Argument(&'static str, ArgKind),
}

#[derive(Clone, Debug)]
pub struct Node {
    pub kind: NodeKind,
    pub permission: u8,
    pub children: Vec<Node>,
    pub exec: Option<Exec>,
    pub redirect: Option<Redirect>,
    pub modifier: Option<Modifier>,
}

impl Node {
    fn of(kind: NodeKind) -> Self {
        Self {
            kind,
            permission: 0,
            children: Vec::new(),
            exec: None,
            redirect: None,
            modifier: None,
        }
    }

    pub fn literal(name: &'static str) -> Self {
        Self::of(NodeKind::Literal(name))
    }

    pub fn argument(name: &'static str, kind: ArgKind) -> Self {
        Self::of(NodeKind::Argument(name, kind))
    }

    pub fn requires(mut self, level: u8) -> Self {
        self.permission = level;
        self
    }

    pub fn executes(mut self, exec: Exec) -> Self {
        self.exec = Some(exec);
        self
    }

    pub fn then(mut self, child: Node) -> Self {
        self.children.push(child);
        self
    }

    pub fn redirect(mut self, target: Redirect) -> Self {
        self.redirect = Some(target);
        self
    }

    pub fn redirect_with(mut self, target: Redirect, modifier: Modifier) -> Self {
        self.redirect = Some(target);
        self.modifier = Some(modifier);
        self
    }

    pub fn name(&self) -> &'static str {
        match self.kind {
            NodeKind::Literal(name) | NodeKind::Argument(name, _) => name,
        }
    }

    pub fn usage_text(&self) -> String {
        match self.kind {
            NodeKind::Literal(name) => name.to_owned(),
            NodeKind::Argument(name, _) => format!("<{name}>"),
        }
    }

    pub fn visible(&self, level: u8) -> bool {
        self.permission <= level
    }

    pub fn is_literal(&self) -> bool {
        matches!(self.kind, NodeKind::Literal(_))
    }
}

pub fn sorted(mut nodes: Vec<Node>) -> Vec<Node> {
    nodes.sort_by(|a, b| {
        b.is_literal()
            .cmp(&a.is_literal())
            .then_with(|| a.name().cmp(b.name()))
    });
    for node in &mut nodes {
        node.children = sorted(std::mem::take(&mut node.children));
    }
    nodes
}

#[derive(Clone, Debug)]
pub struct Stage<'t> {
    pub root: Option<&'t Node>,
    pub start: usize,
    pub nodes: usize,
    pub args: usize,
    pub literals: usize,
}

#[derive(Clone, Debug)]
pub struct Parsed<'t> {
    pub nodes: Vec<&'t Node>,
    pub ranges: Vec<std::ops::Range<usize>>,
    pub args: Vec<(&'static str, Arg)>,
    pub literals: Vec<&'static str>,
    pub cursor: usize,
    pub errors: Vec<CommandError>,
    pub stages: Vec<Stage<'t>>,
}

impl Parsed<'_> {
    pub fn stage_nodes(&self, index: usize) -> std::ops::Range<usize> {
        let start = self.stages.get(index).map_or(self.nodes.len(), |stage| stage.nodes);
        let end = self
            .stages
            .get(index + 1)
            .map_or(self.nodes.len(), |next| next.nodes);
        start..end
    }
}

pub fn parse<'t>(roots: &'t [Node], input: &str, level: u8) -> Parsed<'t> {
    parse_from(roots, input, 0, level)
}

pub fn parse_from<'t>(roots: &'t [Node], input: &str, from: usize, level: u8) -> Parsed<'t> {
    let start = Parsed {
        nodes: Vec::new(),
        ranges: Vec::new(),
        args: Vec::new(),
        literals: Vec::new(),
        cursor: from,
        errors: Vec::new(),
        stages: vec![Stage {
            root: None,
            start: from,
            nodes: 0,
            args: 0,
            literals: 0,
        }],
    };
    parse_nodes(roots, roots, input, from, level, start)
}

fn redirect_target(roots: &[Node], target: Redirect) -> Option<&Node> {
    match target {
        Redirect::Root => None,
        Redirect::Command(name) => roots.iter().find(|node| node.name() == name),
    }
}

fn relevant<'t>(children: &'t [Node], input: &str, start: usize) -> Vec<&'t Node> {
    let word = StringReader::at(input, start).read_word();
    let literal: Vec<&Node> = children
        .iter()
        .filter(|child| matches!(child.kind, NodeKind::Literal(name) if name == word))
        .collect();
    if literal.is_empty() {
        children
            .iter()
            .filter(|child| !child.is_literal())
            .collect()
    } else {
        literal
    }
}

fn parse_nodes<'t>(
    roots: &'t [Node],
    children: &'t [Node],
    input: &str,
    start: usize,
    level: u8,
    mut context: Parsed<'t>,
) -> Parsed<'t> {
    let mut potentials = Vec::new();
    let mut errors = Vec::new();
    for child in relevant(children, input, start) {
        if !child.visible(level) {
            continue;
        }
        let mut reader = StringReader::at(input, start);
        let mut next = context.clone();
        let step = match child.kind {
            NodeKind::Literal(name) => {
                reader.cursor += name.len();
                next.literals.push(name);
                Ok(())
            }
            NodeKind::Argument(name, kind) => kind
                .parse(&mut reader, &next.args)
                .map(|arg| next.args.push((name, arg))),
        };
        let step = step.and_then(|()| {
            if reader.can_read() && reader.peek() != Some(' ') {
                Err(CommandError::new("command.expected.separator", Vec::new()).at(reader.cursor))
            } else {
                Ok(())
            }
        });
        if let Err(error) = step {
            errors.push(error);
            continue;
        }
        next.nodes.push(child);
        next.ranges.push(start..reader.cursor);
        next.cursor = reader.cursor;
        next.errors.clear();
        if let Some(target) = child.redirect {
            if reader.cursor < input.len() {
                let root = redirect_target(roots, target);
                let from = reader.cursor + 1;
                next.stages.push(Stage {
                    root,
                    start: from,
                    nodes: next.nodes.len(),
                    args: next.args.len(),
                    literals: next.literals.len(),
                });
                let children = root.map_or(roots, |node| node.children.as_slice());
                return parse_nodes(roots, children, input, from, level, next);
            }
            potentials.push(next);
        } else if input.len().saturating_sub(reader.cursor) >= 2 {
            potentials.push(parse_nodes(
                roots,
                &child.children,
                input,
                reader.cursor + 1,
                level,
                next,
            ));
        } else {
            potentials.push(next);
        }
    }
    if potentials.is_empty() {
        context.cursor = start;
        context.errors = errors;
        return context;
    }
    potentials.sort_by_key(|parsed| (parsed.cursor < input.len(), !parsed.errors.is_empty()));
    potentials.swap_remove(0)
}

pub fn runnable(parsed: &Parsed, input: &str) -> Result<Exec, CommandError> {
    if parsed.cursor < input.len() {
        if parsed.errors.len() == 1 {
            return Err(parsed.errors[0].clone());
        }
        let key = if parsed.nodes.is_empty() {
            "command.unknown.command"
        } else {
            "command.unknown.argument"
        };
        return Err(CommandError::new(key, Vec::new()).at(parsed.cursor));
    }
    parsed
        .nodes
        .last()
        .and_then(|node| node.exec)
        .ok_or_else(|| CommandError::new("command.unknown.command", Vec::new()).at(parsed.cursor))
}

pub fn smart_usage(node: &Node, level: u8, optional: bool, deep: bool) -> Option<String> {
    if !node.visible(level) {
        return None;
    }
    let me = if optional {
        format!("[{}]", node.usage_text())
    } else {
        node.usage_text()
    };
    let child_optional = node.exec.is_some();
    let (open, close) = if child_optional {
        ("[", "]")
    } else {
        ("(", ")")
    };
    if !deep && let Some(target) = node.redirect {
        return Some(match target {
            Redirect::Root => format!("{me} ..."),
            Redirect::Command(name) => format!("{me} -> {name}"),
        });
    }
    if !deep {
        let children: Vec<&Node> = node
            .children
            .iter()
            .filter(|child| child.visible(level))
            .collect();
        if let [only] = children.as_slice()
            && let Some(usage) = smart_usage(only, level, child_optional, child_optional)
        {
            return Some(format!("{me} {usage}"));
        } else if children.len() > 1 {
            let mut usages: Vec<String> = Vec::new();
            for child in &children {
                if let Some(usage) = smart_usage(child, level, child_optional, true)
                    && !usages.contains(&usage)
                {
                    usages.push(usage);
                }
            }
            if let [usage] = usages.as_slice() {
                let usage = if child_optional {
                    format!("[{usage}]")
                } else {
                    usage.clone()
                };
                return Some(format!("{me} {usage}"));
            } else if usages.len() > 1 {
                let choices: Vec<String> = children.iter().map(|c| c.usage_text()).collect();
                return Some(format!("{me} {open}{}{close}", choices.join("|")));
            }
        }
    }
    Some(me)
}

pub fn smart_usages(children: &[Node], has_exec: bool, level: u8) -> Vec<String> {
    children
        .iter()
        .filter_map(|child| smart_usage(child, level, has_exec, false))
        .collect()
}
