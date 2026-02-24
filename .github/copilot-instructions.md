<!--
SPDX-License-Identifier: Apache-2.0
SPDX-FileCopyrightText: 2026 Alexander Mohr
-->

# Instructions

* Run `cargo fmt` to format the code according to Rust's standard style.
* Run `cargo clippy` to check for common mistakes and improve the code quality.

## Strucutre
* The project structure in src/tree should reflect how the tree is shown. All elements that have children should be reflected in the tree as such by using modules.
* If a cell is used as jump source, the color of this cell should be blue
* Database description is found in database.fbs
* Never modify dependencies in Cargo.toml without approval. If you think a new dependency is needed, ask first.
* Never change code that is downloaded from a dependency. If you think a change is needed, ask first and we can submit a PR to the dependency.
* Jump or link means that clicking on the element in the UI should navigate to the target element in the UI. This should be implemented using the existing navigation system and not by re-rendering the entire tree or using a different mechanism, also don't implemenet a popup or something like that. The target element should be focused and highlighted as if the user had navigated to it manually.
* Do not disable clippy warnings or prefix parameters with _ to silence warnings. If clippy is complaining, fix the underlying issue or ask if it is a false positive.

## Style guide

### Control Flow: Use Iterator Chains, Not for Loops

```rust
// DON'T
let mut results = Vec::new();
for item in items {
    if item.is_valid() {
        results.push(item.process());
    }
}
```

```rust
// DO
let results: Vec<_> = items
    .iter()
    .filter(|item| item.is_valid())
    .map(|item| item.process())
    .collect();
```

```rust
// DON'T
let mut total = 0;
for value in values {
    total += value.amount();
}
```

```rust
// DO
let total: i64 = values.iter().map(|v| v.amount()).sum();
```

### Error Handling: Use `?` Operator, Not `unwrap()`

```rust
// DON'T
fn read_file(path: &str) -> String {
    std::fs::read_to_string(path)
        .expect("Failed to read file")
}
```

```rust
// DO
fn read_file(path: &str) -> Result<String, std::io::Error> {
    std::fs::read_to_string(path)
}
```

### Early Returns: Use let ... else
```rust
// DO
let Some(user) = get_user(id) else {
    return Err(Error::NotFound);
};
let Ok(session) = user.active_session() else {
    return Err(Error::NoSession);
};
// continue with user and session

// DON'T
if let Some(user) = get_user(id) {
    if let Ok(session) = user.active_session() {
        // deeply nested code
    } else {
        return Err(Error::NoSession);
    }
} else {
    return Err(Error::NotFound);
}
```

```rust
// DO
let Some(value) = maybe_value else { continue };
let Ok(parsed) = input.parse::<i32>() else { continue };

// DON'T
if let Some(value) = maybe_value {
    if let Ok(parsed) = input.parse::<i32>() {
        // ...
    }
}
```

### Variable Naming: Shadow, Don't Rename
```rust
// DO
let input = get_raw_input();
let input = input.trim();
let input = input.to_lowercase();
let input = parse(input)?;

// DON'T
let raw_input = get_raw_input();
let trimmed_input = raw_input.trim();
let lowercase_input = trimmed_input.to_lowercase();
let parsed_input = parse(lowercase_input)?;
```

```rust
// DO
let path = args.path;
let path = path.canonicalize()?;
let path = path.join("config.toml");

// DON'T
let input_path = args.path;
let canonical_path = input_path.canonicalize()?;
let config_path = canonical_path.join("config.toml");
```


### Comments
* Keep to a minimum, no obvious comments.
* Good code should be self-explanatory.
* If using comments, explain the "why" behind a decision, not the "what".


### Type Safety
* Never use `unwrap()`, `expect()`, or `panic!()` in production code. Always handle errors gracefully with `Result` and the `?` operator.
* Avoid using `unsafe` code.
* Do not use String comparisons for logic. Use Enums or Structs instead.

### Pattern Matching: Never Use Wildcard Matches
```rust
// DO
match status {
    Status::Pending => handle_pending(),
    Status::Active => handle_active(),
    Status::Completed => handle_completed(),
}

// DON'T
match status {
    Status::Pending => handle_pending(),
    _ => handle_other(),
}
```

If a wildcard makes sense, ask the user if it is ok.


### Code Navigation: Always Use rust-analyzer LSP

When searching or navigating Rust code, always use the LSP tool with rust-analyzer operations:

* goToDefinition - Find where a symbol is defined
* findReferences - Find all references to a symbol
* hover - Get type info and documentation
* documentSymbol - Get all symbols in a file
* goToImplementation - Find trait implementations


### Ownership: Borrow Instead of Clone

Prefer borrowing (`&T`, `&mut T`) over cloning. Only clone when ownership transfer is truly needed.

```rust
// DO
fn process(data: &str) -> Result<()> {
    println!("{data}");
    Ok(())
}

// DON'T
fn process(data: String) -> Result<()> {
    println!("{data}");
    Ok(())
}
```

```rust
// DO
fn longest<'a>(a: &'a str, b: &'a str) -> &'a str {
    if a.len() > b.len() { a } else { b }
}

// DON'T
fn longest(a: &str, b: &str) -> String {
    if a.len() > b.len() { a.to_string() } else { b.to_string() }
}
```


### Conversions: Use `From`/`Into` Traits, Not Ad-Hoc Methods

```rust
// DO
impl From<RawConfig> for AppConfig {
    fn from(raw: RawConfig) -> Self {
        Self {
            name: raw.name,
            timeout: Duration::from_secs(raw.timeout_secs),
        }
    }
}
let config: AppConfig = raw_config.into();

// DON'T
impl RawConfig {
    fn to_app_config(&self) -> AppConfig {
        AppConfig {
            name: self.name.clone(),
            timeout: Duration::from_secs(self.timeout_secs),
        }
    }
}
let config = raw_config.to_app_config();
```


### Display: Implement `Display`, Not Custom `to_string()` Methods

```rust
// DO
impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Status::Pending => write!(f, "pending"),
            Status::Active => write!(f, "active"),
            Status::Completed => write!(f, "completed"),
        }
    }
}

// DON'T
impl Status {
    fn to_string(&self) -> String {
        match self {
            Status::Pending => "pending".to_string(),
            Status::Active => "active".to_string(),
            Status::Completed => "completed".to_string(),
        }
    }
}
```


### Strings: Use Format Strings, Not Concatenation

```rust
// DO
let msg = format!("{name} has {count} items");
println!("Processing {path:?}");

// DON'T
let msg = name.to_string() + " has " + &count.to_string() + " items";
println!("Processing {:?}", path);
```


### Constructors: Use `Default` and Builder Patterns

```rust
// DO
#[derive(Default)]
struct Config {
    retries: u32,
    verbose: bool,
    timeout: Option<Duration>,
}

let config = Config {
    retries: 3,
    ..Config::default()
};

// DON'T
let config = Config {
    retries: 3,
    verbose: false,
    timeout: None,
};
```


### Derive: Use Derive Macros Over Manual Implementations

Derive standard traits instead of implementing them manually when the default derivation is correct.

```rust
// DO
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Point {
    x: i32,
    y: i32,
}

// DON'T (unless custom logic is required)
impl PartialEq for Point {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}
```


### Newtype Pattern: Wrap Primitive Types for Semantic Meaning

```rust
// DO
struct UserId(u64);
struct Email(String);

fn send_email(to: Email, from: Email) { /* ... */ }

// DON'T
fn send_email(to: String, from: String) { /* ... */ }
```


### Closures: Prefer Closures Over Named Functions for Short Logic

```rust
// DO
items.iter().filter(|i| i.is_active()).count()

// DON'T
fn is_active(item: &&Item) -> bool {
    item.is_active()
}
items.iter().filter(is_active).count()
```


### Option/Result Combinators: Use `map`, `and_then`, `unwrap_or_else`

```rust
// DO
let name = user
    .nickname()
    .or_else(|| user.full_name())
    .unwrap_or_else(|| "anonymous".to_string());

// DON'T
let name = if let Some(n) = user.nickname() {
    n
} else if let Some(n) = user.full_name() {
    n
} else {
    "anonymous".to_string()
};
```

```rust
// DO
let port = config.port.unwrap_or(8080);

// DON'T
let port = match config.port {
    Some(p) => p,
    None => 8080,
};
```


### Slices: Accept `&[T]` and `&str`, Not `&Vec<T>` and `&String`

```rust
// DO
fn process(items: &[Item]) { /* ... */ }
fn greet(name: &str) { /* ... */ }

// DON'T
fn process(items: &Vec<Item>) { /* ... */ }
fn greet(name: &String) { /* ... */ }
```


### Enums: Use Enums With Data Over Separate Structs

```rust
// DO
enum Shape {
    Circle { radius: f64 },
    Rectangle { width: f64, height: f64 },
}

fn area(shape: &Shape) -> f64 {
    match shape {
        Shape::Circle { radius } => std::f64::consts::PI * radius * radius,
        Shape::Rectangle { width, height } => width * height,
    }
}

// DON'T
struct Circle { radius: f64 }
struct Rectangle { width: f64, height: f64 }
// then use trait objects or separate functions
```


### Iterators: Prefer `iter()` Method Chains Over Index Access

```rust
// DO
for (i, item) in items.iter().enumerate() {
    println!("{i}: {item}");
}

// DON'T
for i in 0..items.len() {
    println!("{}: {}", i, items[i]);
}
```


### Tests: Use `#[cfg(test)]` Module in the Same File

```rust
// DO — tests at the bottom of the same file
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_input() {
        let result = parse("42");
        assert_eq!(result, Ok(42));
    }
}
```
