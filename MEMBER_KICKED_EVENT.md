# MemberKicked Event Implementation

## Overview
Added a `MemberKicked` event that is emitted whenever a member is forcibly removed from a savings circle. This enables the frontend to detect and react to membership changes in real-time.

## Implementation Details

### Event Structure
```rust
#[contracttype]
#[derive(Clone, Debug)]
pub struct MemberKickedEvent {
    pub circle_id: u64,
    pub member_address: Address,
    pub reason: String,
}
```

### Event Symbol
- **Event Name**: `MEM_KICK`
- **Topics**: `(symbol_short!("MEM_KICK"), circle_id)`
- **Data**: `MemberKickedEvent` struct containing circle_id, member_address, and reason

### Function Signature
```rust
fn kick_member(
    env: Env, 
    admin: Address, 
    member: Address, 
    circle_id: u64, 
    reason: String
) -> Result<(), Error>
```

### Authorization
- Only the contract admin can kick members
- Requires admin authentication via `admin.require_auth()`
- Returns `Error::Unauthorized` if caller is not the admin

### Error Handling
- `Error::Unauthorized` - Caller is not the admin
- `Error::MemberNotFound` - Member does not exist in the circle

## Frontend Integration

### Listening for Events
Frontend applications should listen for events with the symbol `MEM_KICK`:

```javascript
// Example: Listening for MemberKicked events
const events = await contract.getEvents({
  filters: [{
    type: "contract",
    contractIds: [contractId],
    topics: [["MEM_KICK", "*"]]
  }]
});

// Process events
events.forEach(event => {
  const { circle_id, member_address, reason } = event.data;
  // Update UI to reflect member removal
  updateMembershipList(circle_id, member_address, reason);
});
```

### Event Payload
The event contains:
- `circle_id`: The ID of the circle from which the member was removed
- `member_address`: The address of the removed member
- `reason`: A string explaining why the member was kicked

### Use Cases
1. **Real-time UI Updates**: Automatically update membership lists when a member is kicked
2. **Audit Trail**: Track all forced removals with reasons for compliance
3. **Notifications**: Alert users when they've been removed from a circle
4. **Analytics**: Monitor circle health and admin actions

## Test Coverage

All tests pass successfully:

### Test Cases
1. ✅ `test_kick_member_emits_event` - Verifies event is emitted with correct data
2. ✅ `test_kick_member_with_reason` - Tests with a descriptive reason string
3. ✅ `test_kick_member_empty_reason` - Handles empty reason strings gracefully
4. ✅ `test_kick_member_unauthorized` - Prevents non-admin from kicking members
5. ✅ `test_kick_member_not_found` - Returns error when member doesn't exist
6. ✅ `test_kick_member_updates_member_count` - Verifies member count is decremented

### Running Tests
```bash
cargo test --lib test_kick_member
```

## Security Considerations

1. **Admin-Only Access**: Only the contract admin can kick members, preventing abuse
2. **Authentication Required**: Uses Soroban's `require_auth()` for secure authorization
3. **Reason Tracking**: All kicks must include a reason for transparency
4. **Member Verification**: Checks member existence before removal

## State Changes

When a member is kicked:
1. Member data is removed from storage (`DataKey::Member`)
2. Circle's `member_count` is decremented
3. `MemberKicked` event is emitted with full context

## Example Usage

```rust
// Admin kicks a member for missing payments
let reason = String::from_str(&env, "Missed 3 consecutive payments");
client.kick_member(
    &admin_address,
    &member_address,
    &circle_id,
    &reason
)?;
```

## Commit
```
feat(events): emit MemberKicked event on forced member removal
```
