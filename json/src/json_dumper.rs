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
                self.write.write_all(s.as_bytes())?;
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

impl<'write> Dumper for JsonDumper<'write> {
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
    }

    #[test]
    fn test_empty_document() {
        run_test(&[], expect![]);
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
                Event::map_key("nan"),
                Event::number(f64::NAN),
                Event::map_key("infinity"),
                Event::number(f64::INFINITY),
                Event::map_key("tau"),
                Event::number(std::f64::consts::TAU),
                MapEnd,
            ],
            expect![[r#"
                {
                	"zero": 0,
                	"one": 1,
                	"nan": NaN,
                	"infinity": inf,
                	"tau": 6.283185307179586
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
