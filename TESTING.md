# Testing Guide for Oxker

This guide describes the testing strategy, organization, and best practices for the Oxker project.

## Test Organization

### Unit Tests
Unit tests are located within the source files using `#[cfg(test)]` modules. They test individual functions and modules in isolation.

**Location**: `src/**/*.rs` files  
**Run with**: `cargo test --lib`

### Integration Tests
Integration tests are located in the `tests/` directory. They test the interaction between modules and the public API.

**Location**: `tests/*.rs` files  
**Run with**: `cargo test --tests`

## Project Structure

```
oxker/
├── oxker-core/
│   ├── src/
│   │   └── [modules with #[cfg(test)] blocks] (unit tests)
│   └── tests/
│       └── event_system_integration.rs (integration tests)
└── oxker-tui/
    ├── src/
    │   ├── [modules with #[cfg(test)] blocks] (unit tests)
    │   └── test_utils/
    │       ├── mock_core_handle.rs (mock implementation)
    │       └── [test utilities]
    └── tests/
        ├── event_handler_integration.rs
        ├── filter_component_test.rs
        └── help_component_test.rs
```

## Test Categories

### oxker-core Tests

**Unit Tests (104 tests)**:
- App state management
- Container state handling
- Configuration parsing
- Docker data processing
- Event system
- Command execution
- Security sanitization

**Integration Tests (3 tests)**:
- Event system integration
- Command processing
- Multi-command scenarios

### oxker-tui Tests

**Unit Tests (30 tests)**:
- UI components
- Color conversion
- Panel rendering
- Widget behavior
- Log sanitization
- Mock CoreHandle functionality

**Integration Tests (5 tests)**:
- Event handler integration with MockCoreHandle
- Component rendering tests
- GUI state management

## Key Testing Patterns

### 1. MockCoreHandle
The `MockCoreHandle` in `oxker-tui/src/test_utils/mock_core_handle.rs` provides a test double for the `CoreHandle` trait, allowing UI tests to run without Docker dependencies.

**Usage**:
```rust
let (event_bus, _receiver) = EventBus::new(100);
let mock_handle = MockCoreHandle::new(event_bus);
```

### 2. UI Component Testing
UI components are tested using the `ratatui` `TestBackend`:
```rust
let backend = TestBackend::new(80, 20);
let mut terminal = Terminal::new(backend).unwrap();
```

### 3. Test Isolation
- **oxker-core**: Tests are completely independent of UI libraries (no ratatui/crossterm dependencies)
- **oxker-tui**: Tests use MockCoreHandle to avoid real Docker operations

## Running Tests

### Run all tests:
```bash
cargo test
```

### Run specific crate tests:
```bash
cargo test -p oxker-core
cargo test -p oxker-tui
```

### Run only unit tests:
```bash
cargo test --lib
```

### Run only integration tests:
```bash
cargo test --tests
```

### Run tests with output:
```bash
cargo test -- --nocapture
```

## CI/CD Integration

Tests should be run in CI with the following steps:

```yaml
# Example GitHub Actions workflow
- name: Run tests
  run: |
    cargo test --all --lib --tests
    cargo fmt -- --check
    cargo clippy -- -D warnings
```

## Best Practices

1. **Test Naming**: Use descriptive test names that explain what is being tested
   ```rust
   #[test]
   fn test_container_state_updates_when_stopped() { }
   ```

2. **Test Organization**: Keep tests close to the code they test
   - Unit tests in the same file as the implementation
   - Integration tests in the `tests/` directory

3. **Mock Usage**: Use MockCoreHandle for all UI tests that would otherwise require Docker

4. **Assertions**: Preserve all existing assertions when refactoring tests

5. **Test Independence**: Each test should be independent and not rely on external state

## Adding New Tests

### Adding a Unit Test
1. Add a `#[cfg(test)]` module to your source file if it doesn't exist
2. Write your test function with the `#[test]` attribute
3. Use appropriate assertions

### Adding an Integration Test
1. Create a new file in the `tests/` directory
2. Import the crate using `use oxker_core::*` or `use oxker_tui::*`
3. Write tests that verify public API behavior

### Adding Mock Functionality
1. Extend the `MockCoreHandle` implementation as needed
2. Add corresponding tests in `mock_core_handle_test.rs`
3. Update this documentation

## Test Coverage

While we don't enforce specific coverage metrics, aim to test:
- All public API functions
- Edge cases and error conditions
- Critical business logic
- UI component rendering

## Troubleshooting

### Common Issues

1. **Import errors in tests**: Ensure you're importing from the crate root for integration tests
2. **Async test failures**: Use `#[tokio::test]` for async tests
3. **Mock not working**: Verify MockCoreHandle is properly initialized with an EventBus

### Test Maintenance

- Review and update tests when modifying functionality
- Keep test assertions in sync with expected behavior
- Remove obsolete tests when features are removed
- Update this guide when test patterns change

## Future Improvements

1. **Snapshot Testing**: Consider adding snapshot tests for UI components using the `insta` crate
2. **Property Testing**: Add property-based tests for complex logic
3. **Benchmarks**: Expand benchmark tests for performance-critical paths
4. **Coverage Reports**: Set up test coverage reporting in CI