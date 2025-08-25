# Snapshot Testing Guide for Oxker TUI

This guide explains how to work with snapshot tests in the oxker-tui crate.

## Overview

We use [insta](https://insta.rs/) for snapshot testing to ensure UI components render consistently. This is critical for the refactoring effort as we must maintain exact visual compatibility.

## Running Snapshot Tests

```bash
# Run all tests including snapshots
cargo test --lib

# Run specific snapshot tests
cargo test draw_blocks::

# Run with output to see what's being tested
cargo test -- --show-output
```

## Reviewing Snapshot Changes

When tests fail due to snapshot differences:

```bash
# Review all new/changed snapshots interactively
cargo insta review

# Accept all new snapshots (use with caution)
cargo insta accept

# Reject all changes
cargo insta reject
```

## Adding New Snapshot Tests

1. Use the test helpers in `src/ui/draw_blocks/mod.rs`:

```rust
#[test]
fn test_my_component() {
    let mut setup = test_setup(80, 10, true, true);
    
    setup.terminal.draw(|f| {
        my_component::draw(setup.area, colors, f, &view_model);
    }).unwrap();
    
    assert_snapshot!(setup.terminal.backend());
}
```

2. Run the test to generate the initial snapshot
3. Review and commit the `.snap` file

## Snapshot File Organization

- Snapshots are stored in `src/ui/draw_blocks/snapshots/`
- Files are named: `{module}__{test_function}.snap`
- Keep snapshots close to their test files

## Best Practices

1. **Granular Tests**: Test individual components rather than entire screens when possible
2. **Deterministic Data**: Use consistent test data to avoid flaky snapshots
3. **Clear Names**: Use descriptive test names that explain what's being tested
4. **Review Carefully**: Always review snapshot changes to ensure they're intentional
5. **Commit Separately**: Commit snapshot updates in separate commits from code changes

## Troubleshooting

### Tests Failing After UI Changes

1. Run `cargo test` to see failures
2. Use `cargo insta review` to inspect differences
3. Accept changes only if they're intentional improvements
4. If changes are unintended, fix the code to match original output

### New Snapshots Not Being Created

1. Ensure test is actually running: `cargo test {test_name} -- --nocapture`
2. Check that `assert_snapshot!` is being called
3. Look for `.snap.new` files after test runs

### Platform Differences

Snapshots should be consistent across platforms. If you see differences:
1. Check for platform-specific code
2. Ensure terminal size is consistent in tests
3. Use fixed test data rather than system-dependent values

## Configuration

The `insta` configuration is in `.config/insta.yaml`:

```yaml
snapshot_path: snapshots
require_full_match: true
```

This ensures snapshots are stored in the standard location and must match exactly.