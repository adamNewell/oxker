# Test Reorganization Mapping

## Current State Analysis

### oxker-core Tests
**Integration Tests (tests/)**
- `event_system_integration.rs` - Tests EventBus communication patterns

**Unit Tests (src/ with #[cfg(test)])**
- `app_state.rs` - AppState management tests
- `config.rs` - Configuration parsing and validation
- `docker_data.rs` - Docker data structure tests  
- `event_bus.rs` - EventBus unit tests
- `exec_interface.rs` - Mock implementations and interface tests
- `handler.rs` - CoreHandle functionality tests
- `message.rs` - Message serialization/deserialization tests

### oxker-tui Tests
**Integration Tests (tests/)**
- `event_handler_integration.rs` - UI event handling with CoreHandle
- `filter_component_test.rs` - Filter panel component testing
- `help_component_test.rs` - Help panel component testing

**Unit Tests (src/ with #[cfg(test)])**
- Various component-level tests in `ui/draw_blocks/`
- Panel-specific logic tests
- Widget behavior tests

## Reorganization Plan

### Phase 1: Create MockCoreHandle
**New Files:**
- `oxker-tui/src/test_utils/mock_core_handle.rs`
- Implement CoreHandle trait with mock behavior
- Support configurable responses for testing scenarios
- Track method calls for verification

### Phase 2: Reorganize oxker-core Tests
**Keep As-Is (Already UI Independent):**
- All existing unit tests in src/
- `tests/event_system_integration.rs`

**New Structure:**
```
oxker-core/
├── src/
│   └── [modules with #[cfg(test)] blocks] (unit tests)
└── tests/
    ├── integration/
    │   ├── event_system.rs (moved from event_system_integration.rs)
    │   └── core_handle.rs (new - test public API)
    └── lib.rs
```

### Phase 3: Refactor oxker-tui Tests
**Current Tests to Update:**
1. `event_handler_integration.rs`
   - Replace real CoreHandle with MockCoreHandle
   - Remove dependency on real Docker events
   - Focus on UI event handling logic

2. `filter_component_test.rs` & `help_component_test.rs`
   - Already use TestBackend (good)
   - Add MockCoreHandle for any CoreHandle interactions
   - Ensure no real Docker calls

**New Structure:**
```
oxker-tui/
├── src/
│   ├── [components with #[cfg(test)] blocks] (unit tests)
│   └── test_utils/
│       ├── mod.rs
│       ├── mock_core_handle.rs (new)
│       └── test_setup.rs (existing helpers)
└── tests/
    ├── integration/
    │   ├── event_handling.rs (refactored from event_handler_integration.rs)
    │   ├── component_rendering.rs (combined component tests)
    │   └── ui_flow.rs (new - test UI workflows)
    └── lib.rs
```

### Phase 4: Clear Unit/Integration Separation
**Unit Test Criteria:**
- Test single module/component in isolation
- Use mocks for all external dependencies
- Fast execution (< 100ms per test)
- Located in src/ with #[cfg(test)]

**Integration Test Criteria:**
- Test interaction between modules
- May use real implementations (except Docker)
- Located in tests/ directory
- Test public APIs and contracts

### Phase 5: Preserve Test Assertions
**Migration Strategy:**
1. Copy existing test to new location
2. Update imports and mock usage
3. Run both old and new tests
4. Verify identical behavior
5. Remove old test only after validation

**Assertion Preservation Checklist:**
- [ ] All test names preserved or clearly mapped
- [ ] All assertions maintain same logic
- [ ] Test coverage metrics remain same or improve
- [ ] No test functionality lost in migration

### Phase 6: Documentation Updates
**Files to Update:**
- `docs/architecture/9-testing-strategy.md` - Document new patterns
- `README.md` - Update test running instructions
- Create `TESTING.md` - Comprehensive testing guide

**New Documentation:**
- MockCoreHandle usage guide
- Test organization conventions
- CI/CD test configuration updates

## Implementation Order
1. Create MockCoreHandle implementation
2. Update oxker-tui tests to use mocks
3. Reorganize test directory structures
4. Update documentation
5. Validate all tests pass
6. Update CI/CD configuration if needed

## Success Criteria
- All existing tests pass with new organization
- Clear separation between unit and integration tests
- UI tests completely isolated from Docker operations
- Improved test maintainability and clarity