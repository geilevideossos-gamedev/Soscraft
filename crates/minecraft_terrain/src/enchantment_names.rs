use std::io::Read;
use std::path::Path;

use anyhow::{Result, anyhow, bail};

const ENCHANTMENT_NAMES_CLASS: &str =
    "net/minecraft/client/gui/screens/inventory/EnchantmentNames.class";
const CONSTANT_UTF8: u8 = 1;
const CONSTANT_STRING: u8 = 8;
const LDC: u8 = 0x12;
const LDC_W: u8 = 0x13;

pub fn enchantment_names(jar: &Path) -> Result<Vec<String>> {
    let mut archive = zip::ZipArchive::new(std::fs::File::open(jar)?)?;
    let mut entry = archive.by_name(ENCHANTMENT_NAMES_CLASS)?;
    let mut bytes = Vec::new();
    entry.read_to_end(&mut bytes)?;
    let words = constructor_strings(&bytes)?;
    if words.is_empty() {
        bail!("{ENCHANTMENT_NAMES_CLASS} has no words");
    }
    Ok(words)
}

struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self
            .at
            .checked_add(n)
            .filter(|&end| end <= self.bytes.len());
        let end = end.ok_or_else(|| anyhow!("class file ends early"))?;
        let out = &self.bytes[self.at..end];
        self.at = end;
        Ok(out)
    }

    fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16> {
        let b = self.take(2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    }

    fn u32(&mut self) -> Result<u32> {
        let b = self.take(4)?;
        Ok(u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }
}

enum Constant {
    Utf8(String),
    String(u16),
    Other,
}

pub fn constructor_strings(class: &[u8]) -> Result<Vec<String>> {
    let mut r = Reader {
        bytes: class,
        at: 0,
    };
    if r.u32()? != 0xCAFE_BABE {
        bail!("not a class file");
    }
    r.take(4)?;
    let count = r.u16()? as usize;
    let mut pool: Vec<Constant> = Vec::with_capacity(count);
    pool.push(Constant::Other);
    while pool.len() < count {
        let tag = r.u8()?;
        let (constant, wide) = match tag {
            CONSTANT_UTF8 => {
                let len = r.u16()? as usize;
                (
                    Constant::Utf8(String::from_utf8_lossy(r.take(len)?).into_owned()),
                    false,
                )
            }
            CONSTANT_STRING => (Constant::String(r.u16()?), false),
            3 | 4 | 9 | 10 | 11 | 12 | 17 | 18 => {
                r.take(4)?;
                (Constant::Other, false)
            }
            5 | 6 => {
                r.take(8)?;
                (Constant::Other, true)
            }
            7 | 16 | 19 | 20 => {
                r.take(2)?;
                (Constant::Other, false)
            }
            15 => {
                r.take(3)?;
                (Constant::Other, false)
            }
            other => bail!("unknown constant tag {other}"),
        };
        pool.push(constant);
        if wide {
            pool.push(Constant::Other);
        }
    }
    let utf8 = |index: u16| match pool.get(index as usize) {
        Some(Constant::Utf8(text)) => Some(text.as_str()),
        _ => None,
    };
    r.take(6)?;
    let interfaces = r.u16()? as usize;
    r.take(interfaces * 2)?;
    let skip_members = |r: &mut Reader| -> Result<()> {
        let members = r.u16()?;
        for _ in 0..members {
            r.take(6)?;
            let attributes = r.u16()?;
            for _ in 0..attributes {
                r.take(2)?;
                let len = r.u32()? as usize;
                r.take(len)?;
            }
        }
        Ok(())
    };
    skip_members(&mut r)?;
    let methods = r.u16()?;
    for _ in 0..methods {
        r.take(2)?;
        let name = r.u16()?;
        r.take(2)?;
        let attributes = r.u16()?;
        for _ in 0..attributes {
            let attribute = r.u16()?;
            let len = r.u32()? as usize;
            let body = r.take(len)?;
            if utf8(name) != Some("<init>") || utf8(attribute) != Some("Code") {
                continue;
            }
            let mut code = Reader { bytes: body, at: 0 };
            code.take(4)?;
            let length = code.u32()? as usize;
            let bytecode = code.take(length)?;
            let mut out = Vec::new();
            for index in loaded_constants(bytecode)? {
                if let Some(Constant::String(text)) = pool.get(index as usize) {
                    if let Some(text) = utf8(*text) {
                        out.push(text.to_owned());
                    }
                }
            }
            return Ok(out);
        }
    }
    bail!("class has no constructor")
}

fn loaded_constants(code: &[u8]) -> Result<Vec<u16>> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < code.len() {
        let op = code[at];
        match op {
            LDC => out.push(u16::from(
                *code.get(at + 1).ok_or_else(|| anyhow!("truncated ldc"))?,
            )),
            LDC_W => {
                let hi = *code.get(at + 1).ok_or_else(|| anyhow!("truncated ldc_w"))?;
                let lo = *code.get(at + 2).ok_or_else(|| anyhow!("truncated ldc_w"))?;
                out.push(u16::from_be_bytes([hi, lo]));
            }
            _ => {}
        }
        at += instruction_length(code, at)?;
    }
    Ok(out)
}

fn instruction_length(code: &[u8], at: usize) -> Result<usize> {
    let op = code[at];
    let read_i32 = |offset: usize| -> Result<i32> {
        let b = code
            .get(offset..offset + 4)
            .ok_or_else(|| anyhow!("truncated switch"))?;
        Ok(i32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    };
    Ok(match op {
        0x10 | 0x12 | 0x15..=0x19 | 0x36..=0x3a | 0xa9 | 0xbc => 2,
        0x11
        | 0x13
        | 0x14
        | 0x84
        | 0x99..=0xa8
        | 0xb2..=0xb8
        | 0xbb
        | 0xbd
        | 0xc0
        | 0xc1
        | 0xc6
        | 0xc7 => 3,
        0xc5 => 4,
        0xb9 | 0xba | 0xc8 | 0xc9 => 5,
        0xc4 => {
            if code.get(at + 1) == Some(&0x84) {
                6
            } else {
                4
            }
        }
        0xaa => {
            let base = (at + 4) & !3;
            let low = read_i32(base + 4)?;
            let high = read_i32(base + 8)?;
            let entries = (i64::from(high) - i64::from(low) + 1).max(0) as usize;
            base + 12 + entries * 4 - at
        }
        0xab => {
            let base = (at + 4) & !3;
            let pairs = read_i32(base + 4)?.max(0) as usize;
            base + 8 + pairs * 8 - at
        }
        _ => 1,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utf8(out: &mut Vec<u8>, text: &str) {
        out.push(CONSTANT_UTF8);
        out.extend((text.len() as u16).to_be_bytes());
        out.extend(text.as_bytes());
    }

    fn class(words: &[&str], extra_code: &[u8]) -> Vec<u8> {
        let mut pool = Vec::new();
        let mut count: u16 = 1;
        let mut strings = Vec::new();
        for word in words {
            utf8(&mut pool, word);
            pool.push(CONSTANT_STRING);
            pool.extend(count.to_be_bytes());
            strings.push(count + 1);
            count += 2;
        }
        let init = count;
        utf8(&mut pool, "<init>");
        utf8(&mut pool, "Code");
        utf8(&mut pool, "()V");
        let code_name = init + 1;
        let descriptor = init + 2;
        count += 3;
        let mut code = Vec::new();
        code.extend_from_slice(extra_code);
        for (i, string) in strings.iter().enumerate() {
            if i % 2 == 0 {
                code.push(LDC);
                code.push(*string as u8);
            } else {
                code.push(LDC_W);
                code.extend(string.to_be_bytes());
            }
            code.push(0x53);
        }
        code.push(0xb1);
        let mut out = vec![0xCA, 0xFE, 0xBA, 0xBE, 0, 0, 0, 65];
        out.extend(count.to_be_bytes());
        out.extend(pool);
        out.extend([0, 1, 0, 0, 0, 0]);
        out.extend([0, 0]);
        out.extend([0, 0]);
        out.extend([0, 1]);
        out.extend([0, 2]);
        out.extend(init.to_be_bytes());
        out.extend(descriptor.to_be_bytes());
        out.extend([0, 1]);
        out.extend(code_name.to_be_bytes());
        let body_len = 2 + 2 + 4 + code.len() + 2 + 2;
        out.extend((body_len as u32).to_be_bytes());
        out.extend([0, 4, 0, 1]);
        out.extend((code.len() as u32).to_be_bytes());
        out.extend(code);
        out.extend([0, 0, 0, 0]);
        out.extend([0, 0]);
        out
    }

    #[test]
    fn constructor_strings_come_back_in_load_order() {
        let bytes = class(&["the", "elder", "scrolls"], &[]);
        assert_eq!(
            constructor_strings(&bytes).unwrap(),
            vec!["the", "elder", "scrolls"]
        );
    }

    #[test]
    fn operands_are_not_read_as_opcodes() {
        let bytes = class(&["klaatu"], &[0x10, LDC, 0x11, LDC, LDC, 0x00]);
        assert_eq!(constructor_strings(&bytes).unwrap(), vec!["klaatu"]);
    }

    #[test]
    fn a_broken_class_is_an_error() {
        assert!(constructor_strings(&[0, 1, 2]).is_err());
        let mut bytes = class(&["berata"], &[]);
        bytes.truncate(bytes.len() - 12);
        assert!(constructor_strings(&bytes).is_err());
    }

    #[test]
    fn the_real_word_list_has_sixty_two_words() {
        let Some(root) = std::env::var_os("MINECRAFTOSS_ROOT") else {
            return;
        };
        let jar = Path::new(&root).join("client.jar");
        if !jar.is_file() {
            return;
        }
        let words = enchantment_names(&jar).unwrap();
        assert_eq!(words.len(), 62);
        assert_eq!(words[0], "the");
        assert_eq!(words[61], "baguette");
    }
}
