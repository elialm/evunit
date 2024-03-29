#[warn(dead_code)]
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize, Debug, PartialEq, Eq, Hash, Clone)]
#[serde(untagged)]
enum Address {
	Raw(u16),
	Label(String),
}

impl Default for Address {
	fn default() -> Self {
		Self::Raw(0xFFFF)
	}
}

#[derive(Deserialize, Debug, PartialEq, Eq, Hash, Clone)]
#[serde(untagged)]
enum Addresses {
	Single(Address),
	Multiple(Vec<Address>),
}

impl Default for Addresses {
	fn default() -> Self {
		Self::Single(Address::default())
	}
}

impl TryFrom<&String> for Address {
	type Error = &'static str;

	fn try_from(key: &String) -> Result<Self, Self::Error> {
		// TODO: actually implement
		Ok(Self::Raw(0xFFFF))
	}
}

#[derive(Deserialize, Debug, Default, Clone, PartialEq)]
struct Registers {
	a: Option<u8>,
	b: Option<u8>,
	c: Option<u8>,
	d: Option<u8>,
	e: Option<u8>,
	h: Option<u8>,
	l: Option<u8>,
	bc: Option<Address>,
	de: Option<Address>,
	hl: Option<Address>,
	pc: Option<Address>,
	sp: Option<Address>,
}

#[derive(Deserialize, Debug, Default, Clone, PartialEq)]
struct ConfigResult {
	#[serde(flatten)]
	resulting_registers: Registers,

	#[serde(flatten)]
	resulting_memory: HashMap<Address, MemoryAssignment>,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Default)]
struct ConfigTest {
	#[serde(flatten)]
	initial_registers: Registers,

	#[serde(flatten)]
	initial_memory: HashMap<Address, MemoryAssignment>,

	result: Option<ConfigResult>,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
enum MemoryValue {
	Byte(u8),
	String(String),
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[serde(untagged)]
enum MemoryAssignment {
	Byte(u8),
	String(String),
	Array(Vec<MemoryValue>),
}

#[derive(Deserialize, Debug, PartialEq)]
#[serde(untagged)]
enum ConfigOther {
	Memory(MemoryAssignment),
	Test(ConfigTest),
}

impl ConfigOther {
	fn unwrap_test(&self) -> &ConfigTest {
		match self {
			Self::Test(test) => test,
			_ => panic!("ConfigOther::unwrap_test called on non-Test variant"),
		}
	}

	fn unwrap_memory(&self) -> &MemoryAssignment {
		match self {
			Self::Memory(mem) => mem,
			_ => panic!("ConfigOther::unwrap_memory called on non-Memory variant"),
		}
	}
}

fn default_enable_breakpoint() -> bool {
	false
}

fn default_timeout() -> u16 {
	u16::MAX
}

#[derive(Deserialize, Debug, PartialEq)]
struct ConfigMeta {
	#[serde(default)]
	caller: Address,

	#[serde(default = "default_enable_breakpoint")]
	enable_breakpoints: bool,

	#[serde(default = "default_timeout")]
	timeout: u16,

	#[serde(default)]
	crash: Addresses,

	#[serde(default)]
	exit: Addresses,

	stack: Option<MemoryAssignment>,
}

impl Default for ConfigMeta {
	fn default() -> Self {
		Self {
			caller: Default::default(),
			enable_breakpoints: default_enable_breakpoint(),
			timeout: default_timeout(),
			crash: Default::default(),
			exit: Default::default(),
			stack: Default::default(),
		}
	}
}

#[derive(Deserialize, Debug, PartialEq)]
struct UnprocessedConfigFile {
	#[serde(flatten)]
	pub default_config: ConfigMeta,

	#[serde(flatten)]
	pub default_registers: Registers,

	#[serde(flatten)]
	pub others: HashMap<String, ConfigOther>,
}

impl UnprocessedConfigFile {
	fn tests(&self) -> impl Iterator<Item = ConfigTest> + '_ {
		self.others
			.iter()
			.filter(|(key, value)| matches!(value, ConfigOther::Test(_)))
			.map(|(name, config)| config.unwrap_test().clone())
	}

	fn memory_assignments(&self) -> impl Iterator<Item = (Address, MemoryAssignment)> + '_ {
		self.others
			.iter()
			.filter(|(key, value)| matches!(value, ConfigOther::Memory(_)))
			.map(|(key, value)| (key, value.unwrap_memory().clone()))
			// TODO: do proper error handling
			.map(|(key, memory)| (Address::try_from(key).unwrap(), memory))
	}
}

#[derive(Debug)]
struct ConfigFile {
	default_config: ConfigMeta,
	default_registers: Registers,
	default_memory: HashMap<Address, MemoryAssignment>,
	tests: Vec<ConfigTest>,
}

impl TryFrom<UnprocessedConfigFile> for ConfigFile {
	type Error = &'static str;

	fn try_from(uconfig: UnprocessedConfigFile) -> Result<Self, Self::Error> {
		// TODO: do some proper error checking, for now just make a ConfigFile

		Ok(ConfigFile {
			default_memory: uconfig.memory_assignments().collect(),
			tests: uconfig.tests().collect(),
			default_config: uconfig.default_config,
			default_registers: uconfig.default_registers,
		})
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use pretty_assertions::assert_eq;
	use std::fs;

	#[test]
	fn parse_default() {
		let rconfig = fs::read_to_string("test/configs/default.toml").unwrap();
		let uconfig = toml::from_str::<UnprocessedConfigFile>(&rconfig);

		assert_eq!(
			uconfig,
			Ok(UnprocessedConfigFile {
				default_config: ConfigMeta::default(),
				default_registers: Registers::default(),
				others: HashMap::from([(
					"default".to_string(),
					ConfigOther::Test(ConfigTest::default())
				)]),
			})
		);
	}

	#[test]
	fn parse_basic() {
		let rconfig = fs::read_to_string("test/configs/basic.toml").unwrap();
		let uconfig = toml::from_str::<UnprocessedConfigFile>(&rconfig);

		assert_eq!(
			uconfig,
			Ok(UnprocessedConfigFile {
				default_config: ConfigMeta {
					caller: Address::Raw(0x0100),
					enable_breakpoints: default_enable_breakpoint(),
					timeout: default_timeout(),
					crash: Default::default(),
					exit: Addresses::Single(Address::Label("exit".to_string())),
					stack: Default::default(),
				},
				default_registers: Registers::default(),
				others: HashMap::from([(
					"test1".to_string(),
					ConfigOther::Test(ConfigTest {
						initial_registers: Registers {
							a: Some(9),
							b: None,
							c: None,
							d: None,
							e: None,
							h: None,
							l: None,
							bc: None,
							de: None,
							hl: None,
							pc: None,
							sp: None
						},
                        initial_memory: Default::default(),
                        result: Some(ConfigResult {
                            resulting_registers: Registers {
                                a: Some(10),
                                b: None,
                                c: None,
                                d: None,
                                e: None,
                                h: None,
                                l: None,
                                bc: None,
                                de: None,
                                hl: None,
                                pc: None,
                                sp: None
                            },
                            resulting_memory: Default::default(),
                        })
					})
				)]),
			})
		);
	}
}
