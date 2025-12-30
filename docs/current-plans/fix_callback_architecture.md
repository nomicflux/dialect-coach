# Fix Callback Architecture Violations

## Principle

**Actions carry INPUT. Reducer handles STATE. Callbacks do NEITHER.**

Any callback that captures `Rc<UserState>` and uses it for action data violates this principle. The captured state can become stale, causing one dialect's settings to affect another.

## Violations Found

### 1. `on_add_goal` (callbacks.rs:144-155)

```rust
pub fn on_add_goal(user: Rc<UserState>, dispatch: Callback<UserDomainAction>) -> Callback<String> {
    Callback::from(move |goal: String| {
        let learning_goal = LearningGoal {
            goal,
            dialect: user.selected_dialect,  // STALE STATE
        };
        dispatch.emit(UserDomainAction::Learning(LearningAction::AddGoal(
            learning_goal,
        )));
    })
}
```

**Bug scenario:** User is on Spanish → callback captures Spanish dialect → user switches to Japanese → adds goal → goal associated with Spanish.

### 2. `on_add_goal` inline (main_content.rs:190-202)

```rust
let on_add_goal = use_callback(
    (us.clone(), dispatch_domain.clone()),
    |goal: String, (us, dispatch)| {
        dispatch.emit(UserDomainAction::Learning(LearningAction::AddGoal(
            LearningGoal {
                goal,
                dialect: us.selected_dialect,  // STALE STATE
            },
        )));
    },
);
```

Same bug pattern.

### 3. `on_dialect_change` (callbacks.rs:34-50)

```rust
pub fn on_dialect_change(
    user: Rc<UserState>,
    dispatch: Callback<UserDomainAction>,
) -> Callback<Event> {
    Callback::from(move |e: Event| {
        let dialects = user.current_dialects();  // STALE STATE
        if let Some(dialect_features) = dialects.iter().find(|df| df.dialect.id() == value) {
            dispatch.emit(SettingsAction::ChangeDialect(dialect_features.dialect));
        }
    })
}
```

**Bug scenario:** Uses stale `current_dialects()` to parse dropdown value. If language changed, dialects list is wrong.

### 4. `on_dialect_cycle` (callbacks.rs:118-125)

```rust
pub fn on_dialect_cycle(
    _user: Rc<UserState>,  // UNUSED but still passed
    dispatch: Callback<UserDomainAction>,
) -> Callback<()>
```

Dead parameter. Should be removed.

---

## Phase 1: Fix AddGoal Action Architecture

### Subagent: kiss-code-generator

### Code Style Checklist
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No dead code
- [ ] No defensive coding
- [ ] Scope control - only fix AddGoal

### Changes Required

**1. `frontend/src/app/app_state/user/actions.rs`**

Change:
```rust
AddGoal(LearningGoal),
```

To:
```rust
AddGoal(String),  // Just the goal text - reducer handles dialect
```

Remove `LearningGoal` from imports (line 6).

**2. `frontend/src/app/app_state/user/reducer.rs`**

Change (line 84-85):
```rust
AddGoal(goal) => {
    next.learning_goals = Arc::new(add_learning_goal((*next.learning_goals).clone(), goal));
}
```

To:
```rust
AddGoal(goal_text) => {
    let goal = LearningGoal {
        goal: goal_text,
        dialect: next.selected_dialect,
    };
    next.learning_goals = Arc::new(add_learning_goal((*next.learning_goals).clone(), goal));
}
```

Add `LearningGoal` to imports.

**3. `frontend/src/app/user_state/callbacks.rs`**

Change (lines 144-155):
```rust
pub fn on_add_goal(user: Rc<UserState>, dispatch: Callback<UserDomainAction>) -> Callback<String> {
    Callback::from(move |goal: String| {
        let learning_goal = LearningGoal { goal, dialect: user.selected_dialect };
        dispatch.emit(UserDomainAction::Learning(LearningAction::AddGoal(learning_goal)));
    })
}
```

To:
```rust
pub fn on_add_goal(dispatch: Callback<UserDomainAction>) -> Callback<String> {
    Callback::from(move |goal: String| {
        dispatch.emit(UserDomainAction::Learning(LearningAction::AddGoal(goal)));
    })
}
```

Remove `LearningGoal` from imports (line 7).

**4. `frontend/src/components/main_content.rs`**

Change (lines 190-202):
```rust
let on_add_goal = use_callback(
    (us.clone(), dispatch_domain.clone()),
    |goal: String, (us, dispatch)| {
        if !goal.trim().is_empty() {
            dispatch.emit(UserDomainAction::Learning(LearningAction::AddGoal(
                LearningGoal { goal, dialect: us.selected_dialect },
            )));
        }
    },
);
```

To:
```rust
let on_add_goal = use_callback(dispatch_domain.clone(), |goal: String, dispatch| {
    if !goal.trim().is_empty() {
        dispatch.emit(UserDomainAction::Learning(LearningAction::AddGoal(goal)));
    }
});
```

Remove `LearningGoal` from imports.

### Deliverables
- `AddGoal` action contains only `String`
- Reducer creates `LearningGoal` with `next.selected_dialect`
- No callback captures `UserState` for goal creation
- Cross-dialect goal contamination impossible by construction

### Phase End Checklist
- [ ] `cargo check` passes
- [ ] `cargo test` passes
- [ ] `cargo clippy` clean
- [ ] `cd frontend && trunk build` compiles
- [ ] Git commit: `Phase 1 (fix AddGoal architecture) complete`

---

## Phase 2: Fix ChangeDialect Callback

### Subagent: kiss-code-generator

### Code Style Checklist
- [ ] Functions <20 lines
- [ ] Pure functions where possible
- [ ] No dead code
- [ ] No defensive coding
- [ ] Scope control - only fix ChangeDialect

### Changes Required

**1. `frontend/src/app/user_state/callbacks.rs`**

Change (lines 34-50):
```rust
pub fn on_dialect_change(
    user: Rc<UserState>,
    dispatch: Callback<UserDomainAction>,
) -> Callback<Event> {
    Callback::from(move |e: Event| {
        if let Some(select) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
            let value = select.value();
            let dialects = user.current_dialects();
            if let Some(dialect_features) = dialects.iter().find(|df| df.dialect.id() == value) {
                dispatch.emit(UserDomainAction::Settings(SettingsAction::ChangeDialect(
                    dialect_features.dialect,
                )));
            }
        }
    })
}
```

To:
```rust
pub fn on_dialect_change(dispatch: Callback<UserDomainAction>) -> Callback<Event> {
    Callback::from(move |e: Event| {
        if let Some(select) = e.target_dyn_into::<web_sys::HtmlSelectElement>() {
            if let Some(dialect) = Dialect::from_id(&select.value()) {
                dispatch.emit(UserDomainAction::Settings(SettingsAction::ChangeDialect(
                    dialect,
                )));
            }
        }
    })
}
```

Add `Dialect` to imports from `dialect_coach_shared::models`.

**2. `frontend/src/components/utility_sidebar/settings.rs`**

Change (line 84):
```rust
onchange={on_dialect_change(user.clone(), dispatch.clone())}
```

To:
```rust
onchange={on_dialect_change(dispatch.clone())}
```

### Deliverables
- `on_dialect_change` does not capture `UserState`
- Uses `Dialect::from_id()` for direct parsing
- Stale dialect list impossible by construction

### Phase End Checklist
- [ ] `cargo check` passes
- [ ] `cargo test` passes
- [ ] `cargo clippy` clean
- [ ] `cd frontend && trunk build` compiles
- [ ] Git commit: `Phase 2 (fix ChangeDialect callback) complete`

---

## Phase 3: Remove Dead Parameter from on_dialect_cycle

### Subagent: kiss-code-generator

### Code Style Checklist
- [ ] Functions <20 lines
- [ ] No dead code

### Changes Required

**1. `frontend/src/app/user_state/callbacks.rs`**

Change (lines 118-125):
```rust
pub fn on_dialect_cycle(
    _user: Rc<UserState>,
    dispatch: Callback<UserDomainAction>,
) -> Callback<()>
```

To:
```rust
pub fn on_dialect_cycle(dispatch: Callback<UserDomainAction>) -> Callback<()>
```

**2. Find and update all callers** (if any use this function directly)

### Deliverables
- Dead `_user` parameter removed
- No unused imports

### Phase End Checklist
- [ ] `cargo check` passes
- [ ] `cargo test` passes
- [ ] `cargo clippy` clean
- [ ] Git commit: `Phase 3 (remove dead parameter) complete`

---

## Status

| Phase | Description | Status |
|-------|-------------|--------|
| 1 | Fix AddGoal architecture | **Complete** |
| 2 | Fix ChangeDialect callback | **Complete** |
| 3 | Remove dead parameter | Pending |
