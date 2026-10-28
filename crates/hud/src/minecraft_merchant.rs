use frame::{McGui, McMerchant, McOffer, McStack, MinecraftUi};

use crate::minecraft_gui::{LABEL, Painter, TextLine, WHITE};
use crate::minecraft_screens::Screen;

pub(crate) const SIZE: [i32; 2] = [276, 166];
pub(crate) const OFFER_BUTTONS: usize = 7;
const BUTTON_X: i32 = 5;
const BUTTON_Y: i32 = 18;
const BUTTON_WIDTH: i32 = 88;
const BUTTON_HEIGHT: i32 = 20;
const SCROLLER_X: i32 = 94;
const SCROLLER_WIDTH: i32 = 6;
const SCROLLER_HEIGHT: i32 = 27;
const SCROLL_BAR_HEIGHT: i32 = 139;
const SCROLLER_TRAVEL: i32 = 113;
const SELL_ITEM_1_X: i32 = 5;
const SELL_ITEM_2_X: i32 = 35;
const BUY_ITEM_X: i32 = 68;
const PROGRESS_BAR: [i32; 2] = [136, 16];
const PROGRESS_WIDTH: i32 = 102;
const LEVEL_XP: [i32; 5] = [0, 10, 70, 150, 250];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Part {
    CostA,
    CostB,
    Result,
}

pub(crate) fn can_scroll(count: usize) -> bool {
    count > OFFER_BUTTONS
}

pub(crate) fn max_scroll(count: usize) -> usize {
    count.saturating_sub(OFFER_BUTTONS)
}

pub(crate) fn shown(count: usize, scroll: usize) -> std::ops::Range<usize> {
    if can_scroll(count) {
        let start = scroll.min(max_scroll(count));
        start..start + OFFER_BUTTONS
    } else {
        0..count
    }
}

pub(crate) fn row_at([mx, my]: [f64; 2]) -> Option<usize> {
    (0..OFFER_BUTTONS).find(|&row| {
        let x = f64::from(BUTTON_X);
        let y = f64::from(BUTTON_Y + BUTTON_HEIGHT * row as i32);
        mx >= x && my >= y && mx < x + f64::from(BUTTON_WIDTH) && my < y + f64::from(BUTTON_HEIGHT)
    })
}

pub(crate) fn offer_at(rel: [f64; 2], scroll: usize, count: usize) -> Option<usize> {
    row_at(rel)
        .map(|row| row + scroll)
        .filter(|&index| index < count)
}

pub(crate) fn over_scroller([mx, my]: [f64; 2]) -> bool {
    mx > f64::from(SCROLLER_X)
        && mx < f64::from(SCROLLER_X + SCROLLER_WIDTH)
        && my > f64::from(BUTTON_Y)
        && my <= f64::from(BUTTON_Y + SCROLL_BAR_HEIGHT + 1)
}

pub(crate) fn drag_scroll(my: f64, count: usize) -> usize {
    let max = max_scroll(count) as i32;
    let along = (my as f32 - BUTTON_Y as f32 - 13.5) / (SCROLL_BAR_HEIGHT - SCROLLER_HEIGHT) as f32;
    ((along * max as f32 + 0.5) as i32).clamp(0, max) as usize
}

pub(crate) fn wheel_scroll(scroll: usize, wheel: i32, count: usize) -> usize {
    (scroll as i32 + wheel).clamp(0, max_scroll(count) as i32) as usize
}

pub(crate) fn scroller_y(scroll: usize, count: usize) -> Option<i32> {
    let steps = count as i32 + 1 - OFFER_BUTTONS as i32;
    if steps <= 1 {
        return None;
    }
    let rest = SCROLL_BAR_HEIGHT - (SCROLLER_HEIGHT + (steps - 1) * SCROLL_BAR_HEIGHT / steps);
    let step = 1 + rest / steps + SCROLL_BAR_HEIGHT / steps;
    if scroll as i32 == steps - 1 {
        Some(SCROLLER_TRAVEL)
    } else {
        Some((scroll as i32 * step).min(SCROLLER_TRAVEL))
    }
}

pub(crate) fn discounted(offer: &McOffer) -> bool {
    offer
        .cost_a
        .as_ref()
        .is_some_and(|cost| cost.count != offer.base_count)
}

pub(crate) fn arrow(offer: &McOffer) -> &'static str {
    if offer.out_of_stock {
        "container/villager/trade_arrow_out_of_stock"
    } else {
        "container/villager/trade_arrow"
    }
}

pub(crate) fn xp_widths(level: i32, xp: i32, future: i32) -> Option<[i32; 2]> {
    if !(1..5).contains(&level) {
        return None;
    }
    let min = LEVEL_XP[level as usize - 1];
    let max = LEVEL_XP[level as usize];
    if xp < min {
        return None;
    }
    let ratio = PROGRESS_WIDTH as f32 / (max - min) as f32;
    let current = ((ratio * (xp - min) as f32).floor() as i32).min(PROGRESS_WIDTH);
    let coming = if future > 0 {
        ((future as f32 * ratio).floor() as i32).min(PROGRESS_WIDTH - current)
    } else {
        0
    };
    Some([current, coming])
}

pub(crate) fn title(gui: &McGui, name: &str, merchant: Option<&McMerchant>) -> String {
    match merchant {
        Some(merchant) if (1..=5).contains(&merchant.level) && merchant.show_progress => {
            let level = gui.text(&format!("merchant.level.{}", merchant.level));
            gui.format("merchant.title", &[name, &level])
        }
        _ => name.to_owned(),
    }
}

pub(crate) fn part_at(x: i32) -> Option<Part> {
    if x < BUTTON_X + 20 {
        Some(Part::CostA)
    } else if x < BUTTON_X + 50 && x > BUTTON_X + 30 {
        Some(Part::CostB)
    } else if x > BUTTON_X + 65 {
        Some(Part::Result)
    } else {
        None
    }
}

pub(crate) fn hovered_stack(ui: &MinecraftUi, rel: [i32; 2], scroll: usize) -> Option<&McStack> {
    let merchant = ui.merchant.as_ref()?;
    let index = offer_at(
        [f64::from(rel[0]), f64::from(rel[1])],
        scroll,
        merchant.offers.len(),
    )?;
    let offer = &merchant.offers[index];
    match part_at(rel[0])? {
        Part::CostA => offer.cost_a.as_ref(),
        Part::CostB => offer.cost_b.as_ref(),
        Part::Result => offer.result.as_ref(),
    }
}

pub(crate) fn restock_hint(ui: &MinecraftUi, [mx, my]: [i32; 2]) -> bool {
    let Some(merchant) = ui.merchant.as_ref().filter(|merchant| merchant.can_restock) else {
        return false;
    };
    let shown = merchant.offers.get(ui.chosen.unwrap_or(0));
    shown.is_some_and(|offer| offer.out_of_stock)
        && (185..209).contains(&mx)
        && (34..57).contains(&my)
}

pub(crate) fn tooltip(p: &Painter<'_>, s: &Screen<'_>) -> Option<(Vec<TextLine>, bool)> {
    let rel = [s.mouse[0] - s.origin[0], s.mouse[1] - s.origin[1]];
    restock_hint(s.ui, rel).then(|| {
        (
            vec![TextLine::new(p.gui.text("merchant.deprecated"), WHITE)],
            false,
        )
    })
}

pub(crate) fn draw_background(p: &mut Painter<'_>, s: &Screen<'_>) {
    let Some(merchant) = s.ui.merchant.as_ref().filter(|m| !m.offers.is_empty()) else {
        return;
    };
    let [left, top] = s.origin;
    let count = merchant.offers.len();
    let selected = s.ui.chosen.unwrap_or(0);
    if merchant
        .offers
        .get(selected)
        .is_some_and(|offer| offer.out_of_stock)
    {
        p.sprite(
            "container/villager/out_of_stock",
            left + 83 + 99,
            top + 35,
            28,
            21,
        );
    }
    let scroll = s.station.trade_scroll;
    let rel = [f64::from(s.mouse[0] - left), f64::from(s.mouse[1] - top)];
    let hovered = row_at(rel);
    for row in 0..OFFER_BUTTONS.min(count) {
        let lit = hovered == Some(row) || s.station.trade_row == Some(row);
        p.sprite(
            if lit {
                "widget/button_highlighted"
            } else {
                "widget/button"
            },
            left + BUTTON_X,
            top + BUTTON_Y + BUTTON_HEIGHT * row as i32,
            BUTTON_WIDTH,
            BUTTON_HEIGHT,
        );
    }
    match scroller_y(scroll, count) {
        Some(y) => p.sprite(
            "container/villager/scroller",
            left + SCROLLER_X,
            top + BUTTON_Y + y,
            SCROLLER_WIDTH,
            SCROLLER_HEIGHT,
        ),
        None => p.sprite(
            "container/villager/scroller_disabled",
            left + SCROLLER_X,
            top + BUTTON_Y,
            SCROLLER_WIDTH,
            SCROLLER_HEIGHT,
        ),
    }
    for (k, index) in shown(count, scroll).enumerate() {
        let offer = &merchant.offers[index];
        let y = top + 16 + 1 + 20 * k as i32 + 2;
        let x = left + BUTTON_X;
        if let Some(cost) = offer.cost_a.as_ref() {
            draw_cost_a(p, s, cost, offer, [x + SELL_ITEM_1_X, y]);
        }
        if let Some(cost) = offer.cost_b.as_ref() {
            p.item(s.ui, s.weapons, cost, x + SELL_ITEM_2_X, y);
            p.decorations(s.ui, cost, [x + SELL_ITEM_2_X, y], None);
        }
        p.sprite(arrow(offer), x + SELL_ITEM_2_X + 20, y + 3, 10, 9);
        if let Some(result) = offer.result.as_ref() {
            p.item(s.ui, s.weapons, result, x + BUY_ITEM_X, y);
            p.decorations(s.ui, result, [x + BUY_ITEM_X, y], None);
        }
    }
    if merchant.show_progress && merchant.level < 5 {
        let [bx, by] = [left + PROGRESS_BAR[0], top + PROGRESS_BAR[1]];
        p.sprite(
            "container/villager/experience_bar_background",
            bx,
            by,
            PROGRESS_WIDTH,
            5,
        );
        if let Some([current, coming]) = xp_widths(merchant.level, merchant.xp, merchant.future_xp)
        {
            p.sprite_part(
                "container/villager/experience_bar_current",
                PROGRESS_WIDTH,
                5,
                0,
                0,
                bx,
                by,
                current,
                5,
            );
            if coming > 0 {
                p.sprite_part(
                    "container/villager/experience_bar_result",
                    PROGRESS_WIDTH,
                    5,
                    current,
                    0,
                    bx + current,
                    by,
                    coming,
                    5,
                );
            }
        }
    }
}

fn draw_cost_a(
    p: &mut Painter<'_>,
    s: &Screen<'_>,
    cost: &McStack,
    offer: &McOffer,
    [x, y]: [i32; 2],
) {
    p.item(s.ui, s.weapons, cost, x, y);
    if !discounted(offer) {
        p.decorations(s.ui, cost, [x, y], None);
        return;
    }
    let base = offer.base_count;
    let base_stack = McStack {
        count: base,
        ..cost.clone()
    };
    p.decorations(s.ui, &base_stack, [x, y], Some((base.to_string(), WHITE)));
    p.decorations(
        s.ui,
        cost,
        [x + 14, y],
        Some((cost.count.to_string(), WHITE)),
    );
    p.sprite(
        "container/villager/discount_strikethrough",
        x + 7,
        y + 12,
        9,
        2,
    );
}

pub(crate) fn draw_labels(p: &mut Painter<'_>, s: &Screen<'_>) {
    let [left, top] = s.origin;
    let title = title(p.gui, s.title, s.ui.merchant.as_ref());
    let width = p.width(&title);
    p.text(
        &title,
        left + 49 + SIZE[0] / 2 - width / 2,
        top + 6,
        LABEL,
        false,
    );
    let inventory = p.gui.text("container.inventory");
    p.text(&inventory, left + 107, top + SIZE[1] - 94, LABEL, false);
    let trades = p.gui.text("merchant.trades");
    let width = p.width(&trades);
    p.text(&trades, left + 5 - width / 2 + 48, top + 6, LABEL, false);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stack(id: &str, count: u8) -> McStack {
        McStack {
            id: id.into(),
            count,
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
            ..Default::default()
        }
    }

    fn offer(base: u8, cost: u8, out_of_stock: bool) -> McOffer {
        McOffer {
            cost_a: Some(stack("minecraft:emerald", cost)),
            base_count: base,
            cost_b: Some(stack("minecraft:book", 1)),
            result: Some(stack("minecraft:bookshelf", 1)),
            out_of_stock,
        }
    }

    #[test]
    fn offer_rows_take_clicks_on_the_vanilla_buttons() {
        assert_eq!(row_at([5.0, 18.0]), Some(0));
        assert_eq!(row_at([92.9, 37.9]), Some(0));
        assert_eq!(row_at([93.0, 30.0]), None);
        assert_eq!(row_at([4.9, 30.0]), None);
        assert_eq!(row_at([50.0, 38.0]), Some(1));
        assert_eq!(row_at([50.0, 157.9]), Some(6));
        assert_eq!(row_at([50.0, 158.0]), None);
        assert_eq!(offer_at([50.0, 38.0], 0, 3), Some(1));
        assert_eq!(offer_at([50.0, 78.0], 0, 3), None);
        assert_eq!(offer_at([50.0, 38.0], 2, 9), Some(3));
    }

    #[test]
    fn nine_offers_scroll_two_steps_by_wheel_and_drag() {
        assert!(!can_scroll(7));
        assert!(can_scroll(9));
        assert_eq!(max_scroll(9), 2);
        assert_eq!(shown(9, 0), 0..7);
        assert_eq!(shown(9, 2), 2..9);
        assert_eq!(shown(9, 5), 2..9);
        assert_eq!(shown(5, 3), 0..5);
        assert_eq!(wheel_scroll(0, 1, 9), 1);
        assert_eq!(wheel_scroll(1, 5, 9), 2);
        assert_eq!(wheel_scroll(1, -3, 9), 0);
        assert_eq!(drag_scroll(31.5, 9), 0);
        assert_eq!(drag_scroll(87.5, 9), 1);
        assert_eq!(drag_scroll(143.5, 9), 2);
        assert_eq!(drag_scroll(500.0, 9), 2);
        assert_eq!(drag_scroll(-50.0, 9), 0);
        assert_eq!(scroller_y(0, 7), None);
        assert_eq!(scroller_y(0, 9), Some(0));
        assert_eq!(scroller_y(1, 9), Some(53));
        assert_eq!(scroller_y(2, 9), Some(113));
        assert!(over_scroller([95.0, 19.0]));
        assert!(over_scroller([99.9, 158.0]));
        assert!(!over_scroller([94.0, 30.0]));
        assert!(!over_scroller([96.0, 18.0]));
    }

    #[test]
    fn a_discount_strikes_the_old_price_and_a_sold_out_offer_crosses_its_arrow() {
        assert!(!discounted(&offer(24, 24, false)));
        assert!(discounted(&offer(24, 19, false)));
        assert!(discounted(&offer(1, 3, false)));
        assert_eq!(arrow(&offer(1, 1, false)), "container/villager/trade_arrow");
        assert_eq!(
            arrow(&offer(1, 1, true)),
            "container/villager/trade_arrow_out_of_stock"
        );
    }

    #[test]
    fn the_experience_bar_fills_toward_the_next_level() {
        assert_eq!(xp_widths(1, 0, 0), Some([0, 0]));
        assert_eq!(xp_widths(1, 5, 2), Some([51, 20]));
        assert_eq!(xp_widths(1, 9, 30), Some([91, 11]));
        assert_eq!(xp_widths(2, 40, 0), Some([51, 0]));
        assert_eq!(xp_widths(2, 5, 0), None);
        assert_eq!(xp_widths(5, 300, 0), None);
        assert_eq!(xp_widths(0, 0, 0), None);
    }

    #[test]
    fn the_title_names_the_level_and_tooltips_follow_the_columns() {
        let mut gui = McGui::default();
        gui.texts.insert("merchant.title".into(), "%s - %s".into());
        gui.texts
            .insert("merchant.level.2".into(), "Apprentice".into());
        let merchant = McMerchant {
            offers: (0..9).map(|i| offer(1 + i, 1 + i, false)).collect(),
            level: 2,
            show_progress: true,
            ..McMerchant::default()
        };
        assert_eq!(
            title(&gui, "Farmer", Some(&merchant)),
            "Farmer - Apprentice"
        );
        let hidden = McMerchant {
            show_progress: false,
            ..merchant.clone()
        };
        assert_eq!(title(&gui, "Farmer", Some(&hidden)), "Farmer");
        assert_eq!(part_at(24), Some(Part::CostA));
        assert_eq!(part_at(35), None);
        assert_eq!(part_at(36), Some(Part::CostB));
        assert_eq!(part_at(55), None);
        assert_eq!(part_at(71), Some(Part::Result));
        let mut ui = MinecraftUi {
            merchant: Some(merchant),
            ..MinecraftUi::default()
        };
        assert_eq!(hovered_stack(&ui, [10, 40], 2).map(|s| s.count), Some(4));
        assert_eq!(
            hovered_stack(&ui, [80, 40], 2).map(|s| s.id.as_str()),
            Some("minecraft:bookshelf")
        );
        assert!(!restock_hint(&ui, [190, 40]));
        if let Some(merchant) = ui.merchant.as_mut() {
            merchant.offers[3].out_of_stock = true;
            merchant.can_restock = true;
        }
        ui.chosen = Some(3);
        assert!(restock_hint(&ui, [190, 40]));
        assert!(!restock_hint(&ui, [209, 40]));
    }
}
