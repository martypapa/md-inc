use crate::parse::{transform, Command, Parser, ParserConfig};
use crate::{transform_files_with_args, Args};
use std::path::Path;
use crate::config;
use crate::config::Config;

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
    let (cfg, _files) = config::find_config(&dir).unwrap().unwrap();
    assert_eq!(cfg.base_dir, dir.join("include"));
}
#[test]
fn read_config_none() {
    let dir = Path::new(&env!("CARGO_MANIFEST_DIR"))
        .to_path_buf()
        .join("test_helpers")
        .join("no_include");
    println!("DIR: {}", &dir.to_str().unwrap());
    let cfg = config::find_config(&dir);
    assert!(matches!(cfg, Ok(None)));
}

#[test]
fn from_args() {
    let root_dir = Path::new(&env!("CARGO_MANIFEST_DIR")).to_path_buf();
    let res = transform_files_with_args(Args {
        files: vec![root_dir.join("test_helpers").join("short.md")],
        base_dir: Some(root_dir.join("test_helpers").join("include_dir")),
        ignore_config: true,
        read_only: true,
        print: true,
        ..Default::default()
    })
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
    let res = transform_files_with_args(Args {
        files: vec![root_dir.join("test_helpers").join("short_2.md")],
        base_dir: Some(root_dir.join("test_helpers").join("include_dir")),
        ignore_config: true,
        open_tag: Some("<!--(".to_string()),
        close_tag: Some(")-->".to_string()),
        end_command: Some("".to_string()),
        read_only: true,
        print: true,
        ..Default::default()
    })
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
    let (parser, files) =
        Config {
            open_tag: "(".to_string(),
            close_tag: ")".to_string(),
            end_command: "end".to_string(),
            base_dir: "include".to_string(),
            files: vec!["file".to_string()],
        }.into_parser(&Path::new("root").to_path_buf());
    assert_eq!(
        files.first().unwrap().as_path(),
        Path::new("root").join("file")
    );

    assert_eq!(parser.base_dir, Path::new("root").join("include"))
}

#[test]
fn custom_config_file() {
    let root_dir = Path::new(&env!("CARGO_MANIFEST_DIR")).to_path_buf();
    let res = transform_files_with_args(Args {
        config: Some(root_dir.join("test_helpers").join("custom_config.toml")),
        read_only: true,
        print: true,
        ..Default::default()
    })
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
    let original = r#"0
1
2
3
4"#;
    let expected = r#"2
3"#;

    let cmd = Command::new("lines", vec!["2", "4"]);
    let parsed = transform(original, &cmd).unwrap();
    assert_eq!(parsed, expected);
}

#[test]
fn cmd_line() {
    let original = r#"0
1
2
3"#;
    let expected = r#"2"#;
    let cmd = Command::new("line", vec!["2"]);
    let parsed = transform(original, &cmd).unwrap();
    assert_eq!(parsed, expected);

    let expected = r#"2
0"#;
    let cmd = Command::new("line", vec!["2", "0"]);
    let parsed = transform(original, &cmd).unwrap();
    assert_eq!(parsed, expected);
}

#[test]
fn cmd_before() {
    let original = r#"txt"#;
    let expected = r#"abc_txt"#;
    let cmd = Command::new("before", vec!["abc_"]);
    let parsed = transform(original, &cmd).unwrap();
    assert_eq!(parsed, expected);
}
#[test]
fn cmd_after() {
    let original = r#"txt"#;
    let expected = r#"txt
abc"#;
    let cmd = Command::new("after", vec!["\\nabc"]);
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
