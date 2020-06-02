<!--{ "hello_world.rs" | match: "\n(fn main[\s\S]*?\n\})" 1 | code: rust }-->
```rust
fn main() {
    println!("Hello, World!");
}
```
<!--{ end }-->