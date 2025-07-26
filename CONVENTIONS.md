# Rust Coding Conventions

## String Interpolation

For `format!`, `println!`, `debug!`, `bail!`, and similar macros:

### Rules:
1. **Use direct variable names** when the variable name matches the placeholder:
   ```rust
   let name = "John";
   println!("Hello {name}"); // GOOD
   println!("Hello {}", name); // BAD - avoid positional
   ```

2. **Use named parameters** for method calls or when you want a different name:
   ```rust
   println!("Count: {count}", count = items.len()); // Method call
   println!("Processing {input_file}", input_file = file_path); // Different name
   ```

3. **Never create temporary variables** just for string interpolation:
   ```rust
   // BAD
   let count = items.len();
   println!("Found {count} items");

   // GOOD
   println!("Found {count} items", count = items.len());
   ```

## Error Handling

Always use `anyhow` for error handling throughout the codebase.

### Import Pattern
Always import anyhow's Result type to use as the default Result:
```rust
use anyhow::{anyhow, bail, Context, Result};

// Then use Result<T> in function signatures
pub fn some_function() -> Result<String> {
    // ...
}
```

### Correct Usage:
```rust
// For conditional checks
ensure!(condition, "Error message with {value}");

// For early returns with errors
bail!("Failed with error: {error_message}");

// Instead of return Err(anyhow::anyhow!(...))
bail!("Error message"); // Cleaner and more idiomatic

// For adding context to errors
let result = some_operation
    .with_context(|| format!("Failed to parse '{value}' as u32"))?;

// Prefer .context() or .with_context() over .map_err() when adding error context
// GOOD - Use context methods for adding descriptive error information
let data = parse_data(input)
    .context("Failed to parse configuration data")?;

let value = string.parse::<u32>()
    .with_context(|| format!("Invalid number format: '{string}'"))?;

// LESS PREFERRED - Using .map_err() for simple context addition
let data = parse_data(input)
    .map_err(|e| anyhow!("Failed to parse configuration data: {e}"))?;
```

### When to Use Each Error Handling Approach:

- **`.context("message")`**: Use for static error messages that don't need formatting
- **`.with_context(|| format!("..."))`**: Use when you need dynamic error messages with variables
- **`.map_err(|e| anyhow!("..."))`**: Use only when you need to transform the error type or preserve the original error details
- **`bail!("...")`**: Use for early returns with custom error messages
- **`ensure!(condition, "...")`**: Use for validation checks that should fail with specific messages

**Note**: Some external crate error types may not implement the traits required for `.context()`. In these cases, continue using `.map_err(|e| anyhow!("..."))` as needed.

### Incorrect Usage:
```rust
// BAD - Will crash on None:
let result = optional_value.unwrap();

// BAD - Will crash on Err:
let data = fallible_operation().unwrap();

// BAD - Explicit panic:
panic!("This failed");

// BAD - Silently ignores errors:
std::fs::remove_file(path).ok();

// BETTER - Log the error but continue:
if let Err(e) = std::fs::remove_file(path) {
    debug!("Failed to remove file: {e}");
}
```

## Bitcoin and Lightning Development

### Using rust-bitcoin Library
When working with Bitcoin functionality, prefer using rust-bitcoin's types and methods:

```rust
// GOOD - Use rust-bitcoin types for type safety
use bitcoin::{Amount, Weight};
use bitcoin::transaction::{predict_weight, InputWeightPrediction};

// Calculate fees with proper types
let fee_amount = Amount::from_sat(fee_sats);
let fee_btc = fee_amount.to_btc();

// Use predict_weight for transaction size estimation
let weight = predict_weight(inputs, outputs);
```

```rust
// BAD - Manual calculations that rust-bitcoin already provides
let fee_btc = fee_sats as f64 / 100_000_000.0;  // Avoid manual conversions
let tx_size = base + inputs * 68 + outputs * 31; // Use predict_weight instead
```

## Code Quality Standards

### Before Committing
Always run the complete check sequence:
```bash
cargo clippy --fix --allow-dirty && cargo fmt && cargo test && cargo clippy && cargo fmt --check
```

### Required Quality Checks
1. **All tests must pass**: `cargo test`
2. **No clippy warnings**: `cargo clippy`
3. **Code must be formatted**: `cargo fmt`

### Running Tests
- Run all tests: `cargo test`
- Run with output: `cargo test -- --nocapture`
- Run specific test: `cargo test test_name`
- Run tests in a module: `cargo test module_name`

## CLI-Specific Conventions

### Output Format
- **JSON by default**: All command outputs should be valid JSON for easy parsing
- **Clean output**: No debug prints or progress messages to stdout (use stderr if needed)
- **Consistent structure**: Similar commands should have similar output structures

### Command Line Arguments
- Use descriptive names for arguments and options
- Provide sensible defaults where appropriate
- Use `--output` or `-o` for file output consistently
- Group related commands using subcommands

### Error Messages
- Write user-friendly error messages
- Include actionable information (what went wrong, how to fix it)
- Use consistent error formatting
- Return appropriate exit codes (0 for success, non-zero for errors)

### CLI Patterns
- Accept input from file argument or stdin when not provided
- Write output to file with `-o/--output` or stdout by default
- Use descriptive help text for all arguments
- Provide sensible defaults where appropriate

## Development Process

### Before Implementing New Features
1. **Review similar code**: Look at how similar functionality is implemented elsewhere
2. **Use existing utilities**: Check if helper functions or utilities already exist
3. **Follow module patterns**: Maintain consistency with the module's existing structure
4. **Reuse types**: Use existing structs and enums where applicable

## Test Error Handling

### Test Function Standards

All test functions should use proper error handling patterns:

```rust
#[test]
fn test_example() -> Result<()> {
    // Use ? operator instead of .unwrap()
    let data = parse_data(input)?;
    let json = serde_json::to_string(&data)?;
    
    // For testing error conditions, use pattern matching
    let result = fallible_operation();
    assert!(result.is_err());
    if let Err(e) = result {
        assert!(e.to_string().contains("expected error message"));
    }
    
    // Always end with Ok(())
    Ok(())
}
```

### Test Error Handling Rules

1. **Return `Result<()>`**: All test functions should return `Result<()>` to enable proper error propagation
2. **Use `?` operator**: Replace `.unwrap()` calls with the `?` operator for cleaner error handling
3. **End with `Ok(())`**: Always end test functions with `Ok(())` to satisfy the return type
4. **Error testing**: When testing error conditions, use pattern matching instead of `.unwrap_err()`
5. **File operations**: Use `?` operator for file I/O operations in tests
6. **JSON operations**: Use `?` operator for JSON serialization/deserialization in tests

### Incorrect Test Patterns

```rust
// BAD - Will panic on error
#[test]
fn bad_test() {
    let data = parse_data(input).unwrap(); // BAD
    let json = serde_json::to_string(&data).unwrap(); // BAD
    
    let result = fallible_operation();
    assert!(result.unwrap_err().to_string().contains("error")); // BAD
}

// BAD - Missing return type
#[test]
fn another_bad_test() -> Result<()> {
    let data = parse_data(input)?;
    assert_eq!(data.len(), 5);
    // Missing Ok(()) - will not compile
}
```

### Development Checklist
Before submitting code:
- [ ] Review existing similar code to understand patterns
- [ ] Follow established conventions in this document
- [ ] Add appropriate error handling using `anyhow`
- [ ] Add unit tests for new functionality with proper error handling
- [ ] Ensure tests return `Result<()>` and use `?` operator instead of `.unwrap()`
- [ ] Run quality checks: `cargo test && cargo clippy && cargo fmt --check`
- [ ] Update relevant documentation (README.md for new commands)
