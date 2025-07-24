# Rust Coding Conventions

## String Interpolation
For format!, println!, info!, debug!, bail!, ensure!, and similar macros, follow these rules:

### Rule 1: Use Direct Variable Names When Possible
When you have a variable and want to use its name as the placeholder, use the variable name directly:

```rust
let name = "John";
let count = 42;
let endpoint = "users";

// GOOD - Direct variable names
println!("Hello {name}");
debug!("Processing {count} items for {endpoint}");
anyhow::bail!("Failed to process {endpoint}");
```

### Rule 2: Use Named Parameters for Method Calls, Properties, or Different Names
When you need to call a method, access a property, or want a different placeholder name than the variable name, use named parameters:

```rust
let items = vec![1, 2, 3];
let user = User { name: "John" };
let config = Config::new();

// GOOD - Method calls and properties need named parameters
println!("Count: {count}", count = items.len());
debug!("User: {user_name}", user_name = user.name);
format!("Config: {debug_info}", debug_info = config.debug());

// GOOD - When you want a different placeholder name
let file_path = "/tmp/data.txt";
println!("Processing {input_file}", input_file = file_path);
```

### Rule 3: NEVER Create Temporary Variables for String Interpolation
Don't create variables just to match placeholder names - use named parameters instead:

```rust
let items = vec![1, 2, 3];

// BAD - Creating temporary variable just for string interpolation
let count = items.len();
println!("Found {count} items");

// GOOD - Use named parameter directly
println!("Found {count} items", count = items.len());
```

### Complete Examples

```rust
// Scenario: Error messages with mixed variable types
let file_name = "data.txt";
let lines = vec!["line1", "line2"];
let max_size = 1000;

// GOOD - Mix of direct variables and named parameters
anyhow::bail!(
    "File {file_name} has {line_count} lines, exceeds max {max_size}",
    line_count = lines.len()
);

// Scenario: URL construction
let domain = "example.com";
let user = "john";
let port = config.get_port();

// GOOD - Direct variables + named parameter for method call
let url = format!("https://{domain}:{port}/users/{user}", port = config.get_port());
```

### What NOT to Do

```rust
// BAD - Positional arguments
println!("Hello {}", name);

// BAD - Redundant named parameters when variable name matches
let name = "John";
println!("Hello {name}", name = name);

// BAD - Creating temporary variables
let len = items.len();
println!("Count: {len}");  // Should be: println!("Count: {len}", len = items.len());

// BAD - Using different names when not needed
let name = "John";
println!("Hello {user}", user = name);  // Should be: println!("Hello {name}");
```

## Error Handling

### Correct Usage:
- ALWAYS use anyhow for error handling, particularly bail! and ensure!:
  ```rust
  // For conditional checks
  ensure!(condition, "Error message with {value}");
  
  // For early returns with errors
  bail!("Failed with error: {error_message}");
  
  // For adding context to errors
  let result = some_operation.context("Operation failed")?;
  ```

### Incorrect Usage:
- NEVER use unwrap() or panic!:
  ```rust
  // BAD - Will crash on None:
  let result = optional_value.unwrap();
  
  // BAD - Will crash on Err:
  let data = fallible_operation().unwrap();
  
  // BAD - Explicit panic:
  panic!("This failed");
  ```

- Avoid using .ok() or .expect() to silently ignore errors:
  ```rust
  // BAD - Silently ignores errors:
  std::fs::remove_file(path).ok();
  
  // BETTER - Log the error but continue:
  if let Err(e) = std::fs::remove_file(path) {
      debug!("Failed to remove file: {e}");
  }
  ```
