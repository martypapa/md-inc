Include files in Markdown docs
=========================

## Example

<!--{{ markdown-example.md | code: markdown }}-->
```markdown
Look at the following rust code:
<!--{ "file.rs" | code: rust }-->
<!--{ end }-->
This will print 'Hello World' to the console.
```
<!--{{ end }}-->


*file.rs*:
<!--{{ file.rs | code: rust }}-->
```rust
fn main() {
    println!("Hello, World!");
}
```
<!--{{ end }}-->


Generated Code:
<!--{{ markdown-example-filled.md | code: markdown | wrap: "`" }}-->
````markdown
Look at the following rust code:
<!--{ "file.rs" | code: rust }-->
```rust
fn main() {
    println!("Hello, World!");
}
```
<!--{ end }-->
This will print 'Hello World' to the console.
````
<!--{{ end }}-->
This will print 'Hello World' to the console.


## Install
```bash
cargo install md-inc
```

## Run
```bash
md-inc [FLAGS] [OPTIONS] [files]...
```

If no files are given, the `files` field in `.md-inc.toml` is used.

## Customisation

`.md-inc.toml` can be configured by setting any of the following:
```toml
open_tag = "<!--{"
close_tag = "}-->"
end_command = "end"
base_dir = "a/sub/directory"
files = ["README.md", "doc/Other.md"]
```


## Commands
Included files can be manipulated using a few commands.

Syntax examples:
```markdown
<!--{ "file.txt | code }-->
<!--{ "file.py" | code: python }-->
<!--{ "file.py" | code: python | lines: 4 10 }-->
```

A few commands are exposed to make importing easier. 
* The first value should always be the filename.
* Commands can be chained together using the pipe (`|`) operator.
    `"file.txt" | code`
* Commands may take space-separated arguments after a colon (`:`) character.
    `"file.txt | code: python`
* Commands are applied to the included file from left to right.

```markdown
<!--{ "doc/file.txt" }-->
<!--{ end }-->
```



### `code: [language]`
* Wraps in the file in a code block (triple backticks)
* `language`: the language used for syntax highlighting
````markdown
<!--{ "doc/file.txt" | code }-->
```
FILE_CONTENTS
```
<!--{ end }-->
````
````markdown
<!--{ "doc/file.html" | code: html }-->
```html
FILE_CONTENTS
```
<!--{ end }-->
````

### `lines: begin [end]`
* includes only a specific range of lines
* `begin`: Import from this line (zero-indexed)
* `end`: Import until this line (not included)

Given a file, `alphabet.txt`: 
```
A
B
C
D
```

````markdown
<!--{ "alphabet.txt" | lines: 2 }-->
C
D
<!--{ end }-->
````
````markdown
<!--{ "alphabet.txt" | lines: 1 3 }-->
B
C
<!--{ end }-->
````


### `line: list...`
* includes only specific line numbers
* `list...`: A list of line numbers to included

````markdown
<!--{ "alphabet.txt" | lines: 3 2 1 }-->
D
C
B
<!--{ end }-->
````


### `before: text`
* Inserts text before the imported file
* `text`: Text that is inserted before the file (no newline)

### `after: text`
* Inserts text after the imported file
* `text`: Text that is inserted after the file (no newline)

### `wrap: text [after]`
* Inserts text before and after the imported file
* `text`: Text that is inserted before and after the file (no newline)
* `after`: Text that is inserted after the file (no newline). If not set, `text` is used for both.


### `match: pattern [group_num]`
* Inserts text from a file that matches the pattern.
* `pattern`: A regex pattern
* `group_num`: If provided, insert only the captured group for this number.

For a file, `hello_world.rs`:
```rust
// Main
fn main() {
    println!("Hello, World!");
}
// Goodbye
fn goodbye() {
    println!("Goodbye, World!");
}
```

The `main()` function can be extracted like this:
````
<!--{ "file.rs" | code: rust | match: "\n(fn main[\s\S]*\n\}" 1 }-->
```rust
fn main() {
    println!("Hello, World!");
}
```
<!--{ end }-->
````