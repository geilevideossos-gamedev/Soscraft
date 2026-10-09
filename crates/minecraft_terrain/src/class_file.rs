use anyhow::{Result, anyhow, bail};

pub const ICONST_M1: u8 = 0x02;
pub const ICONST_5: u8 = 0x08;
pub const BIPUSH: u8 = 0x10;
pub const SIPUSH: u8 = 0x11;
pub const LDC: u8 = 0x12;
pub const LDC_W: u8 = 0x13;
pub const LDC2_W: u8 = 0x14;
pub const IINC: u8 = 0x84;
pub const GOTO: u8 = 0xa7;
pub const GOTO_W: u8 = 0xc8;
pub const TABLESWITCH: u8 = 0xaa;
pub const LOOKUPSWITCH: u8 = 0xab;
pub const GETSTATIC: u8 = 0xb2;
pub const PUTSTATIC: u8 = 0xb3;
pub const GETFIELD: u8 = 0xb4;
pub const PUTFIELD: u8 = 0xb5;
pub const INVOKEVIRTUAL: u8 = 0xb6;
pub const INVOKESPECIAL: u8 = 0xb7;
pub const INVOKESTATIC: u8 = 0xb8;
pub const INVOKEINTERFACE: u8 = 0xb9;
pub const INVOKEDYNAMIC: u8 = 0xba;
pub const NEW: u8 = 0xbb;
pub const WIDE: u8 = 0xc4;

#[derive(Clone, Debug, PartialEq)]
pub enum Constant {
    Unused,
    Utf8(String),
    Integer(i32),
    Float(f32),
    Long(i64),
    Double(f64),
    Class(u16),
    String(u16),
    Field(u16, u16),
    Method(u16, u16),
    InterfaceMethod(u16, u16),
    NameAndType(u16, u16),
    MethodHandle(u8, u16),
    MethodType(u16),
    Dynamic(u16, u16),
    InvokeDynamic(u16, u16),
    Module(u16),
    Package(u16),
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Member {
    pub class: String,
    pub name: String,
    pub descriptor: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Handle {
    pub kind: u8,
    pub member: Member,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dynamic {
    pub name: String,
    pub descriptor: String,
    pub bootstrap: Handle,
    pub arguments: Vec<u16>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Field {
    pub access: u16,
    pub name: String,
    pub descriptor: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Method {
    pub access: u16,
    pub name: String,
    pub descriptor: String,
    pub max_locals: u16,
    pub code: Option<Vec<u8>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Bootstrap {
    pub method: u16,
    pub arguments: Vec<u16>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClassFile {
    pub name: String,
    pub pool: Vec<Constant>,
    pub fields: Vec<Field>,
    pub methods: Vec<Method>,
    pub bootstraps: Vec<Bootstrap>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Instruction {
    pub at: usize,
    pub op: u8,
    pub index: u16,
    pub value: i32,
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
            .filter(|&end| end <= self.bytes.len())
            .ok_or_else(|| anyhow!("class file ends early"))?;
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

impl ClassFile {
    pub fn parse(bytes: &[u8]) -> Result<ClassFile> {
        let mut r = Reader { bytes, at: 0 };
        if r.u32()? != 0xCAFE_BABE {
            bail!("not a class file");
        }
        r.take(4)?;
        let count = r.u16()? as usize;
        let mut pool = Vec::with_capacity(count);
        pool.push(Constant::Unused);
        while pool.len() < count {
            let tag = r.u8()?;
            let constant = match tag {
                1 => {
                    let len = r.u16()? as usize;
                    Constant::Utf8(String::from_utf8_lossy(r.take(len)?).into_owned())
                }
                3 => Constant::Integer(r.u32()? as i32),
                4 => Constant::Float(f32::from_bits(r.u32()?)),
                5 => {
                    let high = u64::from(r.u32()?);
                    let low = u64::from(r.u32()?);
                    Constant::Long(((high << 32) | low) as i64)
                }
                6 => {
                    let high = u64::from(r.u32()?);
                    let low = u64::from(r.u32()?);
                    Constant::Double(f64::from_bits((high << 32) | low))
                }
                7 => Constant::Class(r.u16()?),
                8 => Constant::String(r.u16()?),
                9 => Constant::Field(r.u16()?, r.u16()?),
                10 => Constant::Method(r.u16()?, r.u16()?),
                11 => Constant::InterfaceMethod(r.u16()?, r.u16()?),
                12 => Constant::NameAndType(r.u16()?, r.u16()?),
                15 => Constant::MethodHandle(r.u8()?, r.u16()?),
                16 => Constant::MethodType(r.u16()?),
                17 => Constant::Dynamic(r.u16()?, r.u16()?),
                18 => Constant::InvokeDynamic(r.u16()?, r.u16()?),
                19 => Constant::Module(r.u16()?),
                20 => Constant::Package(r.u16()?),
                other => bail!("unknown constant tag {other}"),
            };
            let wide = matches!(constant, Constant::Long(_) | Constant::Double(_));
            pool.push(constant);
            if wide {
                pool.push(Constant::Unused);
            }
        }
        let mut class = ClassFile {
            name: String::new(),
            pool,
            fields: Vec::new(),
            methods: Vec::new(),
            bootstraps: Vec::new(),
        };
        r.take(2)?;
        let this = r.u16()?;
        class.name = class
            .class_name(this)
            .ok_or_else(|| anyhow!("class file has no name"))?
            .to_owned();
        r.take(2)?;
        let interfaces = r.u16()? as usize;
        r.take(interfaces * 2)?;
        let fields = r.u16()?;
        for _ in 0..fields {
            let access = r.u16()?;
            let name = class.utf8_or_empty(r.u16()?);
            let descriptor = class.utf8_or_empty(r.u16()?);
            skip_attributes(&mut r)?;
            class.fields.push(Field {
                access,
                name,
                descriptor,
            });
        }
        let methods = r.u16()?;
        for _ in 0..methods {
            let access = r.u16()?;
            let name = class.utf8_or_empty(r.u16()?);
            let descriptor = class.utf8_or_empty(r.u16()?);
            let mut method = Method {
                access,
                name,
                descriptor,
                max_locals: 0,
                code: None,
            };
            let attributes = r.u16()?;
            for _ in 0..attributes {
                let attribute = r.u16()?;
                let len = r.u32()? as usize;
                let body = r.take(len)?;
                if class.utf8(attribute) == Some("Code") {
                    let mut code = Reader { bytes: body, at: 0 };
                    code.take(2)?;
                    method.max_locals = code.u16()?;
                    let length = code.u32()? as usize;
                    method.code = Some(code.take(length)?.to_vec());
                }
            }
            class.methods.push(method);
        }
        let attributes = r.u16()?;
        for _ in 0..attributes {
            let attribute = r.u16()?;
            let len = r.u32()? as usize;
            let body = r.take(len)?;
            if class.utf8(attribute) != Some("BootstrapMethods") {
                continue;
            }
            let mut table = Reader { bytes: body, at: 0 };
            let count = table.u16()?;
            for _ in 0..count {
                let method = table.u16()?;
                let arguments = table.u16()?;
                let mut bootstrap = Bootstrap {
                    method,
                    arguments: Vec::with_capacity(arguments as usize),
                };
                for _ in 0..arguments {
                    bootstrap.arguments.push(table.u16()?);
                }
                class.bootstraps.push(bootstrap);
            }
        }
        Ok(class)
    }

    fn utf8_or_empty(&self, index: u16) -> String {
        self.utf8(index).unwrap_or("").to_owned()
    }

    pub fn constant(&self, index: u16) -> Option<&Constant> {
        self.pool.get(index as usize)
    }

    pub fn utf8(&self, index: u16) -> Option<&str> {
        match self.constant(index)? {
            Constant::Utf8(text) => Some(text),
            _ => None,
        }
    }

    pub fn class_name(&self, index: u16) -> Option<&str> {
        match self.constant(index)? {
            Constant::Class(name) => self.utf8(*name),
            _ => None,
        }
    }

    pub fn string(&self, index: u16) -> Option<&str> {
        match self.constant(index)? {
            Constant::String(text) => self.utf8(*text),
            _ => None,
        }
    }

    pub fn integer(&self, index: u16) -> Option<i32> {
        match self.constant(index)? {
            Constant::Integer(value) => Some(*value),
            _ => None,
        }
    }

    fn name_and_type(&self, index: u16) -> Option<(&str, &str)> {
        match self.constant(index)? {
            Constant::NameAndType(name, descriptor) => {
                Some((self.utf8(*name)?, self.utf8(*descriptor)?))
            }
            _ => None,
        }
    }

    pub fn member(&self, index: u16) -> Option<Member> {
        let (class, name_and_type) = match self.constant(index)? {
            Constant::Field(class, nat)
            | Constant::Method(class, nat)
            | Constant::InterfaceMethod(class, nat) => (*class, *nat),
            _ => return None,
        };
        let (name, descriptor) = self.name_and_type(name_and_type)?;
        Some(Member {
            class: self.class_name(class)?.to_owned(),
            name: name.to_owned(),
            descriptor: descriptor.to_owned(),
        })
    }

    pub fn handle(&self, index: u16) -> Option<Handle> {
        match self.constant(index)? {
            Constant::MethodHandle(kind, reference) => Some(Handle {
                kind: *kind,
                member: self.member(*reference)?,
            }),
            _ => None,
        }
    }

    pub fn invoke_dynamic(&self, index: u16) -> Option<Dynamic> {
        let (bootstrap, name_and_type) = match self.constant(index)? {
            Constant::InvokeDynamic(bootstrap, nat) => (*bootstrap, *nat),
            _ => return None,
        };
        let (name, descriptor) = self.name_and_type(name_and_type)?;
        let entry = self.bootstraps.get(bootstrap as usize)?;
        Some(Dynamic {
            name: name.to_owned(),
            descriptor: descriptor.to_owned(),
            bootstrap: self.handle(entry.method)?,
            arguments: entry.arguments.clone(),
        })
    }

    pub fn method(&self, name: &str) -> Option<&Method> {
        self.methods.iter().find(|method| method.name == name)
    }

    pub fn method_with(&self, name: &str, descriptor: &str) -> Option<&Method> {
        self.methods
            .iter()
            .find(|method| method.name == name && method.descriptor == descriptor)
    }
}

fn skip_attributes(r: &mut Reader) -> Result<()> {
    let attributes = r.u16()?;
    for _ in 0..attributes {
        r.take(2)?;
        let len = r.u32()? as usize;
        r.take(len)?;
    }
    Ok(())
}

pub fn parameters(descriptor: &str) -> Vec<String> {
    let inner = descriptor
        .strip_prefix('(')
        .and_then(|rest| rest.split(')').next())
        .unwrap_or("");
    let bytes = inner.as_bytes();
    let mut out = Vec::new();
    let mut at = 0;
    while at < bytes.len() {
        let start = at;
        while at < bytes.len() && bytes[at] == b'[' {
            at += 1;
        }
        if at < bytes.len() && bytes[at] == b'L' {
            while at < bytes.len() && bytes[at] != b';' {
                at += 1;
            }
        }
        at += 1;
        out.push(inner[start..at.min(bytes.len())].to_owned());
    }
    out
}

pub fn returns_value(descriptor: &str) -> bool {
    !descriptor.ends_with(")V")
}

pub fn instructions(code: &[u8]) -> Result<Vec<Instruction>> {
    let mut out = Vec::new();
    let mut at = 0;
    while at < code.len() {
        let op = code[at];
        let byte = |offset: usize| -> Result<u8> {
            code.get(at + offset)
                .copied()
                .ok_or_else(|| anyhow!("instruction at {at} is cut off"))
        };
        let short = |offset: usize| -> Result<u16> {
            Ok(u16::from_be_bytes([byte(offset)?, byte(offset + 1)?]))
        };
        let int = |offset: usize| -> Result<i32> {
            Ok(i32::from_be_bytes([
                byte(offset)?,
                byte(offset + 1)?,
                byte(offset + 2)?,
                byte(offset + 3)?,
            ]))
        };
        let target = |offset: i64| -> Result<i32> {
            let target = at as i64 + offset;
            if target < 0 || target > code.len() as i64 {
                bail!("jump at {at} leaves the method");
            }
            Ok(target as i32)
        };
        let mut instruction = Instruction {
            at,
            op,
            index: 0,
            value: 0,
        };
        let length = match op {
            ICONST_M1..=ICONST_5 => {
                instruction.value = i32::from(op) - 3;
                1
            }
            BIPUSH => {
                instruction.value = i32::from(byte(1)? as i8);
                2
            }
            SIPUSH => {
                instruction.value = i32::from(short(1)? as i16);
                3
            }
            LDC => {
                instruction.index = u16::from(byte(1)?);
                2
            }
            LDC_W | LDC2_W => {
                instruction.index = short(1)?;
                3
            }
            0x15..=0x19 | 0x36..=0x3a | 0xa9 => {
                instruction.index = u16::from(byte(1)?);
                2
            }
            0x1a..=0x2d => {
                instruction.index = u16::from((op - 0x1a) % 4);
                1
            }
            0x3b..=0x4e => {
                instruction.index = u16::from((op - 0x3b) % 4);
                1
            }
            0xbc => 2,
            IINC => {
                instruction.index = u16::from(byte(1)?);
                instruction.value = i32::from(byte(2)? as i8);
                3
            }
            0x99..=0xa8 | 0xc6 | 0xc7 => {
                instruction.value = target(i64::from(short(1)? as i16))?;
                3
            }
            GOTO_W | 0xc9 => {
                instruction.value = target(i64::from(int(1)?))?;
                5
            }
            TABLESWITCH => {
                let base = (at + 4) & !3;
                let low = i64::from(int(base + 4 - at)?);
                let high = i64::from(int(base + 8 - at)?);
                if high < low {
                    bail!("tableswitch at {at} has no range");
                }
                base + 12 + (high - low + 1) as usize * 4 - at
            }
            LOOKUPSWITCH => {
                let base = (at + 4) & !3;
                let pairs = int(base + 4 - at)?;
                if pairs < 0 {
                    bail!("lookupswitch at {at} has a negative size");
                }
                base + 8 + pairs as usize * 8 - at
            }
            GETSTATIC..=INVOKESTATIC | NEW | 0xbd | 0xc0 | 0xc1 => {
                instruction.index = short(1)?;
                3
            }
            INVOKEINTERFACE | INVOKEDYNAMIC => {
                instruction.index = short(1)?;
                5
            }
            0xc5 => {
                instruction.index = short(1)?;
                instruction.value = i32::from(byte(3)?);
                4
            }
            WIDE => {
                let widened = byte(1)?;
                instruction.op = widened;
                instruction.index = short(2)?;
                if widened == IINC {
                    instruction.value = i32::from(short(4)? as i16);
                    6
                } else {
                    4
                }
            }
            _ => 1,
        };
        if at + length > code.len() {
            bail!("instruction at {at} is cut off");
        }
        out.push(instruction);
        at += length;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Pool {
        bytes: Vec<u8>,
        count: u16,
    }

    impl Pool {
        fn add(&mut self, entry: &[u8]) -> u16 {
            self.count += 1;
            self.bytes.extend_from_slice(entry);
            self.count
        }

        fn utf8(&mut self, text: &str) -> u16 {
            let mut entry = vec![1];
            entry.extend((text.len() as u16).to_be_bytes());
            entry.extend(text.as_bytes());
            self.add(&entry)
        }

        fn pair(&mut self, tag: u8, a: u16, b: u16) -> u16 {
            let mut entry = vec![tag];
            entry.extend(a.to_be_bytes());
            entry.extend(b.to_be_bytes());
            self.add(&entry)
        }

        fn single(&mut self, tag: u8, a: u16) -> u16 {
            let mut entry = vec![tag];
            entry.extend(a.to_be_bytes());
            self.add(&entry)
        }

        fn class(&mut self, name: &str) -> u16 {
            let name = self.utf8(name);
            self.single(7, name)
        }

        fn member(&mut self, tag: u8, class: &str, name: &str, descriptor: &str) -> u16 {
            let class = self.class(class);
            let name = self.utf8(name);
            let descriptor = self.utf8(descriptor);
            let nat = self.pair(12, name, descriptor);
            self.pair(tag, class, nat)
        }
    }

    fn sample() -> Vec<u8> {
        let mut pool = Pool::default();
        let this = pool.class("demo/Sample");
        let text = pool.utf8("hello");
        let string = pool.single(8, text);
        let mut long = vec![5];
        long.extend(7i64.to_be_bytes());
        let long = pool.add(&long);
        pool.count += 1;
        let number = pool.add(&[3, 0, 0, 0, 42]);
        let field = pool.member(9, "demo/Other", "VALUE", "I");
        let target = pool.member(10, "demo/Sample", "lambda$run$0", "(I)V");
        let handle = {
            let mut entry = vec![15, 6];
            entry.extend(target.to_be_bytes());
            pool.add(&entry)
        };
        let factory = pool.member(
            10,
            "java/lang/invoke/LambdaMetafactory",
            "metafactory",
            "()V",
        );
        let factory_handle = {
            let mut entry = vec![15, 6];
            entry.extend(factory.to_be_bytes());
            pool.add(&entry)
        };
        let indy_name = pool.utf8("accept");
        let indy_descriptor = pool.utf8("()Ljava/util/function/IntConsumer;");
        let indy_nat = pool.pair(12, indy_name, indy_descriptor);
        let indy = pool.pair(18, 0, indy_nat);
        let code_name = pool.utf8("Code");
        let bootstrap_name = pool.utf8("BootstrapMethods");
        let method_name = pool.utf8("run");
        let method_descriptor = pool.utf8("(JLjava/lang/String;)V");
        let field_name = pool.utf8("COUNT");
        let field_descriptor = pool.utf8("I");
        let mut code = vec![0x12, string as u8, 0x13];
        code.extend(number.to_be_bytes());
        code.push(0x14);
        code.extend(long.to_be_bytes());
        code.push(GETSTATIC);
        code.extend(field.to_be_bytes());
        code.push(INVOKEDYNAMIC);
        code.extend(indy.to_be_bytes());
        code.extend([0, 0]);
        code.push(0xb1);
        let mut out = vec![0xCA, 0xFE, 0xBA, 0xBE, 0, 0, 0, 65];
        out.extend((pool.count + 1).to_be_bytes());
        out.extend(&pool.bytes);
        out.extend([0, 0x21]);
        out.extend(this.to_be_bytes());
        out.extend([0, 0, 0, 0]);
        out.extend([0, 1, 0, 0x19]);
        out.extend(field_name.to_be_bytes());
        out.extend(field_descriptor.to_be_bytes());
        out.extend([0, 0]);
        out.extend([0, 1, 0, 9]);
        out.extend(method_name.to_be_bytes());
        out.extend(method_descriptor.to_be_bytes());
        out.extend([0, 1]);
        out.extend(code_name.to_be_bytes());
        out.extend(((12 + code.len()) as u32).to_be_bytes());
        out.extend([0, 4, 0, 5]);
        out.extend((code.len() as u32).to_be_bytes());
        out.extend(&code);
        out.extend([0, 0, 0, 0]);
        out.extend([0, 1]);
        out.extend(bootstrap_name.to_be_bytes());
        out.extend(10u32.to_be_bytes());
        out.extend([0, 1]);
        out.extend(factory_handle.to_be_bytes());
        out.extend([0, 2, 0, 0]);
        out.extend(handle.to_be_bytes());
        out
    }

    #[test]
    fn a_class_reads_its_pool_members_and_bootstraps() {
        let class = ClassFile::parse(&sample()).unwrap();
        assert_eq!(class.name, "demo/Sample");
        assert_eq!(class.fields[0].name, "COUNT");
        assert_eq!(class.fields[0].access, 0x19);
        let run = class.method("run").unwrap();
        assert_eq!(run.max_locals, 5);
        assert_eq!(run.descriptor, "(JLjava/lang/String;)V");
        assert!(class.method_with("run", "()V").is_none());
        let program = instructions(run.code.as_ref().unwrap()).unwrap();
        assert_eq!(program.len(), 6);
        assert_eq!(class.string(program[0].index), Some("hello"));
        assert_eq!(class.integer(program[1].index), Some(42));
        assert_eq!(class.constant(program[2].index), Some(&Constant::Long(7)));
        assert_eq!(
            class.constant(program[2].index + 1),
            Some(&Constant::Unused)
        );
        assert_eq!(
            class.member(program[3].index),
            Some(Member {
                class: "demo/Other".to_owned(),
                name: "VALUE".to_owned(),
                descriptor: "I".to_owned(),
            })
        );
        let dynamic = class.invoke_dynamic(program[4].index).unwrap();
        assert_eq!(dynamic.name, "accept");
        assert_eq!(dynamic.descriptor, "()Ljava/util/function/IntConsumer;");
        assert_eq!(
            dynamic.bootstrap.member.class,
            "java/lang/invoke/LambdaMetafactory"
        );
        let target = class.handle(dynamic.arguments[1]).unwrap();
        assert_eq!(target.kind, 6);
        assert_eq!(target.member.name, "lambda$run$0");
        assert!(class.member(program[0].index).is_none());
    }

    #[test]
    fn instructions_decode_operands_jumps_and_wide_forms() {
        let code = [
            0x02, 0x08, BIPUSH, 0xfe, SIPUSH, 0x01, 0x00, 0x2b, 0x4c, IINC, 3, 0xff, WIDE, IINC,
            0x01, 0x00, 0x00, 0x10, WIDE, 0x19, 0x01, 0x02, 0x99, 0xff, 0xf2, GOTO, 0x00, 0x03,
            0x00, 0xb1,
        ];
        let program = instructions(&code).unwrap();
        let ops: Vec<(usize, u8, u16, i32)> = program
            .iter()
            .map(|i| (i.at, i.op, i.index, i.value))
            .collect();
        assert_eq!(
            ops,
            vec![
                (0, 0x02, 0, -1),
                (1, 0x08, 0, 5),
                (2, BIPUSH, 0, -2),
                (4, SIPUSH, 0, 256),
                (7, 0x2b, 1, 0),
                (8, 0x4c, 1, 0),
                (9, IINC, 3, -1),
                (12, IINC, 256, 16),
                (18, 0x19, 258, 0),
                (22, 0x99, 0, 8),
                (25, GOTO, 0, 28),
                (28, 0x00, 0, 0),
                (29, 0xb1, 0, 0),
            ]
        );
    }

    #[test]
    fn switches_skip_their_padding_and_tables() {
        let mut code = vec![0x00, TABLESWITCH, 0, 0];
        code.extend(0i32.to_be_bytes());
        code.extend(1i32.to_be_bytes());
        code.extend(2i32.to_be_bytes());
        code.extend([0; 8]);
        code.push(LOOKUPSWITCH);
        code.extend([0, 0, 0]);
        code.extend(0i32.to_be_bytes());
        code.extend(1i32.to_be_bytes());
        code.extend([0; 8]);
        code.push(0xb1);
        let program = instructions(&code).unwrap();
        let ops: Vec<(usize, u8)> = program.iter().map(|i| (i.at, i.op)).collect();
        assert_eq!(
            ops,
            vec![(0, 0x00), (1, TABLESWITCH), (24, LOOKUPSWITCH), (44, 0xb1)]
        );
    }

    #[test]
    fn broken_code_and_classes_are_errors() {
        assert!(instructions(&[GETSTATIC, 0]).is_err());
        assert!(instructions(&[GOTO, 0x7f, 0x00]).is_err());
        assert!(instructions(&[TABLESWITCH, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 1]).is_err());
        assert!(ClassFile::parse(&[0, 1, 2, 3]).is_err());
        let mut bytes = sample();
        bytes.truncate(bytes.len() - 3);
        assert!(ClassFile::parse(&bytes).is_err());
    }

    #[test]
    fn descriptors_split_into_parameters() {
        assert_eq!(
            parameters("(I[[JLjava/lang/String;[Lnet/A;D)V"),
            vec!["I", "[[J", "Ljava/lang/String;", "[Lnet/A;", "D"]
        );
        assert!(parameters("()Ljava/lang/Object;").is_empty());
        assert!(returns_value("()Ljava/lang/Object;"));
        assert!(!returns_value("(I)V"));
    }
}
