# Pullpiri Coding Specification
Our Pullpiri is developed using the Rust Programming language for the language semantics and rust usage the developers shall refer and use the standard rust documentation publicly available

## Folder Names
Folder names should be written in lowercase with hyphens (-).
- `src`: Source code folder
- `tests`: Test code folder
- `examples`: Example code folder
- `docs`: Documentation folder

## File Names
File names should be written in lowercase with underscores (_).
- `main.rs`: Main file
- `lib.rs`: Library file
- `mod_name.rs`: Module file

## Variable Names
Variable names should be written in lowercase with underscores (_).
- `user_name`
- `total_count`
- `is_valid`

## Function Names
Function names should be written in lowercase with underscores (_).
- `calculate_sum`
- `fetch_data`
- `is_valid_user`

## Coding Rules
1. **Consistency**: Maintain a consistent code style.
2. **Clear Names**: Use clear names for variables, functions, and modules that reflect their roles.
3. **Comments**: The code can be self-annotated, and the documentation should be concise.
   
   a. Write comments where necessary to explain the intent of the code.
   
   b. File header comments to include a copyright notice
   
   c. Use FIXME and TODO in comments to help with task collaboration
   
   d. normal comments use // or /* ... */, and
   
   e. document comments use ///, //! or /** ... **/
4. **Modularization**: Modularize functionality to improve readability and reusability.
5. **Error Handling**: Handle errors thoroughly to enhance stability.

    a. recoverable - use Result<T, E>
   
    b. unrecoverable errors - use panic!
   
6. **Write Tests**: Write test code to ensure the reliability of the code.
7. **Use Standard Library**: Use the Rust standard library whenever possible to improve efficiency.
8. **Add License information and dependencies to Cargo.toml conventions**: This helps in build and performing static check 

    The license field must contain a valid SPDX expression, using valid SPDX license names. (As an exception, by widespread convention, the license field may use / in place of OR; for example, MIT/Apache-2.0.)

## Example Code

```rust
// src/main.rs
fn main() {
    let user_name = "Alice";
    let total_count = calculate_sum(5, 10);
    println!("Hello, {}! The total count is {}.", user_name, total_count);
}

fn calculate_sum(a: i32, b: i32) -> i32 {
    a + b
}

// src/lib.rs
pub fn fetch_data() -> String {
    String::from("Sample data")
}

// src/tests/mod_name.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_sum() {
        assert_eq!(calculate_sum(2, 3), 5);
    }

    #[test]
    fn test_fetch_data() {
        assert_eq!(fetch_data(), "Sample data");
    }
}
