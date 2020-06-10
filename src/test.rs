use crate::config::{Config, ConfigAndPath};
use crate::parse::{transform, Command, Parser, ParserConfig};
use crate::{transform_files_with_args, Args};
use std::path::Path;

#[test]
fn with_language() {
    let original = r#"
start
<!--{ code_snippet.rs | code: rust }-->
<!--{ end }-->
end"#;
    let expected = r#"
start
<!--{ code_snippet.rs | code: rust }-->
```rust
fn main() {
    println!("Hello World!");
}
```
<!--{ end }-->
end"#;
    let parsed = Parser {
        config: ParserConfig {
            base_dir: "test_helpers".into(),
            ..Default::default()
        },
        content: original.to_string(),
    }
    .parse()
    .unwrap();
    assert_eq!(parsed, expected);
}
#[test]
fn code() {
    let original = r#"
start
<!--{ code_snippet.rs | code }-->
<!--{ end }-->
end"#;
    let expected = r#"
start
<!--{ code_snippet.rs | code }-->
```
fn main() {
    println!("Hello World!");
}
```
<!--{ end }-->
end"#;
    let parsed = Parser {
        config: ParserConfig {
            base_dir: "test_helpers".into(),
            ..Default::default()
        },
        content: original.to_string(),
    }
    .parse()
    .unwrap();
    assert_eq!(parsed, expected);
}

#[test]
fn raw_input() {
    let original = r#"
start
<!--{ code_snippet.rs }-->
<!--{ end }-->
end"#;
    let expected = r#"
start
<!--{ code_snippet.rs }-->
fn main() {
    println!("Hello World!");
}
<!--{ end }-->
end"#;
    let parsed = Parser {
        config: ParserConfig {
            base_dir: "test_helpers".into(),
            ..Default::default()
        },
        content: original.to_string(),
    }
    .parse()
    .unwrap();
    assert_eq!(parsed, expected);
}

#[test]
fn read_config_found() {
    let dir = Path::new(&env!("CARGO_MANIFEST_DIR"))
        .to_path_buf()
        .join("test_helpers")
        .join("with_include");

    println!("DIR: {}", &dir.to_str().unwrap());
    let (cfg, _files) = Config::try_from_dir(&dir)
        .unwrap()
        .unwrap()
        .into_parser()
        .unwrap();
    assert_eq!(cfg.base_dir, dir.join("include"));
}
#[test]
fn read_config_none() {
    let dir = Path::new(&env!("CARGO_MANIFEST_DIR"))
        .to_path_buf()
        .join("test_helpers")
        .join("no_include");
    println!("DIR: {}", &dir.to_str().unwrap());
    let cfg = Config::try_from_dir(&dir);
    assert!(matches!(cfg, Ok(None)));
}

#[test]
fn from_args() {
    let root_dir = Path::new(&env!("CARGO_MANIFEST_DIR")).to_path_buf();
    let res = transform_files_with_args(
        Args {
            files: vec![root_dir.join("test_helpers").join("short.md")],
            base_dir: Some(root_dir.join("test_helpers").join("include_dir")),
            ignore_config: true,
            read_only: true,
            print: true,
            ..Default::default()
        },
        None,
    )
    .unwrap();
    let expected: Vec<String> = vec![r#"1
<!--{include_me.txt}-->
INCLUDED_CONTENT
<!--{end}-->
2"#
    .to_string()];
    assert_eq!(res, expected);
}

#[test]
fn from_args_custom_tags() {
    let root_dir = Path::new(&env!("CARGO_MANIFEST_DIR")).to_path_buf();
    let res = transform_files_with_args(
        Args {
            files: vec![root_dir.join("test_helpers").join("short_2.md")],
            base_dir: Some(root_dir.join("test_helpers").join("include_dir")),
            ignore_config: true,
            open_tag: Some("<!--(".to_string()),
            close_tag: Some(")-->".to_string()),
            end_command: Some("".to_string()),
            read_only: true,
            print: true,
            ..Default::default()
        },
        None,
    )
    .unwrap();
    let expected: Vec<String> = vec![r#"1
<!--(include_me.txt | code: rust)-->
```rust
INCLUDED_CONTENT
```
<!--()-->
2"#
    .to_string()];
    assert_eq!(res, expected);
}

#[test]
fn process_config_correct() {
    let (parser, files) = ConfigAndPath {
        config: Config {
            open_tag: "(".to_string(),
            close_tag: ")".to_string(),
            end_command: "end".to_string(),
            base_dir: "include".to_string(),
            files: vec!["file".to_string()],
            next_dirs: vec![],
            depend_dirs: vec![],
            out_dir: None,
        },
        path: Path::new("root").join(".md-inc.toml"),
    }
    .into_parser()
    .unwrap();
    assert_eq!(
        files.first().unwrap().as_path(),
        Path::new("root").join("file")
    );

    assert_eq!(parser.base_dir, Path::new("root").join("include"))
}

#[test]
fn custom_config_file() {
    let root_dir = Path::new(&env!("CARGO_MANIFEST_DIR")).to_path_buf();
    let config_path = root_dir.join("test_helpers").join("custom_config.toml");

    let config = Config::try_from_path(&config_path).expect("Bad config path");
    let res = transform_files_with_args(
        Args {
            config: Some(config_path.clone()),
            read_only: true,
            print: true,
            ..Default::default()
        },
        Some(ConfigAndPath {
            config,
            path: config_path,
        }),
    )
    .unwrap();
    let expected: Vec<String> = vec![r#"1
<!--(include_me.txt)-->
INCLUDED_CONTENT
<!--()-->
2"#
    .to_string()];
    assert_eq!(res, expected);
}

#[test]
fn cmd_lines() {
    let original = r#"1
2
3
4
5"#;
    let expected = r#"2
3
4"#;

    let cmd = Command::new("lines", vec!["2", "4"]);
    let parsed = transform(original, &cmd).unwrap();
    assert_eq!(parsed, expected);
}

#[test]
fn cmd_line() {
    let original = r#"1
2
3
4"#;
    let expected = r#"2"#;
    let cmd = Command::new("line", vec!["2"]);
    let parsed = transform(original, &cmd).unwrap();
    assert_eq!(parsed, expected);

    let expected = r#"2
1"#;
    let cmd = Command::new("line", vec!["2", "1"]);
    let parsed = transform(original, &cmd).unwrap();
    assert_eq!(parsed, expected);
}

#[test]
fn cmd_before() {
    let original = r#"txt"#;
    let expected = r#"abc_txt"#;
    let cmd = Command::new("wrap", vec!["abc_", ""]);
    let parsed = transform(original, &cmd).unwrap();
    assert_eq!(parsed, expected);
}
#[test]
fn cmd_after() {
    let original = r#"txt"#;
    let expected = r#"txt
abc"#;
    let cmd = Command::new("wrap", vec!["", "\\nabc"]);
    let parsed = transform(original, &cmd).unwrap();
    assert_eq!(parsed, expected);
}
#[test]
fn cmd_wrap() {
    let original = r#"txt"#;
    let expected = r#"vvvvvvvv
txt
^^^^^^^^"#;
    let cmd = Command::new("wrap", vec!["vvvvvvvv\\n", "\\n^^^^^^^^"]);
    let parsed = transform(original, &cmd).unwrap();
    assert_eq!(parsed, expected);
}

#[test]
fn cmd_wrap_same() {
    let original = r#"txt"#;
    let expected = r#"--txt--"#;
    let cmd = Command::new("wrap", vec!["--"]);
    let parsed = transform(original, &cmd).unwrap();
    assert_eq!(parsed, expected);
}

#[test]
fn cmd_wrap_lines() {
    let original = r#"a
b
c"#;
    let expected = r#"<<a>>
<<b>>
<<c>>"#;
    let cmd = Command::new("wrap-lines", vec!["<<", ">>"]);
    let parsed = transform(original, &cmd).unwrap();
    assert_eq!(parsed, expected);
}

#[test]
fn cmd_wrap_lines_same() {
    let original = r#"a
b
c"#;
    let expected = r#"```a```
```b```
```c```"#;
    let cmd = Command::new("wrap-lines", vec!["```"]);
    let parsed = transform(original, &cmd).unwrap();
    assert_eq!(parsed, expected);
}
#[test]
fn cmd_line_numbers() {
    let original = r#"a
b
c"#;
    let expected = r#"1: a
2: b
3: c"#;
    let cmd = Command::new("line-numbers", vec![]);
    let parsed = transform(original, &cmd).unwrap();
    assert_eq!(parsed, expected);
    let expected = r#"1) a
2) b
3) c"#;
    let cmd = Command::new("line-numbers", vec![") "]);
    let parsed = transform(original, &cmd).unwrap();
    assert_eq!(parsed, expected);
}

#[test]
fn cmd_line_numbers_width() {
    let original = r#"a
b
c
d
e
f
g
h
i
j
k
l"#;
    let expected = r#" 1: a
 2: b
 3: c
 4: d
 5: e
 6: f
 7: g
 8: h
 9: i
10: j
11: k
12: l"#;
    let cmd = Command::new("line-numbers", vec![]);
    let parsed = transform(original, &cmd).unwrap();
    assert_eq!(parsed, expected);
    let expected = r#"   1: a
   2: b
   3: c
   4: d
   5: e
   6: f
   7: g
   8: h
   9: i
  10: j
  11: k
  12: l"#;
    let cmd = Command::new("line-numbers", vec![": ", "4"]);
    let parsed = transform(original, &cmd).unwrap();
    assert_eq!(parsed, expected);
}

#[test]
fn cmd_match() {
    let original = r#"
// Comments
// More comments
fn main() {
    println!("Hello, World!");
}
// End comments
fn other() {
}
//etc...
"#;
    let expected = r#"fn main() {
    println!("Hello, World!");
}"#;
    let cmd = Command::new("match", vec![r#"\n(fn main[\s\S]+?\n\})"#, "1"]);
    let parsed = transform(original, &cmd).unwrap();
    assert_eq!(parsed, expected);
}
