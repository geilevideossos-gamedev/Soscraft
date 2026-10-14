use serde_json::Value;

use crate::fork_items::{self, Bucket, Kind};

pub const JSON: &str = include_str!("../data/fork_recipes.json");

pub fn recipes() -> Vec<(String, Value)> {
    let Ok(Value::Object(map)) = serde_json::from_str::<Value>(JSON) else {
        return Vec::new();
    };
    let mut recipes: Vec<(String, Value)> = map.into_iter().collect();
    recipes
        .sort_by_key(|(id, _)| fork_items::get(id).map_or(usize::MAX, |row| row.creative_order()));
    recipes
}

pub fn group(row: &fork_items::ForkItem) -> &'static str {
    match (row.kind, row.bucket) {
        (Kind::Part, _) => "iw4l_parts",
        (Kind::Attachment, _) => "iw4l_attachments",
        (Kind::Ammo, _) => "iw4l_ammo",
        (Kind::Lethal, _) => "iw4l_lethal",
        (Kind::Tactical, _) => "iw4l_tactical",
        (Kind::Gun, Bucket::Pistol) => "iw4l_pistols",
        (Kind::Gun, Bucket::MachinePistol) => "iw4l_machine_pistols",
        (Kind::Gun, Bucket::Smg) => "iw4l_smgs",
        (Kind::Gun, Bucket::AssaultRifle) => "iw4l_assault_rifles",
        (Kind::Gun, Bucket::Shotgun) => "iw4l_shotguns",
        (Kind::Gun, Bucket::Sniper) => "iw4l_snipers",
        (Kind::Gun, Bucket::Lmg) => "iw4l_lmgs",
        (Kind::Gun, Bucket::Launcher) => "iw4l_launchers",
        (Kind::Gun, _) => "iw4l_specials",
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};
    use std::io::Read;
    use std::path::{Path, PathBuf};

    use minecraftoss_player::crafting::CookingKind;
    use minecraftoss_player::inventory::{Inventory, ItemStack};

    use super::*;
    use crate::crafting::{self, Grid};

    type Items = HashSet<String>;

    #[derive(Debug)]
    enum Form {
        Shaped(Vec<Vec<Option<Items>>>),
        Shapeless(Vec<Items>),
    }

    struct Jar {
        items: HashSet<String>,
        tags: HashMap<String, Vec<String>>,
        recipes: Vec<(String, Value)>,
    }

    fn jar_path() -> Option<PathBuf> {
        let root = std::env::var_os("MINECRAFTOSS_ROOT")?;
        let jar = Path::new(&root).join("client.jar");
        jar.is_file().then_some(jar)
    }

    fn read_jar(path: &Path) -> Jar {
        let mut archive = zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
        let mut jar = Jar {
            items: HashSet::new(),
            tags: HashMap::new(),
            recipes: Vec::new(),
        };
        for index in 0..archive.len() {
            let mut entry = archive.by_index(index).unwrap();
            let name = entry.name().to_owned();
            let Some(stem) = name.strip_suffix(".json") else {
                continue;
            };
            if let Some(item) = stem.strip_prefix("assets/minecraft/items/") {
                jar.items.insert(format!("minecraft:{item}"));
                continue;
            }
            let tag = stem.strip_prefix("data/minecraft/tags/item/");
            let recipe = stem.strip_prefix("data/minecraft/recipe/");
            if tag.is_none() && recipe.is_none() {
                continue;
            }
            let mut text = String::new();
            entry.read_to_string(&mut text).unwrap();
            let value: Value = serde_json::from_str(&text).unwrap();
            if let Some(tag) = tag {
                let values = value["values"].as_array().unwrap();
                let entries = values
                    .iter()
                    .filter_map(|v| v.as_str().or_else(|| v["id"].as_str()))
                    .map(str::to_owned)
                    .collect();
                jar.tags.insert(format!("minecraft:{tag}"), entries);
            } else if let Some(recipe) = recipe {
                jar.recipes.push((format!("minecraft:{recipe}"), value));
            }
        }
        jar
    }

    fn expand(jar: &Jar, value: &Value, seen: &mut Vec<String>, out: &mut Items) {
        match value {
            Value::Array(options) => {
                for option in options {
                    expand(jar, option, seen, out);
                }
            }
            Value::String(id) => match id.strip_prefix('#') {
                Some(tag) if !seen.iter().any(|known| known == tag) => {
                    seen.push(tag.to_owned());
                    for entry in jar.tags.get(tag).into_iter().flatten() {
                        expand(jar, &Value::String(entry.clone()), seen, out);
                    }
                    seen.pop();
                }
                Some(_) => {}
                None => {
                    out.insert(id.clone());
                }
            },
            _ => {}
        }
    }

    fn items(jar: &Jar, value: &Value) -> Items {
        let mut out = Items::new();
        expand(jar, value, &mut Vec::new(), &mut out);
        out
    }

    fn trim(rows: Vec<Vec<Option<Items>>>) -> Vec<Vec<Option<Items>>> {
        let used = |cell: &Option<Items>| cell.is_some();
        let top = rows.iter().position(|row| row.iter().any(used)).unwrap();
        let bottom = rows.iter().rposition(|row| row.iter().any(used)).unwrap();
        let width = rows[0].len();
        let left = (0..width)
            .find(|&x| rows.iter().any(|row| used(&row[x])))
            .unwrap();
        let right = (0..width)
            .rev()
            .find(|&x| rows.iter().any(|row| used(&row[x])))
            .unwrap();
        rows[top..=bottom]
            .iter()
            .map(|row| row[left..=right].to_vec())
            .collect()
    }

    fn forms(jar: &Jar, recipe: &Value) -> Vec<Form> {
        let set = |key: &str| items(jar, &recipe[key]);
        match recipe["type"].as_str().unwrap_or("none") {
            "minecraft:crafting_shaped" => {
                let key = recipe["key"].as_object().unwrap();
                let rows = recipe["pattern"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|row| {
                        row.as_str()
                            .unwrap()
                            .chars()
                            .map(|symbol| {
                                (symbol != ' ').then(|| items(jar, &key[&symbol.to_string()]))
                            })
                            .collect()
                    })
                    .collect();
                vec![Form::Shaped(trim(rows))]
            }
            "minecraft:crafting_shapeless" => vec![Form::Shapeless(
                recipe["ingredients"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|ingredient| items(jar, ingredient))
                    .collect(),
            )],
            "minecraft:crafting_transmute" => {
                vec![Form::Shapeless(vec![set("input"), set("material")])]
            }
            "minecraft:crafting_dye" => (1..=8)
                .map(|dyes| {
                    let mut cells = vec![set("target")];
                    cells.extend((0..dyes).map(|_| set("dye")));
                    Form::Shapeless(cells)
                })
                .collect(),
            "minecraft:crafting_imbue" => {
                let ring = || Some(set("material"));
                vec![Form::Shaped(vec![
                    vec![ring(), ring(), ring()],
                    vec![ring(), Some(set("source")), ring()],
                    vec![ring(), ring(), ring()],
                ])]
            }
            _ => Vec::new(),
        }
    }

    fn meets(a: &Items, b: &Items) -> bool {
        a.iter().any(|id| b.contains(id))
    }

    fn perfect(a: &[&Items], b: &[&Items]) -> bool {
        fn augment(
            a: &[&Items],
            b: &[&Items],
            i: usize,
            seen: &mut [bool],
            owner: &mut [Option<usize>],
        ) -> bool {
            for j in 0..b.len() {
                if !seen[j] && meets(a[i], b[j]) {
                    seen[j] = true;
                    if owner[j].is_none_or(|k| augment(a, b, k, seen, owner)) {
                        owner[j] = Some(i);
                        return true;
                    }
                }
            }
            false
        }
        if a.len() != b.len() {
            return false;
        }
        let mut owner = vec![None; b.len()];
        (0..a.len()).all(|i| augment(a, b, i, &mut vec![false; b.len()], &mut owner))
    }

    fn cells(rows: &[Vec<Option<Items>>]) -> Vec<&Items> {
        rows.iter().flatten().flatten().collect()
    }

    fn collide(a: &Form, b: &Form) -> bool {
        match (a, b) {
            (Form::Shaped(p), Form::Shaped(q)) => {
                if p.len() != q.len() || p[0].len() != q[0].len() {
                    return false;
                }
                let width = p[0].len();
                [false, true].into_iter().any(|mirror| {
                    p.iter().zip(q).all(|(row, other)| {
                        (0..width).all(|x| {
                            let cell = &row[if mirror { width - 1 - x } else { x }];
                            match (cell, &other[x]) {
                                (None, None) => true,
                                (Some(a), Some(b)) => meets(a, b),
                                _ => false,
                            }
                        })
                    })
                })
            }
            (Form::Shapeless(p), Form::Shapeless(q)) => {
                perfect(&p.iter().collect::<Vec<_>>(), &q.iter().collect::<Vec<_>>())
            }
            (Form::Shaped(rows), Form::Shapeless(list))
            | (Form::Shapeless(list), Form::Shaped(rows)) => {
                perfect(&cells(rows), &list.iter().collect::<Vec<_>>())
            }
        }
    }

    fn real_book() -> Option<minecraftoss_player::crafting::RecipeBook> {
        let jar = jar_path()?;
        let root = jar.parent().unwrap().to_owned();
        let catalog = crafting::find_item_catalog(Some(&root), &jar)?;
        let (book, warning) = crafting::load_recipes(&jar, Some(&catalog)).unwrap();
        assert!(warning.is_none());
        Some(book)
    }

    fn ingredients(recipe: &Value) -> Vec<&str> {
        let values: Vec<&Value> = match recipe["key"].as_object() {
            Some(key) => key.values().collect(),
            None => recipe["ingredients"].as_array().unwrap().iter().collect(),
        };
        values.into_iter().map(|v| v.as_str().unwrap()).collect()
    }

    #[test]
    fn every_fork_item_has_exactly_one_recipe_of_its_kind_and_group() {
        let recipes = recipes();
        let written = JSON
            .lines()
            .filter(|line| line.starts_with("  \"iw4l:"))
            .count();
        assert_eq!(written, fork_items::rows().len());
        assert_eq!(recipes.len(), fork_items::rows().len());
        for (row, (id, recipe)) in fork_items::rows().iter().zip(&recipes) {
            assert_eq!(id, row.id);
            assert_eq!(recipe["result"]["id"], row.id);
            assert_eq!(recipe["category"], "equipment", "{id}");
            assert_eq!(recipe["group"], group(row), "{id}");
            assert!(recipe["result"].get("components").is_none(), "{id}");
            let kind = match row.kind {
                Kind::Gun | Kind::Part | Kind::Attachment => "minecraft:crafting_shaped",
                Kind::Ammo | Kind::Lethal | Kind::Tactical => "minecraft:crafting_shapeless",
            };
            assert_eq!(recipe["type"], kind, "{id}");
            for ingredient in ingredients(recipe) {
                assert!(
                    ingredient.starts_with("minecraft:")
                        || ingredient.starts_with("#minecraft:")
                        || fork_items::get(ingredient).is_some_and(|item| item.kind == Kind::Part),
                    "{id}: {ingredient}"
                );
            }
        }
    }

    #[test]
    fn yields_follow_the_design() {
        let more_than_one = [
            ("iw4l:gun_barrel", 2),
            ("iw4l:pistol_ammo", 24),
            ("iw4l:rifle_ammo", 16),
            ("iw4l:lmg_ammo", 32),
            ("iw4l:shotgun_shells", 8),
            ("iw4l:sniper_rounds", 4),
            ("iw4l:explosive_round", 2),
            ("iw4l:rocket", 2),
            ("iw4l:frag_grenade", 2),
            ("iw4l:semtex", 2),
            ("iw4l:throwing_knife", 2),
            ("iw4l:claymore", 2),
            ("iw4l:c4", 2),
            ("iw4l:flash_grenade", 2),
            ("iw4l:concussion_grenade", 2),
            ("iw4l:smoke_grenade", 2),
        ];
        for (id, recipe) in recipes() {
            let count = recipe["result"]["count"].as_u64().unwrap_or(1);
            let expected = more_than_one
                .iter()
                .find(|(known, _)| *known == id)
                .map_or(1, |(_, count)| *count);
            assert_eq!(count, expected, "{id}");
        }
    }

    #[test]
    fn the_m4a1_recipe_is_the_design_example() {
        let recipes = recipes();
        let (_, m4) = recipes.iter().find(|(id, _)| id == "iw4l:m4").unwrap();
        assert_eq!(
            *m4,
            serde_json::json!({
                "type": "minecraft:crafting_shaped", "category": "equipment",
                "group": "iw4l_assault_rifles",
                "key": {"B": "iw4l:gun_barrel", "K": "minecraft:iron_block",
                    "F": "minecraft:repeater", "M": "iw4l:firing_mechanism",
                    "X": "minecraft:iron_ingot"},
                "pattern": ["BKF", " MX"],
                "result": {"id": "iw4l:m4"}
            })
        );
    }

    #[test]
    fn every_ingredient_exists_in_the_jar_or_the_table() {
        let Some(path) = jar_path() else {
            return;
        };
        let jar = read_jar(&path);
        assert!(jar.items.contains("minecraft:iron_chain"));
        for (id, recipe) in recipes() {
            for ingredient in ingredients(&recipe) {
                let known = match ingredient.strip_prefix('#') {
                    Some(tag) => jar.tags.contains_key(tag),
                    None => jar.items.contains(ingredient) || fork_items::is_fork(ingredient),
                };
                assert!(known, "{id}: {ingredient}");
            }
        }
    }

    #[test]
    fn no_fork_grid_matches_a_vanilla_grid_or_another_fork_grid() {
        let Some(path) = jar_path() else {
            return;
        };
        let jar = read_jar(&path);
        let count = |kind: &str| {
            jar.recipes
                .iter()
                .filter(|(_, recipe)| recipe["type"] == kind)
                .count()
        };
        assert_eq!(count("minecraft:crafting_shaped"), 827);
        assert_eq!(count("minecraft:crafting_shapeless"), 375);
        assert_eq!(count("minecraft:crafting_transmute"), 33);
        assert_eq!(count("minecraft:crafting_dye"), 6);
        let vanilla: Vec<(String, Vec<Form>)> = jar
            .recipes
            .iter()
            .map(|(id, recipe)| (id.clone(), forms(&jar, recipe)))
            .collect();
        let fork: Vec<(String, Vec<Form>)> = recipes()
            .iter()
            .map(|(id, recipe)| (id.clone(), forms(&jar, recipe)))
            .collect();
        let shears = vanilla
            .iter()
            .find(|(id, _)| id == "minecraft:shears")
            .unwrap();
        let mirrored = Form::Shaped(vec![
            vec![None, Some(Items::from(["minecraft:iron_ingot".to_owned()]))],
            vec![Some(Items::from(["minecraft:iron_ingot".to_owned()])), None],
        ]);
        assert!(collide(&mirrored, &shears.1[0]));
        let mut clashes = Vec::new();
        for (index, (id, mine)) in fork.iter().enumerate() {
            assert_eq!(mine.len(), 1, "{id}");
            let others = vanilla.iter().chain(&fork[index + 1..]);
            for (other, theirs) in others {
                if theirs.iter().any(|form| collide(&mine[0], form)) {
                    clashes.push(format!("{id} and {other}"));
                }
            }
        }
        assert!(clashes.is_empty(), "{clashes:?}");
    }

    #[test]
    fn fork_display_indices_come_after_vanilla() {
        let Some(book) = real_book() else {
            return;
        };
        let vanilla = book
            .crafting_recipes()
            .filter(|recipe| !fork_items::is_fork(recipe.id))
            .count()
            + [
                CookingKind::Furnace,
                CookingKind::BlastFurnace,
                CookingKind::Smoker,
            ]
            .into_iter()
            .map(|kind| book.cooking_count(kind))
            .sum::<usize>();
        let indices: Vec<u32> = fork_items::rows()
            .iter()
            .map(|row| book.display_indices(row.id).unwrap()[0])
            .collect();
        assert_eq!(indices[0] as usize, vanilla);
        assert!(indices.windows(2).all(|pair| pair[1] == pair[0] + 1));
        assert!(
            book.crafting_recipes()
                .filter(|recipe| !fork_items::is_fork(recipe.id))
                .all(|recipe| book
                    .display_indices(recipe.id)
                    .is_none_or(|known| known.iter().all(|index| *index < indices[0])))
        );
        let fork_recipes = book
            .crafting_recipes()
            .filter(|recipe| fork_items::is_fork(recipe.id))
            .count();
        assert_eq!(fork_recipes, fork_items::rows().len());
        let mut inventory = Inventory::default().with_recipes(book);
        for id in [
            "minecraft:iron_ingot",
            "minecraft:gunpowder",
            "iw4l:gun_barrel",
        ] {
            inventory.notice_item_changed(&ItemStack::new(id, 1));
        }
        assert!(!inventory.unlocked_recipes.is_empty());
        assert!(
            inventory
                .unlocked_recipes
                .iter()
                .all(|id| !fork_items::is_fork(id))
        );
    }

    #[test]
    fn crafting_an_m4a1_gives_an_empty_gun_that_stacks_to_one() {
        let Some(book) = real_book() else {
            return;
        };
        let mut inventory = Inventory::default().with_recipes(book);
        let put = |inventory: &mut Inventory, cell: usize, id: &str, count: u8| {
            inventory.workbench[cell] = Some(inventory.recipes.stack(id, count));
        };
        put(&mut inventory, 0, "minecraft:iron_ingot", 3);
        put(&mut inventory, 1, "minecraft:iron_ingot", 3);
        put(&mut inventory, 2, "minecraft:iron_ingot", 3);
        let barrels = crafting::output(&inventory, Grid::Table).unwrap();
        assert_eq!(
            (barrels.id.as_str(), barrels.count, barrels.max),
            ("iw4l:gun_barrel", 2, 64)
        );
        assert!(crafting::take(&mut inventory, Grid::Table));
        assert_eq!(inventory.cursor.as_ref().map(|s| s.count), Some(2));
        inventory.cursor = None;
        inventory.workbench = std::array::from_fn(|_| None);
        put(&mut inventory, 5, "iw4l:gun_barrel", 1);
        put(&mut inventory, 4, "minecraft:iron_block", 1);
        put(&mut inventory, 3, "minecraft:repeater", 1);
        put(&mut inventory, 7, "iw4l:firing_mechanism", 1);
        put(&mut inventory, 6, "minecraft:iron_ingot", 1);
        let gun = crafting::output(&inventory, Grid::Table).unwrap();
        assert_eq!((gun.id.as_str(), gun.count, gun.max), ("iw4l:m4", 1, 1));
        assert_eq!(gun.components, None);
        assert!(crafting::take(&mut inventory, Grid::Table));
        assert_eq!(inventory.cursor, Some(gun));
        assert!(inventory.workbench.iter().all(Option::is_none));
        inventory.cursor = None;
        inventory.crafting[0] = Some(inventory.recipes.stack("minecraft:gunpowder", 1));
        inventory.crafting[1] = Some(inventory.recipes.stack("minecraft:iron_nugget", 1));
        inventory.crafting[3] = Some(inventory.recipes.stack("minecraft:copper_ingot", 1));
        let ammo = crafting::output(&inventory, Grid::Inventory).unwrap();
        assert_eq!(
            (ammo.id.as_str(), ammo.count, ammo.max),
            ("iw4l:rifle_ammo", 16, 64)
        );
        inventory.crafting = std::array::from_fn(|_| None);
        inventory.crafting[0] = Some(inventory.recipes.stack("minecraft:tnt", 1));
        inventory.crafting[1] = Some(inventory.recipes.stack("minecraft:fire_charge", 1));
        inventory.crafting[2] = Some(inventory.recipes.stack("minecraft:iron_ingot", 1));
        let rockets = crafting::output(&inventory, Grid::Inventory).unwrap();
        assert_eq!(
            (rockets.id.as_str(), rockets.count, rockets.max),
            ("iw4l:rocket", 2, 16)
        );
    }
}
