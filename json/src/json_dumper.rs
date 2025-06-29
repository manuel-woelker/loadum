use loadum::LoadumString;
use loadum::dumper::Dumper;
use loadum::event::Event;
use loadum::result::LoadumResult;
use loadum::value::Value;
use std::io::Write;

pub struct JsonDumper<'write> {
    indentation_level: u32,
    indentation: &'static str,
    write: Box<dyn Write + 'write>,
    state: Vec<DumperState>,
}

#[derive(Debug, PartialEq)]
enum DumperState {
    Initial,
    WantMapping,
    MapInitial,
    MapHasKey,
    MapHasValue,
    ListInitial,
    ListHasValue,
}

impl<'write> JsonDumper<'write> {
    pub fn new(write: impl Write + 'write) -> JsonDumper<'write> {
        JsonDumper {
            write: Box::new(write),
            indentation_level: 0,
            indentation: "\t",
            state: vec![DumperState::Initial],
        }
    }

    fn indent(&mut self) -> LoadumResult<()> {
        for _ in 0..self.indentation_level {
            self.write.write_all(self.indentation.as_bytes())?;
        }
        Ok(())
    }

    fn emit_value(&mut self, value: &Value) -> LoadumResult<()> {
        match value {
            Value::String(s) => {
                self.write.write_all(b"\"")?;
                let escaped_string = escape_string(s);
                self.write.write_all(escaped_string.as_bytes())?;
                self.write.write_all(b"\"")?;
            }
            Value::Number(i) => {
                self.write.write_all(i.to_string().as_bytes())?;
            }
            Value::Boolean(b) => {
                self.write.write_all(if *b { b"true" } else { b"false" })?;
            }
            Value::Null => {
                self.write.write_all(b"null")?;
            }
        }
        Ok(())
    }

    fn emit_comma_if_needed(&mut self) -> LoadumResult<()> {
        let last_value = self.state.last_mut().unwrap();
        let mut write_comma = false;
        let mut indent = false;
        match last_value {
            DumperState::MapInitial => {}
            DumperState::MapHasKey => {
                *last_value = DumperState::MapHasValue;
            }
            DumperState::MapHasValue => {
                write_comma = true;
            }
            DumperState::ListInitial => {
                *last_value = DumperState::ListHasValue;
                indent = true;
            }
            DumperState::ListHasValue => {
                write_comma = true;
                indent = true;
            }
            DumperState::WantMapping => {}
            _ => {
                panic!("Invalid state: {:?}", last_value);
            }
        };
        if write_comma {
            self.write.write_all(b",\n")?;
        }
        if indent {
            self.indent()?;
        }
        Ok(())
    }

    fn emit_indent_if_necessary(&mut self) -> LoadumResult<()> {
        let last_value = self.state.last_mut().unwrap();
        let mut newline = false;
        match last_value {
            DumperState::ListHasValue | DumperState::MapHasValue => {
                newline = true;
            }
            _ => {}
        }
        if newline {
            self.write.write_all(b"\n")?;
            self.indent()?;
        }
        Ok(())
    }
}

macro_rules! assert_state {
    ($self:ident, $expected_state:pat) => {
        let Some(state) = $self.state.last() else {
            panic!("State stack is empty");
        };
        if !matches!(state, $expected_state) {
            panic!(
                "Invalid state: expected {:?}, but was: {:?}",
                stringify!($expected_state),
                state
            );
        }
    };
}

impl Dumper for JsonDumper<'_> {
    fn emit(&mut self, event: &Event) -> LoadumResult<()> {
        match event {
            Event::DocumentStart => {
                assert_state!(self, DumperState::Initial);
                self.state.push(DumperState::WantMapping);
            }
            Event::DocumentEnd => {
                self.state.pop();
                assert_state!(self, DumperState::Initial);
            }
            Event::MapStart => {
                assert_state!(
                    self,
                    DumperState::WantMapping
                        | DumperState::MapHasKey
                        | DumperState::ListInitial
                        | DumperState::ListHasValue
                );
                self.emit_comma_if_needed()?;
                self.state.push(DumperState::MapInitial);
                self.write.write_all(b"{\n")?;
                self.indentation_level += 1;
            }
            Event::MapEnd => {
                assert_state!(self, DumperState::MapInitial | DumperState::MapHasValue);
                self.state.pop();
                self.indentation_level -= 1;
                self.write.write_all(b"\n")?;
                self.indent()?;
                self.write.write_all(b"}")?;
            }
            Event::MapKey(value) => {
                assert_state!(self, DumperState::MapInitial | DumperState::MapHasValue);
                self.emit_comma_if_needed()?;
                *self.state.last_mut().unwrap() = DumperState::MapHasKey;
                self.indent()?;
                self.emit_value(value)?;
                self.write.write_all(b": ")?;
            }
            Event::ListStart => {
                assert_state!(
                    self,
                    DumperState::MapHasKey | DumperState::ListInitial | DumperState::ListHasValue
                );
                self.emit_comma_if_needed()?;

                self.write.write_all(b"[\n")?;
                self.indentation_level += 1;
                self.state.push(DumperState::ListInitial);
            }
            Event::ListEnd => {
                assert_state!(self, DumperState::ListInitial | DumperState::ListHasValue);
                self.state.pop();
                self.indentation_level -= 1;
                self.emit_indent_if_necessary()?;
                self.write.write_all(b"]")?;
            }
            Event::Literal(value) => {
                assert_state!(
                    self,
                    DumperState::MapHasKey | DumperState::ListHasValue | DumperState::ListInitial
                );
                self.emit_comma_if_needed()?;
                self.emit_value(value)?;
            }
        }
        Ok(())
    }
}

fn escape_string(string: &LoadumString) -> LoadumString {
    let mut must_escape = false;
    for c in string.chars() {
        match c {
            '\u{0000}'..='\u{001f}' | '"' | '\\' => {
                must_escape = true;
                break;
            }
            _ => {}
        }
    }
    if !must_escape {
        return string.clone();
    }
    let mut new_string = LoadumString::with_capacity(string.len() + 1);
    for c in string.chars() {
        match c {
            '"' | '\\' => {
                new_string.push('\\');
                new_string.push(c)
            }
            '\t' => {
                new_string.push_str("\\t");
            }
            '\n' => {
                new_string.push_str("\\n");
            }
            '\r' => {
                new_string.push_str("\\r");
            }
            '\u{0000}'..='\u{001f}' => {
                new_string.push('\\');
                new_string.push('u');
                new_string.push_str(&format!("{:04x}", c as u32));
            }
            _ => new_string.push(c),
        }
    }
    LoadumString::from(new_string)
}

#[cfg(test)]
mod tests {
    use super::JsonDumper;
    use expect_test::expect;
    use loadum::dumper::Dumper;
    use loadum::event::Event;
    use loadum::event::Event::{DocumentEnd, DocumentStart, MapEnd, MapStart};
    use std::io::Cursor;

    fn run_test(events: &[Event], expected: expect_test::Expect) {
        let mut cursor = Cursor::new(vec![]);
        let mut dumper = JsonDumper::new(&mut cursor);
        dumper.emit(&DocumentStart).unwrap();
        for event in events {
            dumper.emit(event).unwrap();
        }
        dumper.emit(&DocumentEnd).unwrap();
        drop(dumper);
        let result = String::from_utf8(cursor.into_inner()).unwrap();
        expected.assert_eq(&result);
        assert_valid_json(&result);
    }

    fn assert_valid_json(json: &str) {
        let _value: serde_json::Value = serde_json::from_str(json)
            .unwrap_or_else(|e| panic!("Invalid JSON: {}\n JSON:\n{}", e, json));
    }

    #[test]
    fn test_empty_mapping() {
        run_test(
            &[MapStart, MapEnd],
            expect![[r#"
                {

                }"#]],
        );
    }

    #[test]
    fn test_null_value() {
        run_test(
            &[MapStart, Event::map_key("foo"), Event::null(), MapEnd],
            expect![[r#"
                {
                	"foo": null
                }"#]],
        );
    }

    #[test]
    fn test_string_value() {
        run_test(
            &[
                MapStart,
                Event::map_key("foo"),
                Event::bool(true),
                Event::map_key("fizz"),
                Event::bool(false),
                MapEnd,
            ],
            expect![[r#"
                {
                	"foo": true,
                	"fizz": false
                }"#]],
        );
    }

    #[test]
    fn test_string_escapes() {
        run_test(
            &[
                MapStart,
                Event::map_key("quote"),
                Event::string("\""),
                Event::map_key("backslash"),
                Event::string("\\"),
                Event::map_key("tab"),
                Event::string("\t"),
                Event::map_key("newline"),
                Event::string("\n"),
                Event::map_key("control_chars"),
                Event::string(('\u{0000}'..='\u{001f}').collect::<String>()),
                Event::map_key("unicode"),
                Event::string("👨‍👩‍👦‍👦"),
                Event::map_key("zalgo"),
                Event::string("ļ̴̞̼̒͂o̸̡̱̗͆ă̷̫͉̗̄͂ḑ̶͚̘͆͂̚u̸͇͂̍̌m̶̫̓̈̈́"),
                MapEnd,
            ],
            expect![[r#"
                {
                	"quote": "\"",
                	"backslash": "\\",
                	"tab": "\t",
                	"newline": "\n",
                	"control_chars": "\u0000\u0001\u0002\u0003\u0004\u0005\u0006\u0007\u0008\t\n\u000b\u000c\r\u000e\u000f\u0010\u0011\u0012\u0013\u0014\u0015\u0016\u0017\u0018\u0019\u001a\u001b\u001c\u001d\u001e\u001f",
                	"unicode": "👨‍👩‍👦‍👦",
                	"zalgo": "ļ̴̞̼̒͂o̸̡̱̗͆ă̷̫͉̗̄͂ḑ̶͚̘͆͂̚u̸͇͂̍̌m̶̫̓̈̈́"
                }"#]],
        );
    }

    #[test]
    fn test_bool_value() {
        run_test(
            &[
                MapStart,
                Event::map_key("foo"),
                Event::bool(true),
                Event::map_key("fizz"),
                Event::bool(false),
                MapEnd,
            ],
            expect![[r#"
                {
                	"foo": true,
                	"fizz": false
                }"#]],
        );
    }

    #[test]
    fn test_number_value() {
        run_test(
            &[
                MapStart,
                Event::map_key("zero"),
                Event::number(0.0),
                Event::map_key("one"),
                Event::number(1.0),
                /*              NaN and Infinity are not supported by JSON
                Event::map_key("nan"),
                Event::number(f64::NAN),
                Event::map_key("infinity"),
                Event::number(f64::INFINITY),*/
                Event::map_key("tau"),
                Event::number(std::f64::consts::TAU),
                Event::map_key("googol"),
                Event::number(1e100),
                MapEnd,
            ],
            expect![[r#"
                {
                	"zero": 0,
                	"one": 1,
                	"tau": 6.283185307179586,
                	"googol": 10000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000
                }"#]],
        );
    }

    #[test]
    fn test_empty_sequence() {
        run_test(
            &[
                MapStart,
                Event::map_key("list"),
                Event::ListStart,
                Event::ListEnd,
                MapEnd,
            ],
            expect![[r#"
                {
                	"list": [

                	]
                }"#]],
        );
    }

    #[test]
    fn test_nested_empty_sequence() {
        run_test(
            &[
                MapStart,
                Event::map_key("list"),
                Event::ListStart,
                Event::ListStart,
                Event::ListStart,
                Event::ListEnd,
                Event::ListEnd,
                Event::ListEnd,
                MapEnd,
            ],
            expect![[r#"
                {
                	"list": [
                		[
                			[

                			]
                		]
                	]
                }"#]],
        );
    }

    #[test]
    fn test_mixed_sequence() {
        run_test(
            &[
                MapStart,
                Event::map_key("list"),
                Event::ListStart,
                Event::null(),
                Event::number(1.0),
                Event::bool(true),
                Event::bool(false),
                Event::string("blub"),
                Event::ListEnd,
                MapEnd,
            ],
            expect![[r#"
                {
                	"list": [
                		null,
                		1,
                		true,
                		false,
                		"blub"
                	]
                }"#]],
        );
    }

    #[test]
    fn test_nested_map() {
        run_test(
            &[
                MapStart,
                Event::map_key("a"),
                MapStart,
                Event::map_key("b"),
                MapStart,
                Event::map_key("c"),
                Event::null(),
                MapEnd,
                MapEnd,
                MapEnd,
            ],
            expect![[r#"
                {
                	"a": {
                		"b": {
                			"c": null
                		}
                	}
                }"#]],
        );
    }

    #[test]
    fn test_map_of_list() {
        run_test(
            &[
                MapStart,
                Event::map_key("a"),
                Event::ListStart,
                Event::null(),
                Event::null(),
                Event::ListEnd,
                Event::map_key("b"),
                Event::ListStart,
                Event::null(),
                Event::null(),
                Event::ListEnd,
                MapEnd,
            ],
            expect![[r#"
                {
                	"a": [
                		null,
                		null
                	],
                	"b": [
                		null,
                		null
                	]
                }"#]],
        );
    }

    #[test]
    fn test_list_of_map() {
        run_test(
            &[
                MapStart,
                Event::map_key("a"),
                Event::ListStart,
                MapStart,
                Event::map_key("b"),
                Event::null(),
                MapEnd,
                MapStart,
                Event::map_key("c"),
                Event::null(),
                MapEnd,
                Event::ListEnd,
                MapEnd,
            ],
            expect![[r#"
                {
                	"a": [
                		{
                			"b": null
                		},
                		{
                			"c": null
                		}
                	]
                }"#]],
        );
    }
}
