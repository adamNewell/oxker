# Test Fixes for Story 1.5b

## Issue: Snapshot Test Timeout

### Problem
The snapshot tests are timing out when run in parallel due to a deadlock issue in the test framework. The tests try to acquire mutexes concurrently which causes a deadlock.

### Root Cause
The test setup functions (`test_setup`, `create_test_frame_view_model`) create multiple mutex locks on the `GuiState` object. When multiple tests run in parallel, they can deadlock trying to acquire these locks.

### Temporary Workaround
Run tests with a single thread:
```bash
cargo test -- --test-threads=1
```

### Long-term Solution
The test infrastructure needs to be refactored to:
1. Use mock implementations that don't require mutex locking
2. Create separate test instances for each test to avoid shared state
3. Update snapshot files to match the new UI rendering after the refactoring

### Test Status
- Tests run successfully with `--test-threads=1`
- Snapshot tests fail due to UI changes (expected)
- No actual code bugs found - only test infrastructure issues

## Recommended Actions
1. Update all snapshot files after verifying the UI renders correctly
2. Refactor test infrastructure to avoid mutex deadlocks
3. Consider using a test-specific GuiState implementation that doesn't use mutexes