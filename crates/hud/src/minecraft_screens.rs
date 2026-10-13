use assets::PreparedWeapons;
use frame::{McOven, McScreen, McSlot, McStack, MinecraftUi};

use crate::minecraft_clicks::{Drag, Menu, preview, remainder};
use crate::minecraft_gui::{LABEL, Painter, TextLine, WHITE, YELLOW};

pub(crate) const PUPPET_BOX: [i32; 4] = [26, 8, 75, 78];
pub(crate) const RECIPE_COLUMNS: usize = 4;
pub(crate) const RECIPE_ROWS: usize = 3;
const BUBBLES: [i32; 7] = [29, 24, 20, 16, 11, 6, 0];
const CYCLE_TICKS: u32 = 30;
const SMITHING_TEMPLATES: [&str; 2] = [
    "container/slot/smithing_template_armor_trim",
    "container/slot/smithing_template_netherite_upgrade",
];
const TRIM_BASES: [&str; 4] = [
    "container/slot/helmet",
    "container/slot/chestplate",
    "container/slot/leggings",
    "container/slot/boots",
];
const TRIM_ADDITIONS: [&str; 7] = [
    "container/slot/ingot",
    "container/slot/redstone_dust",
    "container/slot/lapis_lazuli",
    "container/slot/quartz",
    "container/slot/diamond",
    "container/slot/emerald",
    "container/slot/amethyst_shard",
];
const NETHERITE_BASES: [&str; 11] = [
    "container/slot/helmet",
    "container/slot/sword",
    "container/slot/chestplate",
    "container/slot/pickaxe",
    "container/slot/leggings",
    "container/slot/axe",
    "container/slot/boots",
    "container/slot/hoe",
    "container/slot/shovel",
    "container/slot/nautilus_armor",
    "container/slot/spear",
];
const NETHERITE_ADDITIONS: [&str; 1] = ["container/slot/ingot"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Group {
    Player,
    Grid,
    Result,
    Cells,
    Output,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SlotSpec {
    pub slot: McSlot,
    pub x: i32,
    pub y: i32,
    pub group: Group,
    pub empty: Option<&'static str>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Layout {
    pub screen: McScreen,
    pub size: [i32; 2],
    pub slots: Vec<SlotSpec>,
}

pub(crate) const LETHAL: usize = 43;
pub(crate) const TACTICAL: usize = 44;
pub(crate) const GRENADE_SLOTS: [(usize, i32, i32); 2] = [(LETHAL, 134, 62), (TACTICAL, 152, 62)];

pub(crate) fn draw_grenade_frames(p: &mut Painter<'_>, origin: [i32; 2], slots: [(usize, i32, i32); 2]) {
    for (_, x, y) in slots {
        p.sprite("container/slot", origin[0] + x - 1, origin[1] + y - 1, 18, 18);
    }
}

pub(crate) fn grenade_slot_tooltip(ui: &MinecraftUi, hovered: Option<McSlot>) -> Option<(Vec<TextLine>, bool)> {
    let text = match hovered? {
        McSlot::Inventory(LETHAL) => "Lethal",
        McSlot::Inventory(TACTICAL) => "Tactical",
        _ => return None,
    };
    let empty = ui.cursor.is_none() && crate::minecraft_inventory::stack_at(ui, hovered?).is_none();
    empty.then(|| (vec![TextLine::new(text, WHITE)], false))
}

fn spec(slot: McSlot, x: i32, y: i32, group: Group) -> SlotSpec {
    SlotSpec {
        slot,
        x,
        y,
        group,
        empty: None,
    }
}

fn player_inventory(out: &mut Vec<SlotSpec>, top: i32) {
    for row in 0..3 {
        for column in 0..9 {
            out.push(spec(
                McSlot::Inventory(9 + row * 9 + column),
                8 + 18 * column as i32,
                top + 18 * row as i32,
                Group::Player,
            ));
        }
    }
    for column in 0..9 {
        out.push(spec(
            McSlot::Inventory(column),
            8 + 18 * column as i32,
            top + 58,
            Group::Player,
        ));
    }
}

pub(crate) fn layout(screen: McScreen) -> Layout {
    let mut slots = Vec::new();
    let cell = |i: usize, x: i32, y: i32| spec(McSlot::Container(i), x, y, Group::Cells);
    let output = |x: i32, y: i32| spec(McSlot::Output, x, y, Group::Output);
    let mut size = [176, 166];
    match screen {
        McScreen::Inventory => {
            slots.push(spec(McSlot::Result, 154, 28, Group::Result));
            for i in 0..4 {
                slots.push(spec(
                    McSlot::Crafting(i),
                    98 + 18 * (i % 2) as i32,
                    18 + 18 * (i / 2) as i32,
                    Group::Grid,
                ));
            }
            let icons = [
                "container/slot/helmet",
                "container/slot/chestplate",
                "container/slot/leggings",
                "container/slot/boots",
            ];
            for (i, icon) in icons.into_iter().enumerate() {
                slots.push(SlotSpec {
                    empty: Some(icon),
                    ..spec(
                        McSlot::Inventory(39 - i),
                        8,
                        8 + 18 * i as i32,
                        Group::Player,
                    )
                });
            }
            player_inventory(&mut slots, 84);
            slots.push(SlotSpec {
                empty: Some("container/slot/shield"),
                ..spec(McSlot::Inventory(40), 77, 62, Group::Player)
            });
            for (slot, x, y) in GRENADE_SLOTS {
                slots.push(spec(McSlot::Inventory(slot), x, y, Group::Player));
            }
        }
        McScreen::CraftingTable => {
            slots.push(spec(McSlot::WorkbenchResult, 124, 35, Group::Result));
            for i in 0..9 {
                slots.push(spec(
                    McSlot::Workbench(i),
                    30 + 18 * (i % 3) as i32,
                    17 + 18 * (i / 3) as i32,
                    Group::Grid,
                ));
            }
            player_inventory(&mut slots, 84);
        }
        McScreen::Chest { rows } => {
            let rows = i32::from(rows);
            size = [176, 114 + rows * 18];
            for i in 0..(rows * 9) as usize {
                slots.push(cell(i, 8 + 18 * (i % 9) as i32, 18 + 18 * (i / 9) as i32));
            }
            player_inventory(&mut slots, 18 + rows * 18 + 13);
        }
        McScreen::ShulkerBox => {
            size = [176, 167];
            for i in 0..27 {
                slots.push(cell(i, 8 + 18 * (i % 9) as i32, 18 + 18 * (i / 9) as i32));
            }
            player_inventory(&mut slots, 84);
        }
        McScreen::Furnace(_) => {
            slots.extend([cell(0, 56, 17), cell(1, 56, 53), output(116, 35)]);
            player_inventory(&mut slots, 84);
        }
        McScreen::Brewing => {
            for (i, (x, y)) in [(56, 51), (79, 58), (102, 51)].into_iter().enumerate() {
                slots.push(SlotSpec {
                    empty: Some("container/slot/potion"),
                    ..cell(i, x, y)
                });
            }
            slots.push(cell(3, 79, 17));
            slots.push(SlotSpec {
                empty: Some("container/slot/brewing_fuel"),
                ..cell(4, 17, 17)
            });
            player_inventory(&mut slots, 84);
        }
        McScreen::Enchanting => {
            slots.push(cell(0, 15, 47));
            slots.push(SlotSpec {
                empty: Some("container/slot/lapis_lazuli"),
                ..cell(1, 35, 47)
            });
            player_inventory(&mut slots, 84);
        }
        McScreen::Anvil => {
            slots.extend([cell(0, 27, 47), cell(1, 76, 47), output(134, 47)]);
            player_inventory(&mut slots, 84);
        }
        McScreen::Grindstone => {
            slots.extend([cell(0, 49, 19), cell(1, 49, 40), output(129, 34)]);
            player_inventory(&mut slots, 84);
        }
        McScreen::Stonecutter => {
            slots.extend([cell(0, 20, 33), output(143, 33)]);
            player_inventory(&mut slots, 84);
        }
        McScreen::Smithing => {
            slots.extend([
                cell(0, 8, 48),
                cell(1, 26, 48),
                cell(2, 44, 48),
                output(98, 48),
            ]);
            player_inventory(&mut slots, 84);
        }
        McScreen::Creative(kind) => return crate::minecraft_creative::layout(kind, 0),
        McScreen::Lectern | McScreen::BookEdit | McScreen::BookSign => size = [192, 192],
    }
    Layout {
        screen,
        size,
        slots,
    }
}

impl Layout {
    pub(crate) fn origin(&self, gui: [i32; 2]) -> [i32; 2] {
        [(gui[0] - self.size[0]) / 2, (gui[1] - self.size[1]) / 2]
    }

    pub(crate) fn slot_at(&self, [mx, my]: [f64; 2]) -> Option<McSlot> {
        self.slots
            .iter()
            .find(|s| {
                mx >= f64::from(s.x - 1)
                    && mx < f64::from(s.x + 17)
                    && my >= f64::from(s.y - 1)
                    && my < f64::from(s.y + 17)
            })
            .map(|s| s.slot)
    }

    pub(crate) fn outside(&self, [mx, my]: [f64; 2]) -> bool {
        mx < 0.0 || my < 0.0 || mx >= f64::from(self.size[0]) || my >= f64::from(self.size[1])
    }

    pub(crate) fn spec(&self, slot: McSlot) -> Option<&SlotSpec> {
        self.slots.iter().find(|s| s.slot == slot)
    }

    pub(crate) fn sheet(&self) -> &'static str {
        match self.screen {
            McScreen::Inventory => "inventory",
            McScreen::CraftingTable => "crafting_table",
            McScreen::Chest { .. } => "generic_54",
            McScreen::ShulkerBox => "shulker_box",
            McScreen::Furnace(McOven::Furnace) => "furnace",
            McScreen::Furnace(McOven::Smoker) => "smoker",
            McScreen::Furnace(McOven::BlastFurnace) => "blast_furnace",
            McScreen::Brewing => "brewing_stand",
            McScreen::Enchanting => "enchanting_table",
            McScreen::Anvil => "anvil",
            McScreen::Grindstone => "grindstone",
            McScreen::Stonecutter => "stonecutter",
            McScreen::Smithing => "smithing",
            McScreen::Creative(_) => "creative_inventory/tab_items",
            McScreen::Lectern | McScreen::BookEdit | McScreen::BookSign => "book",
        }
    }
}

pub(crate) fn enchant_row_at([mx, my]: [f64; 2]) -> Option<usize> {
    (0..3).find(|&i| {
        let (dx, dy) = (mx - 60.0, my - f64::from(14 + 19 * i as i32));
        (0.0..108.0).contains(&dx) && (0.0..19.0).contains(&dy)
    })
}

pub(crate) fn recipe_at([mx, my]: [f64; 2], start: usize, count: usize) -> Option<usize> {
    (start..count.min(start + RECIPE_COLUMNS * RECIPE_ROWS)).find(|&index| {
        let k = index - start;
        let dx = mx - f64::from(52 + (k % RECIPE_COLUMNS) as i32 * 16);
        let dy = my - f64::from(14 + (k / RECIPE_COLUMNS) as i32 * 18);
        (0.0..16.0).contains(&dx) && (0.0..18.0).contains(&dy)
    })
}

pub(crate) fn over_scroller([mx, my]: [f64; 2]) -> bool {
    (119.0..131.0).contains(&mx) && (9.0..63.0).contains(&my)
}

pub(crate) fn hidden_rows(count: usize) -> usize {
    count.div_ceil(RECIPE_COLUMNS).saturating_sub(RECIPE_ROWS)
}

pub(crate) fn start_index(offset: f32, count: usize) -> usize {
    ((offset * hidden_rows(count) as f32 + 0.5) as usize) * RECIPE_COLUMNS
}

pub(crate) fn drag_offset(my: f64) -> f32 {
    ((my as f32 - 14.0 - 7.5) / 39.0).clamp(0.0, 1.0)
}

pub(crate) fn lit_height(lit: i32, duration: i32) -> i32 {
    let duration = if duration == 0 { 200 } else { duration };
    ((lit as f32 / duration as f32).clamp(0.0, 1.0) * 13.0).ceil() as i32 + 1
}

pub(crate) fn burn_width(progress: i32, total: i32) -> i32 {
    let fraction = if total != 0 && progress != 0 {
        (progress as f32 / total as f32).clamp(0.0, 1.0)
    } else {
        0.0
    };
    (fraction * 24.0).ceil() as i32
}

pub(crate) fn fuel_width(fuel: i32, total: i32) -> i32 {
    if total <= 0 {
        return 0;
    }
    (-(-18 * fuel).div_euclid(total)).clamp(0, 18)
}

pub(crate) fn brew_height(ticks: i32, total: i32) -> i32 {
    if ticks <= 0 || total <= 0 {
        return 0;
    }
    (28.0 * (1.0 - ticks as f32 / total as f32)) as i32
}

pub(crate) fn bubble_height(ticks: i32) -> i32 {
    BUBBLES[(ticks / 2).rem_euclid(7) as usize]
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Cycler {
    icons: Vec<&'static str>,
    tick: u32,
    index: usize,
}

impl Cycler {
    pub(crate) fn tick(&mut self, icons: &[&'static str]) {
        if self.icons != icons {
            self.icons = icons.to_vec();
            self.index = 0;
        }
        if !self.icons.is_empty() {
            self.tick += 1;
            if self.tick.is_multiple_of(CYCLE_TICKS) {
                self.index = (self.index + 1) % self.icons.len();
            }
        }
    }

    pub(crate) fn shown(&self, partial: f32) -> Vec<(&'static str, f32)> {
        let Some(&current) = self.icons.get(self.index) else {
            return Vec::new();
        };
        let switching = self.icons.len() > 1 && self.tick >= CYCLE_TICKS;
        let alpha = if switching {
            ((self.tick % CYCLE_TICKS) as f32 + partial).min(4.0) / 4.0
        } else {
            1.0
        };
        let mut out = Vec::new();
        if alpha < 1.0 {
            let previous = self.icons[(self.index + self.icons.len() - 1) % self.icons.len()];
            out.push((previous, 1.0 - alpha));
        }
        out.push((current, alpha));
        out
    }
}

pub(crate) fn smithing_icons(template: Option<&McStack>) -> [&'static [&'static str]; 2] {
    match template.map(|stack| stack.id.as_str()) {
        Some("minecraft:netherite_upgrade_smithing_template") => {
            [&NETHERITE_BASES, &NETHERITE_ADDITIONS]
        }
        Some(id) if id.ends_with("_armor_trim_smithing_template") => [&TRIM_BASES, &TRIM_ADDITIONS],
        _ => [&[], &[]],
    }
}

pub(crate) fn smithing_templates() -> &'static [&'static str] {
    &SMITHING_TEMPLATES
}

pub(crate) struct Station<'a> {
    pub hovered_choice: Option<usize>,
    pub focused_choice: Option<usize>,
    pub scroll: f32,
    pub start: usize,
    pub name: &'a str,
    pub name_from: usize,
    pub cursor: bool,
    pub cyclers: &'a [Cycler; 3],
    pub partial: f32,
    pub level: u32,
}

pub(crate) struct Screen<'a> {
    pub ui: &'a MinecraftUi,
    pub weapons: Option<&'a PreparedWeapons>,
    pub layout: &'a Layout,
    pub menu: &'a dyn Menu,
    pub origin: [i32; 2],
    pub mouse: [i32; 2],
    pub hovered: Option<McSlot>,
    pub drag: Option<&'a Drag>,
    pub title: &'a str,
    pub station: Station<'a>,
}

impl Screen<'_> {
    fn at(&self, x: i32, y: i32) -> (i32, i32) {
        (self.origin[0] + x, self.origin[1] + y)
    }

    fn cell(&self, index: usize) -> Option<&McStack> {
        self.ui.container.get(index).and_then(Option::as_ref)
    }
}

pub(crate) fn draw_background(p: &mut Painter<'_>, s: &Screen<'_>) {
    let [left, top] = s.origin;
    let [w, h] = s.layout.size;
    let sheet = s.layout.sheet();
    match s.layout.screen {
        McScreen::Inventory => {
            let [bx0, by0, bx1, by1] = PUPPET_BOX;
            p.blit(sheet, left, top, 0, 0, w, by0);
            p.blit(sheet, left, top + by1, 0, by1, w, h - by1);
            p.blit(sheet, left, top + by0, 0, by0, bx0, by1 - by0);
            p.blit(sheet, left + bx1, top + by0, bx1, by0, w - bx1, by1 - by0);
            draw_grenade_frames(p, s.origin, GRENADE_SLOTS);
        }
        McScreen::Chest { rows } => {
            let rows = i32::from(rows);
            p.blit(sheet, left, top, 0, 0, w, rows * 18 + 17);
            p.blit(sheet, left, top + rows * 18 + 17, 0, 126, w, 96);
        }
        _ => p.blit(sheet, left, top, 0, 0, w, h),
    }
    match s.layout.screen {
        McScreen::Furnace(oven) => furnace(p, s, oven),
        McScreen::Brewing => brewing(p, s),
        McScreen::Enchanting => enchanting(p, s),
        McScreen::Anvil => anvil(p, s),
        McScreen::Grindstone => {
            if (s.cell(0).is_some() || s.cell(1).is_some()) && s.ui.output.is_none() {
                let (x, y) = s.at(92, 31);
                p.sprite("container/grindstone/error", x, y, 28, 21);
            }
        }
        McScreen::Stonecutter => stonecutter(p, s),
        McScreen::Smithing => smithing(p, s),
        _ => {}
    }
}

fn furnace(p: &mut Painter<'_>, s: &Screen<'_>, oven: McOven) {
    let kind = match oven {
        McOven::Furnace => "furnace",
        McOven::Smoker => "smoker",
        McOven::BlastFurnace => "blast_furnace",
    };
    let [lit, duration, cook, total] = s.ui.data;
    if lit > 0 {
        let k = lit_height(lit, duration);
        let (x, y) = s.at(56, 36 + 14 - k);
        p.sprite_part(
            &format!("container/{kind}/lit_progress"),
            14,
            14,
            0,
            14 - k,
            x,
            y,
            14,
            k,
        );
    }
    let (x, y) = s.at(79, 34);
    p.sprite_part(
        &format!("container/{kind}/burn_progress"),
        24,
        16,
        0,
        0,
        x,
        y,
        burn_width(cook, total),
        16,
    );
}

fn brewing(p: &mut Painter<'_>, s: &Screen<'_>) {
    let [ticks, fuel, total_ticks, total_fuel] = s.ui.data;
    let width = fuel_width(fuel, total_fuel);
    if width > 0 {
        let (x, y) = s.at(60, 44);
        p.sprite_part(
            "container/brewing_stand/fuel_length",
            18,
            4,
            0,
            0,
            x,
            y,
            width,
            4,
        );
    }
    if ticks > 0 && total_ticks > 0 {
        let height = brew_height(ticks, total_ticks);
        if height > 0 {
            let (x, y) = s.at(97, 16);
            p.sprite_part(
                "container/brewing_stand/brew_progress",
                9,
                28,
                0,
                0,
                x,
                y,
                9,
                height,
            );
        }
        let bubbles = bubble_height(ticks);
        if bubbles > 0 {
            let (x, y) = s.at(63, 14 + 29 - bubbles);
            p.sprite_part(
                "container/brewing_stand/bubbles",
                12,
                29,
                0,
                29 - bubbles,
                x,
                y,
                12,
                bubbles,
            );
        }
    }
}

fn enchanting(p: &mut Painter<'_>, s: &Screen<'_>) {
    let lapis = s.cell(1).map_or(0, |stack| u32::from(stack.count));
    for i in 0..3 {
        let (x, y) = s.at(60, 14 + 19 * i as i32);
        let Some(choice) = s.ui.choices.get(i).filter(|choice| choice.cost > 0) else {
            p.sprite(
                "container/enchanting_table/enchantment_slot_disabled",
                x,
                y,
                108,
                19,
            );
            continue;
        };
        let cost = choice.cost.to_string();
        let width = 86 - p.width(&cost);
        let level_y = y + 1;
        let disabled = lapis < i as u32 + 1 || s.station.level < choice.cost;
        let color = if disabled {
            p.sprite(
                "container/enchanting_table/enchantment_slot_disabled",
                x,
                y,
                108,
                19,
            );
            p.sprite(
                &format!("container/enchanting_table/level_{}_disabled", i + 1),
                x + 1,
                level_y,
                16,
                16,
            );
            for (line, text) in p
                .gui
                .alt_font
                .split(&choice.runes, width)
                .iter()
                .enumerate()
            {
                p.styled_text(
                    true,
                    text,
                    x + 20,
                    y + 2 + 9 * line as i32,
                    0xff00_0000 | ((0x685E4A & 0xFEFEFE) >> 1),
                    false,
                    false,
                );
            }
            0xFF40_7F10
        } else {
            let lit = s.station.hovered_choice == Some(i) || s.station.focused_choice == Some(i);
            let (sprite, rune_color) = if lit {
                (
                    "container/enchanting_table/enchantment_slot_highlighted",
                    0xFFFF_FF80,
                )
            } else {
                ("container/enchanting_table/enchantment_slot", 0xFF68_5E4A)
            };
            p.sprite(sprite, x, y, 108, 19);
            p.sprite(
                &format!("container/enchanting_table/level_{}", i + 1),
                x + 1,
                level_y,
                16,
                16,
            );
            for (line, text) in p
                .gui
                .alt_font
                .split(&choice.runes, width)
                .iter()
                .enumerate()
            {
                p.styled_text(
                    true,
                    text,
                    x + 20,
                    y + 2 + 9 * line as i32,
                    rune_color,
                    false,
                    false,
                );
            }
            0xFF80_FF20
        };
        p.text(&cost, x + 20 + 86 - p.width(&cost), y + 2 + 7, color, true);
    }
}

fn anvil(p: &mut Painter<'_>, s: &Screen<'_>) {
    if (s.cell(0).is_some() || s.cell(1).is_some()) && s.ui.output.is_none() {
        let (x, y) = s.at(99, 45);
        p.sprite("container/anvil/error", x, y, 28, 21);
    }
    let (x, y) = s.at(59, 20);
    let editable = s.cell(0).is_some();
    p.sprite(
        if editable {
            "container/anvil/text_field"
        } else {
            "container/anvil/text_field_disabled"
        },
        x,
        y,
        110,
        16,
    );
    let (tx, ty) = s.at(62, 24);
    let shown: String = s.station.name.chars().skip(s.station.name_from).collect();
    let shown = p.gui.font.head_by_width(&shown, 103);
    if !shown.is_empty() {
        p.text(&shown, tx, ty, WHITE, true);
    }
    if editable && s.station.cursor {
        let end = s.station.name_from + shown.chars().count() >= s.station.name.chars().count();
        let x = if shown.is_empty() {
            tx
        } else {
            tx + p.width(&shown) + 1
        };
        if end && s.station.name.chars().count() < 50 {
            p.text("_", x, ty, WHITE, true);
        } else {
            p.fill(x - 1, ty - 1, x, ty + 1 + 9, WHITE);
        }
    }
}

fn stonecutter(p: &mut Painter<'_>, s: &Screen<'_>) {
    let count = s.ui.choices.len();
    let active = count > RECIPE_COLUMNS * RECIPE_ROWS;
    let (x, y) = s.at(119, 15 + (41.0 * s.station.scroll) as i32);
    p.sprite(
        if active {
            "container/stonecutter/scroller"
        } else {
            "container/stonecutter/scroller_disabled"
        },
        x,
        y,
        12,
        15,
    );
    let start = s.station.start;
    let rel = [
        f64::from(s.mouse[0] - s.origin[0]),
        f64::from(s.mouse[1] - s.origin[1]),
    ];
    for index in start..count.min(start + RECIPE_COLUMNS * RECIPE_ROWS) {
        let k = index - start;
        let (cx, cy) = s.at(
            52 + (k % RECIPE_COLUMNS) as i32 * 16,
            14 + (k / RECIPE_COLUMNS) as i32 * 18 + 2,
        );
        let hovered = s.station.focused_choice == Some(index) || {
            let (dx, dy) = (
                rel[0] - f64::from(cx - s.origin[0]),
                rel[1] - f64::from(cy - s.origin[1]),
            );
            s.station.focused_choice.is_none()
                && (0.0..16.0).contains(&dx)
                && (0.0..18.0).contains(&dy)
        };
        let sprite = if s.ui.chosen == Some(index) {
            "container/stonecutter/recipe_selected"
        } else if hovered {
            "container/stonecutter/recipe_highlighted"
        } else {
            "container/stonecutter/recipe"
        };
        p.sprite(sprite, cx, cy - 1, 16, 18);
    }
    for index in start..count.min(start + RECIPE_COLUMNS * RECIPE_ROWS) {
        let k = index - start;
        let (cx, cy) = s.at(
            52 + (k % RECIPE_COLUMNS) as i32 * 16,
            14 + (k / RECIPE_COLUMNS) as i32 * 18 + 2,
        );
        if let Some(icon) = s.ui.choices[index].icon.as_ref() {
            p.item(s.ui, s.weapons, icon, cx, cy);
        }
    }
}

fn smithing(p: &mut Painter<'_>, s: &Screen<'_>) {
    if s.cell(0).is_some() && s.cell(1).is_some() && s.cell(2).is_some() && s.ui.output.is_none() {
        let (x, y) = s.at(65, 46);
        p.sprite("container/smithing/error", x, y, 28, 21);
    }
    for (index, cycler) in s.station.cyclers.iter().enumerate() {
        if s.cell(index).is_some() {
            continue;
        }
        let Some(spec) = s.layout.spec(McSlot::Container(index)) else {
            continue;
        };
        let (x, y) = s.at(spec.x, spec.y);
        for (icon, alpha) in cycler.shown(s.station.partial) {
            let alpha = (alpha * 255.0).floor() as u32;
            p.sprite_tinted(icon, x, y, 16, 16, (alpha << 24) | 0x00ff_ffff);
        }
    }
}

pub(crate) fn draw_labels(p: &mut Painter<'_>, s: &Screen<'_>) {
    let [w, h] = s.layout.size;
    let (x, y) = match s.layout.screen {
        McScreen::Inventory => (97, 6),
        McScreen::CraftingTable => (29, 6),
        McScreen::Furnace(_) | McScreen::Brewing => ((w - p.width(s.title)) / 2, 6),
        McScreen::Anvil => (60, 6),
        McScreen::Stonecutter => (8, 5),
        McScreen::Smithing => (44, 15),
        _ => (8, 6),
    };
    let (tx, ty) = s.at(x, y);
    p.text(s.title, tx, ty, LABEL, false);
    if s.layout.screen != McScreen::Inventory {
        let (ix, iy) = s.at(8, h - 94);
        let label = p.gui.text("container.inventory");
        p.text(&label, ix, iy, LABEL, false);
    }
}

pub(crate) fn draw_highlight(p: &mut Painter<'_>, s: &Screen<'_>, front: bool) {
    let Some(spec) = s.hovered.and_then(|slot| s.layout.spec(slot)) else {
        return;
    };
    let (x, y) = s.at(spec.x - 4, spec.y - 4);
    p.sprite(
        if front {
            "container/slot_highlight_front"
        } else {
            "container/slot_highlight_back"
        },
        x,
        y,
        24,
        24,
    );
}

enum SlotDraw {
    Hidden,
    Empty(&'static str),
    Item {
        stack: McStack,
        fill: bool,
        count: Option<(String, u32)>,
    },
    Nothing,
}

fn slot_draw(s: &Screen<'_>, spec: &SlotSpec) -> SlotDraw {
    let mut stack = crate::minecraft_inventory::stack_at(s.ui, spec.slot).cloned();
    let mut fill = false;
    let mut count = None;
    if let (Some(drag), Some(carried)) = (
        s.drag.filter(|drag| drag.slots.contains(&spec.slot)),
        s.ui.cursor.as_ref(),
    ) {
        if drag.slots.len() == 1 {
            return SlotDraw::Hidden;
        }
        if let Some((shown, capped)) = preview(s.menu, drag, spec.slot) {
            fill = true;
            stack = Some(McStack {
                count: shown.min(255) as u8,
                ..carried.clone()
            });
            if capped {
                count = Some((shown.to_string(), YELLOW));
            }
        }
    }
    match (stack, spec.empty) {
        (Some(stack), _) => SlotDraw::Item { stack, fill, count },
        (None, Some(icon)) => SlotDraw::Empty(icon),
        (None, None) => SlotDraw::Nothing,
    }
}

pub(crate) fn draw_slots(p: &mut Painter<'_>, s: &Screen<'_>) {
    let draws: Vec<(i32, i32, SlotDraw)> = s
        .layout
        .slots
        .iter()
        .map(|spec| {
            let (x, y) = s.at(spec.x, spec.y);
            (x, y, slot_draw(s, spec))
        })
        .collect();
    for (x, y, draw) in &draws {
        match draw {
            SlotDraw::Empty(icon) => p.sprite(icon, *x, *y, 16, 16),
            SlotDraw::Item { fill: true, .. } => p.fill(*x, *y, x + 16, y + 16, 0x80FF_FFFF),
            _ => {}
        }
    }
    for (x, y, draw) in &draws {
        if let SlotDraw::Item { stack, .. } = draw {
            p.item(s.ui, s.weapons, stack, *x, *y);
        }
    }
    for (x, y, draw) in draws {
        if let SlotDraw::Item { stack, count, .. } = draw {
            p.decorations(s.ui, &stack, [x, y], count);
        }
    }
}

pub(crate) fn draw_carried(p: &mut Painter<'_>, s: &Screen<'_>, at: [i32; 2]) {
    let Some(carried) = s.ui.cursor.as_ref() else {
        return;
    };
    let mut stack = carried.clone();
    if let Some(drag) = s.drag.filter(|drag| drag.slots.len() > 1) {
        let left = remainder(s.menu, drag);
        if left == 0 {
            return;
        }
        stack.count = left.min(255) as u8;
    }
    let (x, y) = (at[0] - 8, at[1] - 8);
    p.item(s.ui, s.weapons, &stack, x, y);
    p.decorations(s.ui, &stack, [x, y], None);
}

pub(crate) fn station_tooltip(p: &Painter<'_>, s: &Screen<'_>) -> Option<(Vec<TextLine>, bool)> {
    let rel = [
        f64::from(s.mouse[0] - s.origin[0]),
        f64::from(s.mouse[1] - s.origin[1]),
    ];
    match s.layout.screen {
        McScreen::Enchanting => {
            let lapis = s.cell(1).map_or(0, |stack| u32::from(stack.count));
            let i = (0..3).find(|&i| {
                let (dx, dy) = (rel[0] - 59.0, rel[1] - f64::from(14 + 19 * i as i32 - 1));
                (0.0..110.0).contains(&dx) && (0.0..19.0).contains(&dy)
            })?;
            let choice =
                s.ui.choices
                    .get(i)
                    .filter(|choice| choice.cost > 0 && !choice.label.is_empty())?;
            let (clue, names) = p
                .gui
                .format_spans("container.enchant.clue", &[&choice.label]);
            let mut lines = vec![
                TextLine {
                    accents: names
                        .into_iter()
                        .map(|name| (name, crate::minecraft_gui::GRAY))
                        .collect(),
                    ..TextLine::new(clue, WHITE)
                },
                TextLine::new("", WHITE),
            ];
            let n = (i + 1).to_string();
            if s.station.level < choice.cost {
                lines.push(TextLine::new(
                    p.gui.format(
                        "container.enchant.level.requirement",
                        &[&choice.cost.to_string()],
                    ),
                    crate::minecraft_gui::RED,
                ));
            } else {
                let lapis_line = if i == 0 {
                    p.gui.text("container.enchant.lapis.one")
                } else {
                    p.gui.format("container.enchant.lapis.many", &[&n])
                };
                lines.push(TextLine::new(
                    lapis_line,
                    if lapis > i as u32 {
                        crate::minecraft_gui::GRAY
                    } else {
                        crate::minecraft_gui::RED
                    },
                ));
                let level_line = if i == 0 {
                    p.gui.text("container.enchant.level.one")
                } else {
                    p.gui.format("container.enchant.level.many", &[&n])
                };
                lines.push(TextLine::new(level_line, crate::minecraft_gui::GRAY));
            }
            Some((lines, false))
        }
        McScreen::Smithing => {
            let error = s.cell(0).is_some()
                && s.cell(1).is_some()
                && s.cell(2).is_some()
                && s.ui.output.is_none();
            let over_error = (64.0..94.0).contains(&rel[0]) && (45.0..68.0).contains(&rel[1]);
            let mut text =
                (error && over_error).then(|| p.gui.text("container.upgrade.error_tooltip"));
            let template = s.cell(0).map(|stack| stack.id.as_str());
            let hovered_empty = s
                .hovered
                .is_some_and(|slot| crate::minecraft_inventory::stack_at(s.ui, slot).is_none());
            if let Some(key) = onboarding_key(s.hovered, hovered_empty, template) {
                text = Some(p.gui.text(&key));
            }
            let text = text?;
            Some((
                p.gui
                    .font
                    .split(&text, 115)
                    .into_iter()
                    .map(|line| TextLine::new(line, WHITE))
                    .collect(),
                false,
            ))
        }
        _ => None,
    }
}

fn onboarding_key(
    hovered: Option<McSlot>,
    hovered_empty: bool,
    template: Option<&str>,
) -> Option<String> {
    let index = match hovered? {
        McSlot::Container(index) | McSlot::Inventory(index) => index,
        _ => return None,
    };
    let Some(template) = template else {
        return (index == 0).then(|| "container.upgrade.missing_template_tooltip".to_owned());
    };
    let kind = if template == "minecraft:netherite_upgrade_smithing_template" {
        "netherite_upgrade"
    } else if template.ends_with("_armor_trim_smithing_template") {
        "armor_trim"
    } else {
        return None;
    };
    let slot = match index {
        1 => "base_slot_description",
        2 => "additions_slot_description",
        _ => return None,
    };
    hovered_empty.then(|| format!("item.minecraft.smithing_template.{kind}.{slot}"))
}

pub(crate) fn hovered_recipe(s: &Screen<'_>) -> Option<usize> {
    if s.layout.screen != McScreen::Stonecutter {
        return None;
    }
    let rel = [
        f64::from(s.mouse[0] - s.origin[0]),
        f64::from(s.mouse[1] - s.origin[1]),
    ];
    let start = s.station.start;
    (start..s.ui.choices.len().min(start + RECIPE_COLUMNS * RECIPE_ROWS)).find(|&index| {
        let k = index - start;
        let dx = rel[0] - f64::from(52 + (k % RECIPE_COLUMNS) as i32 * 16);
        let dy = rel[1] - f64::from(14 + (k / RECIPE_COLUMNS) as i32 * 18 + 2);
        (0.0..16.0).contains(&dx) && (0.0..18.0).contains(&dy)
    })
}

pub(crate) fn anvil_cost(p: &mut Painter<'_>, s: &Screen<'_>) {
    let Some(cost) = s.ui.cost.filter(|&cost| cost > 0) else {
        return;
    };
    let (text, color) = if cost >= 40 {
        (p.gui.text("container.repair.expensive"), 0xFFFF_6060)
    } else if s.ui.output.is_none() {
        return;
    } else {
        let color = if s.station.level >= cost {
            0xFF80_FF20
        } else {
            0xFFFF_6060
        };
        (
            p.gui.format("container.repair.cost", &[&cost.to_string()]),
            color,
        )
    };
    let x = 176 - 8 - p.width(&text) - 2;
    let (fx, fy) = s.at(x - 2, 67);
    let (fx1, fy1) = s.at(176 - 8, 79);
    p.fill(fx, fy, fx1, fy1, 0x4F00_0000);
    let (tx, ty) = s.at(x, 69);
    p.text(&text, tx, ty, color, true);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(layout: &Layout, slot: McSlot) -> (i32, i32) {
        let spec = layout.spec(slot).unwrap();
        (spec.x, spec.y)
    }

    #[test]
    fn the_inventory_screen_follows_the_vanilla_menu_order() {
        let layout = layout(McScreen::Inventory);
        assert_eq!(layout.slots.len(), 48);
        assert_eq!(layout.slots[0].slot, McSlot::Result);
        assert_eq!(at(&layout, McSlot::Result), (154, 28));
        assert_eq!(at(&layout, McSlot::Crafting(3)), (116, 36));
        assert_eq!(layout.slots[5].slot, McSlot::Inventory(39));
        assert_eq!(at(&layout, McSlot::Inventory(36)), (8, 62));
        assert_eq!(at(&layout, McSlot::Inventory(9)), (8, 84));
        assert_eq!(at(&layout, McSlot::Inventory(35)), (152, 120));
        assert_eq!(at(&layout, McSlot::Inventory(0)), (8, 142));
        assert_eq!(layout.slots[45].slot, McSlot::Inventory(40));
        assert_eq!(at(&layout, McSlot::Inventory(40)), (77, 62));
        assert_eq!(
            layout.spec(McSlot::Inventory(40)).unwrap().empty,
            Some("container/slot/shield")
        );
        assert_eq!(
            layout.spec(McSlot::Inventory(39)).unwrap().empty,
            Some("container/slot/helmet")
        );
    }

    #[test]
    fn chests_grow_by_eighteen_a_row() {
        let small = layout(McScreen::Chest { rows: 3 });
        let large = layout(McScreen::Chest { rows: 6 });
        assert_eq!(small.size, [176, 168]);
        assert_eq!(large.size, [176, 222]);
        assert_eq!(at(&small, McSlot::Inventory(9)), (8, 85));
        assert_eq!(at(&small, McSlot::Inventory(0)), (8, 143));
        assert_eq!(at(&large, McSlot::Inventory(9)), (8, 139));
        assert_eq!(at(&large, McSlot::Container(53)), (152, 108));
        let shulker = layout(McScreen::ShulkerBox);
        assert_eq!((shulker.size, shulker.sheet()), ([176, 167], "shulker_box"));
        assert_eq!(at(&shulker, McSlot::Container(26)), (152, 54));
        assert_eq!(at(&shulker, McSlot::Inventory(9)), (8, 84));
        assert_eq!(at(&shulker, McSlot::Inventory(0)), (8, 142));
    }

    #[test]
    fn station_slots_sit_where_vanilla_puts_them() {
        let furnace = layout(McScreen::Furnace(McOven::Smoker));
        assert_eq!(at(&furnace, McSlot::Container(1)), (56, 53));
        assert_eq!(at(&furnace, McSlot::Output), (116, 35));
        assert_eq!(furnace.sheet(), "smoker");
        let brewing = layout(McScreen::Brewing);
        assert_eq!(at(&brewing, McSlot::Container(1)), (79, 58));
        assert_eq!(at(&brewing, McSlot::Container(4)), (17, 17));
        assert_eq!(at(&layout(McScreen::Anvil), McSlot::Output), (134, 47));
        assert_eq!(
            at(&layout(McScreen::Grindstone), McSlot::Container(1)),
            (49, 40)
        );
        assert_eq!(
            at(&layout(McScreen::Stonecutter), McSlot::Output),
            (143, 33)
        );
        assert_eq!(
            at(&layout(McScreen::Smithing), McSlot::Container(2)),
            (44, 48)
        );
        assert_eq!(
            at(&layout(McScreen::Enchanting), McSlot::Container(1)),
            (35, 47)
        );
        assert_eq!(
            at(&layout(McScreen::CraftingTable), McSlot::Workbench(8)),
            (66, 53)
        );
    }

    #[test]
    fn slots_are_hovered_one_pixel_beyond_their_sixteen() {
        let layout = layout(McScreen::Inventory);
        assert_eq!(layout.slot_at([7.0, 83.0]), Some(McSlot::Inventory(9)));
        assert_eq!(layout.slot_at([24.9, 100.9]), Some(McSlot::Inventory(9)));
        assert_eq!(layout.slot_at([25.0, 90.0]), Some(McSlot::Inventory(10)));
        assert_eq!(layout.slot_at([6.9, 90.0]), None);
        assert!(layout.outside([-0.5, 10.0]));
        assert!(layout.outside([176.0, 10.0]));
        assert!(!layout.outside([175.9, 165.9]));
        assert_eq!(layout.origin([427, 240]), [125, 37]);
    }

    #[test]
    fn enchanting_rows_and_stonecutter_cells_take_clicks() {
        assert_eq!(enchant_row_at([60.0, 14.0]), Some(0));
        assert_eq!(enchant_row_at([167.9, 51.9]), Some(1));
        assert_eq!(enchant_row_at([59.9, 20.0]), None);
        assert_eq!(enchant_row_at([100.0, 71.0]), None);
        assert_eq!(recipe_at([52.0, 14.0], 0, 20), Some(0));
        assert_eq!(recipe_at([101.0, 50.0], 4, 20), Some(4 + 11));
        assert_eq!(recipe_at([101.0, 50.0], 0, 10), None);
        assert!(over_scroller([119.0, 9.0]) && !over_scroller([131.0, 9.0]));
        assert_eq!(hidden_rows(13), 1);
        assert_eq!(start_index(1.0, 20), 8);
        assert_eq!(start_index(0.4, 20), 4);
        assert_eq!(drag_offset(21.5), 0.0);
        assert_eq!(drag_offset(60.5), 1.0);
    }

    #[test]
    fn progress_bars_use_vanilla_rounding() {
        assert_eq!(lit_height(0, 0), 1);
        assert_eq!(lit_height(100, 200), 8);
        assert_eq!(lit_height(1600, 1600), 14);
        assert_eq!(burn_width(100, 200), 12);
        assert_eq!(burn_width(1, 200), 1);
        assert_eq!(burn_width(0, 200), 0);
        assert_eq!(fuel_width(20, 20), 18);
        assert_eq!(fuel_width(1, 20), 1);
        assert_eq!(fuel_width(10, 20), 9);
        assert_eq!(fuel_width(0, 20), 0);
        assert_eq!(brew_height(400, 400), 0);
        assert_eq!(brew_height(200, 400), 14);
        assert_eq!(brew_height(1, 400), 27);
        assert_eq!(bubble_height(0), 29);
        assert_eq!(bubble_height(13), 0);
        assert_eq!(bubble_height(14), 29);
    }

    #[test]
    fn slot_icons_cycle_every_thirty_ticks_with_a_four_tick_fade() {
        let mut cycler = Cycler::default();
        let icons = ["a", "b"];
        for _ in 0..29 {
            cycler.tick(&icons);
        }
        assert_eq!(cycler.shown(0.0), vec![("a", 1.0)]);
        cycler.tick(&icons);
        assert_eq!(cycler.shown(0.0), vec![("a", 1.0), ("b", 0.0)]);
        cycler.tick(&icons);
        cycler.tick(&icons);
        assert_eq!(cycler.shown(0.0), vec![("a", 0.5), ("b", 0.5)]);
        for _ in 0..3 {
            cycler.tick(&icons);
        }
        assert_eq!(cycler.shown(0.0), vec![("b", 1.0)]);
        cycler.tick(&["c"]);
        assert_eq!(cycler.shown(0.0), vec![("c", 1.0)]);
    }

    #[test]
    fn smithing_icons_follow_the_template() {
        let stack = |id: &str| McStack {
            id: id.into(),
            count: 1,
            weapon: None,
            damage: None,
            max: 64,
            named: false,
            name: None,
            attack: None,
            enchantments: Vec::new(),
            contents: Vec::new(),
            more: None,
            glint: false,
            icon: id.into(),
            lore: Vec::new(),
            clip: None,
        };
        let netherite = stack("minecraft:netherite_upgrade_smithing_template");
        assert_eq!(
            smithing_icons(Some(&netherite))[1],
            &["container/slot/ingot"]
        );
        assert_eq!(smithing_icons(Some(&netherite))[0].len(), 11);
        let trim = stack("minecraft:coast_armor_trim_smithing_template");
        assert_eq!(smithing_icons(Some(&trim))[0].len(), 4);
        assert!(smithing_icons(Some(&stack("minecraft:dirt")))[0].is_empty());
        assert!(smithing_icons(None)[1].is_empty());
    }

    #[test]
    fn smithing_onboarding_follows_the_hovered_slot_index_like_vanilla() {
        let missing = Some("container.upgrade.missing_template_tooltip".to_owned());
        assert_eq!(
            onboarding_key(Some(McSlot::Container(0)), true, None),
            missing
        );
        assert_eq!(
            onboarding_key(Some(McSlot::Inventory(0)), false, None),
            missing
        );
        assert_eq!(onboarding_key(Some(McSlot::Inventory(1)), true, None), None);
        let trim = Some("minecraft:coast_armor_trim_smithing_template");
        assert_eq!(
            onboarding_key(Some(McSlot::Inventory(2)), true, trim),
            Some("item.minecraft.smithing_template.armor_trim.additions_slot_description".into())
        );
        assert_eq!(
            onboarding_key(Some(McSlot::Container(1)), true, trim),
            Some("item.minecraft.smithing_template.armor_trim.base_slot_description".into())
        );
        assert_eq!(
            onboarding_key(Some(McSlot::Inventory(2)), false, trim),
            None
        );
        assert_eq!(onboarding_key(Some(McSlot::Inventory(9)), true, trim), None);
        assert_eq!(onboarding_key(Some(McSlot::Output), true, trim), None);
        assert_eq!(onboarding_key(None, true, None), None);
    }
}
