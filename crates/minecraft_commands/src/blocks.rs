use crate::args::{Arg, ArgKind, Coordinates, nbt, resource_id};
use crate::permissions::GAMEMASTERS;
use crate::reader::{CommandError, StringReader, literal};
use crate::server::{Answer, CommandData, Ctx};
use crate::text::Text;
use crate::tree::Node;

pub const AIR: [&str; 3] = ["minecraft:air", "minecraft:cave_air", "minecraft:void_air"];

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BlockState {
    pub id: String,
    pub properties: Vec<(String, String)>,
}

impl BlockState {
    pub fn is_air(&self) -> bool {
        AIR.contains(&self.id.as_str())
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BlockInput {
    pub id: String,
    pub properties: Vec<(String, String)>,
    pub nbt: Option<String>,
}

impl BlockInput {
    pub fn air() -> Self {
        Self {
            id: AIR[0].to_owned(),
            ..Self::default()
        }
    }

    pub fn is_air(&self) -> bool {
        AIR.contains(&self.id.as_str())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedProperty {
    pub name: String,
    pub value: String,
    pub name_at: usize,
    pub value_at: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedBlock {
    pub id: String,
    pub tag: bool,
    pub properties: Vec<ParsedProperty>,
    pub nbt: Option<String>,
    pub at: usize,
}

pub fn parse_block(reader: &mut StringReader, tags: bool) -> Result<ParsedBlock, CommandError> {
    let at = reader.cursor;
    let tag = reader.peek() == Some('#');
    if tag && !tags {
        return Err(CommandError::new("argument.block.tag.disallowed", Vec::new()).at(at));
    }
    if tag {
        reader.skip();
    }
    let id = resource_id(reader)?;
    let mut properties = Vec::new();
    if reader.peek() == Some('[') {
        read_properties(reader, &id, tag, &mut properties)?;
    }
    let nbt = if reader.peek() == Some('{') {
        Some(nbt(reader)?)
    } else {
        None
    };
    Ok(ParsedBlock {
        id,
        tag,
        properties,
        nbt,
        at,
    })
}

fn read_properties(
    reader: &mut StringReader,
    id: &str,
    tag: bool,
    out: &mut Vec<ParsedProperty>,
) -> Result<(), CommandError> {
    let unclosed = |reader: &StringReader| {
        CommandError::new("argument.block.property.unclosed", Vec::new()).at(reader.cursor)
    };
    reader.skip();
    reader.skip_ws();
    while reader.can_read() && reader.peek() != Some(']') {
        reader.skip_ws();
        let name_at = reader.cursor;
        let name = reader.read_string()?;
        if tag && out.iter().any(|known| known.name == name) {
            reader.cursor = name_at;
            return Err(CommandError::new(
                "argument.block.property.duplicate",
                vec![literal(&name), literal(id)],
            )
            .at(name_at));
        }
        reader.skip_ws();
        if reader.peek() != Some('=') {
            if tag {
                reader.cursor = name_at;
            }
            return Err(CommandError::new(
                "argument.block.property.novalue",
                vec![literal(&name), literal(id)],
            )
            .at(reader.cursor));
        }
        reader.skip();
        reader.skip_ws();
        let value_at = reader.cursor;
        let value = reader.read_string()?;
        out.push(ParsedProperty {
            name,
            value,
            name_at,
            value_at,
        });
        reader.skip_ws();
        match reader.peek() {
            Some(',') => reader.skip(),
            Some(']') => break,
            Some(_) => return Err(unclosed(reader)),
            None => {}
        }
    }
    if !reader.can_read() {
        return Err(unclosed(reader));
    }
    reader.skip();
    Ok(())
}

pub fn resolve_block(data: &CommandData, parsed: &ParsedBlock) -> Result<BlockInput, CommandError> {
    let input = BlockInput {
        id: parsed.id.clone(),
        properties: parsed
            .properties
            .iter()
            .map(|property| (property.name.clone(), property.value.clone()))
            .collect(),
        nbt: parsed.nbt.clone(),
    };
    if data.blocks.is_empty() {
        return Ok(input);
    }
    let Some(known) = data.blocks.get(&parsed.id) else {
        return Err(
            CommandError::new("argument.block.id.invalid", vec![literal(&parsed.id)]).at(parsed.at),
        );
    };
    for (index, property) in parsed.properties.iter().enumerate() {
        let Some((_, values)) = known.iter().find(|(name, _)| *name == property.name) else {
            return Err(CommandError::new(
                "argument.block.property.unknown",
                vec![literal(&parsed.id), literal(&property.name)],
            )
            .at(property.name_at));
        };
        if parsed.properties[..index]
            .iter()
            .any(|earlier| earlier.name == property.name)
        {
            return Err(CommandError::new(
                "argument.block.property.duplicate",
                vec![literal(&property.name), literal(&parsed.id)],
            )
            .at(property.name_at));
        }
        if !values.contains(&property.value) {
            return Err(CommandError::new(
                "argument.block.property.invalid",
                vec![
                    literal(&parsed.id),
                    literal(&property.value),
                    literal(&property.name),
                ],
            )
            .at(property.value_at));
        }
    }
    Ok(input)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Loaded {
    Yes,
    Unloaded,
    OutOfWorld,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FillMode {
    Replace,
    Destroy,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BlockFilter {
    Air,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditKind {
    SetBlock,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockEdit {
    pub kind: EditKind,
    pub dimension: String,
    pub from: [i32; 3],
    pub to: [i32; 3],
    pub block: BlockInput,
    pub mode: FillMode,
    pub filter: Option<BlockFilter>,
    pub strict: bool,
}

pub trait BlockAccess {
    fn state(&self, pos: [i32; 3]) -> BlockState;
    fn destroy(&mut self, pos: [i32; 3]) -> bool;
    fn place(&mut self, pos: [i32; 3], block: &BlockInput, strict: bool) -> bool;
    fn update_neighbours(&mut self, _pos: [i32; 3], _old: &BlockState) {}
}

fn passes(filter: Option<&BlockFilter>, access: &dyn BlockAccess, pos: [i32; 3]) -> bool {
    match filter {
        None => true,
        Some(BlockFilter::Air) => access.state(pos).is_air(),
    }
}

pub fn apply_edit(edit: &BlockEdit, access: &mut dyn BlockAccess) -> i32 {
    let pos = edit.from;
    if !passes(edit.filter.as_ref(), access, pos) {
        return 0;
    }
    let place = if edit.mode == FillMode::Destroy {
        access.destroy(pos);
        !edit.block.is_air() || !access.state(pos).is_air()
    } else {
        true
    };
    let old = access.state(pos);
    if place && !access.place(pos, &edit.block, edit.strict) {
        return 0;
    }
    if !edit.strict {
        access.update_neighbours(pos, &old);
    }
    1
}

pub fn edit_answer(edit: &BlockEdit, count: i32) -> Result<Text, CommandError> {
    if count == 0 {
        return Err(CommandError::new("commands.setblock.failed", Vec::new()));
    }
    let [x, y, z] = edit.from;
    Ok(Text::translate(
        "commands.setblock.success",
        vec![literal(x), literal(y), literal(z)],
    ))
}

pub(crate) fn loaded_pos(ctx: &Ctx, name: &str) -> Result<[i32; 3], CommandError> {
    let Some(Arg::BlockPos(coordinates)) = ctx.arg(name).cloned() else {
        return Err(CommandError::new("command.unknown.argument", Vec::new()));
    };
    let pos = block_pos(&coordinates, ctx);
    match ctx.world.loaded(&ctx.dimension(), pos) {
        Loaded::Yes => Ok(pos),
        Loaded::Unloaded => Err(CommandError::new("argument.pos.unloaded", Vec::new())),
        Loaded::OutOfWorld => Err(CommandError::new("argument.pos.outofworld", Vec::new())),
    }
}

pub(crate) fn block_pos(coordinates: &Coordinates, ctx: &Ctx) -> [i32; 3] {
    coordinates
        .position(ctx.pose())
        .map(|value| value.floor() as i32)
}

pub(crate) fn block_arg(ctx: &Ctx, name: &str) -> Result<BlockInput, CommandError> {
    let Some(Arg::Block(parsed)) = ctx.arg(name) else {
        return Err(CommandError::new("command.unknown.argument", Vec::new()));
    };
    resolve_block(ctx.server.data(), parsed)
}

pub(crate) fn run_edit(ctx: &mut Ctx, edit: BlockEdit) -> Result<i32, CommandError> {
    match ctx.world.edit_blocks(ctx.source, &edit) {
        Answer::Now(count) => {
            let text = edit_answer(&edit, count)?;
            ctx.success(text, true);
            Ok(count)
        }
        Answer::Later => Ok(0),
    }
}

pub(crate) fn setblock_node() -> Node {
    Node::literal("setblock").requires(GAMEMASTERS).then(
        Node::argument("pos", ArgKind::BlockPos).then(
            Node::argument("block", ArgKind::BlockState)
                .executes(setblock)
                .then(Node::literal("destroy").executes(setblock))
                .then(Node::literal("keep").executes(setblock))
                .then(Node::literal("replace").executes(setblock))
                .then(Node::literal("strict").executes(setblock)),
        ),
    )
}

fn setblock(ctx: &mut Ctx) -> Result<i32, CommandError> {
    let block = block_arg(ctx, "block")?;
    let pos = loaded_pos(ctx, "pos")?;
    let last = ctx.literals.last().copied();
    let edit = BlockEdit {
        kind: EditKind::SetBlock,
        dimension: ctx.dimension(),
        from: pos,
        to: pos,
        block,
        mode: if last == Some("destroy") {
            FillMode::Destroy
        } else {
            FillMode::Replace
        },
        filter: (last == Some("keep")).then_some(BlockFilter::Air),
        strict: last == Some("strict"),
    };
    run_edit(ctx, edit)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::server::tests::{FakeWorld, HOST, run, server, texts};
    use crate::text::RED;
    use std::collections::BTreeMap;

    #[derive(Debug, Default)]
    pub(crate) struct Blocks {
        pub(crate) states: BTreeMap<[i32; 3], BlockState>,
        pub(crate) destroyed: Vec<[i32; 3]>,
        pub(crate) updated: Vec<[i32; 3]>,
    }

    pub(crate) fn state(id: &str) -> BlockState {
        BlockState {
            id: id.to_owned(),
            properties: Vec::new(),
        }
    }

    impl BlockAccess for Blocks {
        fn state(&self, pos: [i32; 3]) -> BlockState {
            self.states
                .get(&pos)
                .cloned()
                .unwrap_or_else(|| state("minecraft:air"))
        }

        fn destroy(&mut self, pos: [i32; 3]) -> bool {
            if self.state(pos).is_air() {
                return false;
            }
            self.destroyed.push(pos);
            self.states.remove(&pos);
            true
        }

        fn place(&mut self, pos: [i32; 3], block: &BlockInput, _strict: bool) -> bool {
            let placed = BlockState {
                id: block.id.clone(),
                properties: block.properties.clone(),
            };
            if self.state(pos) == placed {
                return false;
            }
            if placed.is_air() {
                self.states.remove(&pos);
            } else {
                self.states.insert(pos, placed);
            }
            true
        }

        fn update_neighbours(&mut self, pos: [i32; 3], _old: &BlockState) {
            self.updated.push(pos);
        }
    }

    pub(crate) fn data() -> CommandData {
        let mut data = CommandData::default();
        let values = |list: &[&str]| list.iter().map(|value| (*value).to_owned()).collect();
        data.blocks.insert("minecraft:stone".into(), Vec::new());
        data.blocks.insert("minecraft:air".into(), Vec::new());
        data.blocks.insert("minecraft:glass".into(), Vec::new());
        data.blocks
            .insert("minecraft:gold_block".into(), Vec::new());
        data.blocks.insert(
            "minecraft:oak_stairs".into(),
            vec![
                ("facing".into(), values(&["north", "south", "west", "east"])),
                ("half".into(), values(&["top", "bottom"])),
            ],
        );
        data
    }

    fn parse(text: &str, tags: bool) -> Result<ParsedBlock, CommandError> {
        parse_block(&mut StringReader::new(text), tags)
    }

    fn key(error: &CommandError) -> String {
        match &error.text.content {
            crate::text::Content::Translate { key, .. } => key.clone(),
            _ => String::new(),
        }
    }

    #[test]
    fn block_states_parse_properties_and_nbt_like_vanilla() {
        let stairs = parse("oak_stairs[ facing = east , half=top]{x:1}", false).unwrap();
        assert_eq!(stairs.id, "minecraft:oak_stairs");
        assert_eq!(
            stairs
                .properties
                .iter()
                .map(|p| (p.name.as_str(), p.value.as_str(), p.name_at, p.value_at))
                .collect::<Vec<_>>(),
            vec![("facing", "east", 12, 21), ("half", "top", 28, 33)]
        );
        assert_eq!(stairs.nbt.as_deref(), Some("{x:1}"));
        assert_eq!(parse("stone", false).unwrap().properties, Vec::new());
        assert_eq!(parse("stone[]", false).unwrap().properties, Vec::new());
        let tag = parse("#minecraft:logs", false).unwrap_err();
        assert_eq!(
            (key(&tag), tag.cursor),
            ("argument.block.tag.disallowed".into(), Some(0))
        );
        assert!(parse("#logs[axis=y]", true).unwrap().tag);
        let novalue = parse("oak_stairs[facing]", false).unwrap_err();
        assert_eq!(
            (key(&novalue), novalue.cursor),
            ("argument.block.property.novalue".into(), Some(17))
        );
        let unclosed = parse("oak_stairs[facing=east", false).unwrap_err();
        assert_eq!(key(&unclosed), "argument.block.property.unclosed");
        let unclosed = parse("oak_stairs[facing=east;", false).unwrap_err();
        assert_eq!(
            (key(&unclosed), unclosed.cursor),
            ("argument.block.property.unclosed".into(), Some(22))
        );
        let duplicate = parse("#stairs[half=top,half=top]", true).unwrap_err();
        assert_eq!(
            (key(&duplicate), duplicate.cursor),
            ("argument.block.property.duplicate".into(), Some(17))
        );
    }

    #[test]
    fn block_states_resolve_against_the_block_data() {
        let data = data();
        let resolve = |text: &str| resolve_block(&data, &parse(text, false).unwrap());
        assert_eq!(
            resolve("oak_stairs[half=top]").unwrap(),
            BlockInput {
                id: "minecraft:oak_stairs".into(),
                properties: vec![("half".into(), "top".into())],
                nbt: None,
            }
        );
        let unknown = resolve("stoen").unwrap_err();
        assert_eq!(
            (key(&unknown), unknown.cursor),
            ("argument.block.id.invalid".into(), Some(0))
        );
        let property = resolve("oak_stairs[shape=straight]").unwrap_err();
        assert_eq!(
            (key(&property), property.cursor),
            ("argument.block.property.unknown".into(), Some(11))
        );
        let value = resolve("oak_stairs[half=middle]").unwrap_err();
        assert_eq!(
            (key(&value), value.cursor),
            ("argument.block.property.invalid".into(), Some(16))
        );
        let twice = resolve("oak_stairs[half=top,half=bottom]").unwrap_err();
        assert_eq!(
            (key(&twice), twice.cursor),
            ("argument.block.property.duplicate".into(), Some(20))
        );
        let open = CommandData::default();
        assert!(resolve_block(&open, &parse("modded:thing[x=y]", false).unwrap()).is_ok());
    }

    fn edit(block: &str, mode: FillMode, filter: Option<BlockFilter>, strict: bool) -> BlockEdit {
        BlockEdit {
            kind: EditKind::SetBlock,
            dimension: "minecraft:overworld".into(),
            from: [1, 2, 3],
            to: [1, 2, 3],
            block: BlockInput {
                id: block.into(),
                ..BlockInput::default()
            },
            mode,
            filter,
            strict,
        }
    }

    #[test]
    fn setblock_places_destroys_keeps_and_skips_updates_when_strict() {
        let mut blocks = Blocks::default();
        let stone = edit("minecraft:stone", FillMode::Replace, None, false);
        assert_eq!(apply_edit(&stone, &mut blocks), 1);
        assert_eq!(blocks.updated, vec![[1, 2, 3]]);
        assert_eq!(apply_edit(&stone, &mut blocks), 0);
        let keep = edit(
            "minecraft:glass",
            FillMode::Replace,
            Some(BlockFilter::Air),
            false,
        );
        assert_eq!(apply_edit(&keep, &mut blocks), 0);
        let destroy = edit("minecraft:stone", FillMode::Destroy, None, false);
        assert_eq!(apply_edit(&destroy, &mut blocks), 1);
        assert_eq!(blocks.destroyed, vec![[1, 2, 3]]);
        let clear = edit("minecraft:air", FillMode::Destroy, None, false);
        assert_eq!(apply_edit(&clear, &mut blocks), 1);
        assert_eq!(apply_edit(&clear, &mut blocks), 1);
        assert!(blocks.states.is_empty());
        assert_eq!(apply_edit(&keep, &mut blocks), 1);
        let strict = edit("minecraft:stone", FillMode::Replace, None, true);
        let updates = blocks.updated.len();
        assert_eq!(apply_edit(&strict, &mut blocks), 1);
        assert_eq!(blocks.updated.len(), updates);
    }

    fn world() -> FakeWorld {
        FakeWorld {
            loaded: Some(Loaded::Yes),
            ..FakeWorld::default()
        }
    }

    #[test]
    fn setblock_answers_like_vanilla() {
        let mut server = server();
        *server.data_mut() = data();
        let mut world = world();
        world.poses.push((
            HOST,
            crate::server::Pose {
                feet: [10.7, 64.0, -3.2],
                yaw: 0.0,
                pitch: 0.0,
            },
        ));
        let out = run(&mut server, &mut world, HOST, "setblock ~ ~-1 ~ gold_block");
        assert_eq!(texts(&out), vec!["Changed the block at 10, 63, -4"]);
        assert_eq!(world.edits[0].block.id, "minecraft:gold_block");
        let out = run(
            &mut server,
            &mut world,
            HOST,
            "setblock 10 63 -4 gold_block",
        );
        assert_eq!(texts(&out), vec!["Could not set the block"]);
        assert_eq!(out[0].2, RED);
        let out = run(
            &mut server,
            &mut world,
            HOST,
            "setblock 10 63 -4 stone keep",
        );
        assert_eq!(texts(&out), vec!["Could not set the block"]);
        let out = run(
            &mut server,
            &mut world,
            HOST,
            "setblock 10 63 -4 stone destroy",
        );
        assert_eq!(texts(&out), vec!["Changed the block at 10, 63, -4"]);
        assert_eq!(world.blocks.destroyed, vec![[10, 63, -4]]);
        let out = run(
            &mut server,
            &mut world,
            HOST,
            "setblock 0 0 0 oak_stairs[half=up] strict",
        );
        assert_eq!(
            texts(&out),
            vec![
                "Block minecraft:oak_stairs does not accept 'up' for half property",
                "...airs[half=up] strict<--[HERE]"
            ]
        );
        let out = run(&mut server, &mut world, HOST, "setblock 1.5 0 0 stone");
        assert_eq!(texts(&out)[0], "Invalid integer '1.5'");
        let out = run(&mut server, &mut world, HOST, "setblock 1 2");
        assert_eq!(texts(&out)[0], "Incomplete (expected 3 coordinates)");
        let out = run(&mut server, &mut world, HOST, "setblock ^ ^ ^1 #logs");
        assert_eq!(
            texts(&out)[0],
            "Tags aren't allowed here; only actual blocks are"
        );
        world.loaded = Some(Loaded::Unloaded);
        let out = run(&mut server, &mut world, HOST, "setblock 9999 64 0 stone");
        assert_eq!(texts(&out), vec!["That position is not loaded"]);
        world.loaded = Some(Loaded::OutOfWorld);
        let out = run(&mut server, &mut world, HOST, "setblock 0 999 0 stone");
        assert_eq!(texts(&out), vec!["That position is out of this world!"]);
        let out = run(
            &mut server,
            &mut FakeWorld::default(),
            HOST,
            "setblock 0 0 0 stone",
        );
        assert_eq!(texts(&out), vec!["That position is not loaded"]);
    }
}
