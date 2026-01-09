# REQ-GOV-003: Approval Integration

| Metadata | Value |
|----------|-------|
| **ID** | `REQ-GOV-003` |
| **Title** | Approval Integration |
| **Type** | Governance Component |
| **Status** | Draft |
| **Priority** | **High** |
| **Tags** | `#governance` `#approval` `#polling` `#slack` `#integration` |

## 1. Context & Decision Rationale

This requirement defines how ThoughtGate integrates with external approval systems.

### 1.1 The Sidecar Networking Challenge

ThoughtGate runs as a **sidecar** inside a Kubernetes pod. This creates a fundamental networking constraint:

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                    THE CALLBACK PROBLEM                                         │
│                                                                                 │
│   ❌ CALLBACK MODEL (Doesn't work for sidecars)                                │
│                                                                                 │
│   Pod 1: 10.0.0.1 ──webhook──▶ Slack ──callback──▶ ??? (which pod?)            │
│   Pod 2: 10.0.0.2 ──webhook──▶ Slack ──callback──▶ ???                         │
│   Pod 3: 10.0.0.3 ──webhook──▶ Slack ──callback──▶ ???                         │
│                                                                                 │
│   Problem: Slack has no way to route callback to correct sidecar               │
│   Sidecars are not individually addressable from external systems              │
│                                                                                 │
├─────────────────────────────────────────────────────────────────────────────────┤
│                                                                                 │
│   ✅ POLLING MODEL (Sidecar-compatible)                                        │
│                                                                                 │
│   Pod 1: 10.0.0.1 ──post message──▶ Slack ◀──poll for reactions── Pod 1       │
│   Pod 2: 10.0.0.2 ──post message──▶ Slack ◀──poll for reactions── Pod 2       │
│   Pod 3: 10.0.0.3 ──post message──▶ Slack ◀──poll for reactions── Pod 3       │
│                                                                                 │
│   Solution: Each sidecar polls for its own task's approval decision           │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

### 1.2 Design Philosophy

- ThoughtGate posts approval requests to Slack (outbound only)
- ThoughtGate polls Slack API for decisions (no inbound callbacks)
- Each sidecar is self-sufficient (no central coordinator needed)
- Adapters encapsulate polling logic for different systems

**Future Extensibility:**
This architecture supports A2A (Agent-to-Agent) approval in future versions—an approval agent monitors a channel and responds via reactions or replies.

## 2. Dependencies

| Requirement | Relationship | Notes |
|-------------|--------------|-------|
| REQ-GOV-001 | **Updates** | Task state on approval decision |
| REQ-GOV-002 | **Triggers** | Execution pipeline on approval |
| REQ-CORE-004 | **Uses** | Error responses for failures |
| REQ-CORE-005 | **Coordinates** | Shutdown stops polling tasks |

## 3. Intent

The system must:
1. Post approval requests to external systems (Slack)
2. Poll external systems for approval decisions
3. Detect approval via reactions, button clicks, or replies
4. Update task state when decision is detected
5. Provide a Slack adapter as reference implementation
6. Support polling multiple tasks concurrently

## 4. Scope

### 4.1 In Scope
- Outbound message posting to Slack
- Polling Slack API for decisions
- Reaction-based approval detection (👍 = approve, 👎 = reject)
- Reply-based approval detection ("approved", "rejected")
- Polling backoff and rate limiting
- Adapter interface for extensibility
- Approval metrics and logging

### 4.2 Out of Scope
- Inbound webhook/callback endpoints (not needed for polling model)
- Building custom approval UIs
- Teams adapter (future version)
- PagerDuty adapter (future version)
- A2A approval adapter (future version)
- Approval batching (future version)

## 5. Constraints

### 5.1 Configuration

| Setting | Default | Environment Variable |
|---------|---------|---------------------|
| Poll interval (base) | 5s | `THOUGHTGATE_APPROVAL_POLL_INTERVAL_SECS` |
| Poll interval (max) | 30s | `THOUGHTGATE_APPROVAL_POLL_MAX_INTERVAL_SECS` |
| Slack API rate limit | 1/sec | `THOUGHTGATE_SLACK_RATE_LIMIT_PER_SEC` |
| Max concurrent polls | 100 | `THOUGHTGATE_MAX_CONCURRENT_POLLS` |

### 5.2 Slack Configuration

| Setting | Default | Environment Variable |
|---------|---------|---------------------|
| Bot token | (required) | `SLACK_BOT_TOKEN` |
| Default channel | `#approvals` | `SLACK_CHANNEL` |
| Approve reaction | `+1` (👍) | `SLACK_APPROVE_REACTION` |
| Reject reaction | `-1` (👎) | `SLACK_REJECT_REACTION` |

**Note:** Unlike the callback model, polling requires a **Bot Token** (not just a webhook URL) to read messages and reactions via Slack API.

### 5.3 Rate Limiting (CRITICAL)

**⚠️ Slack API Protection**

Slack API has strict rate limits (~1 request/second for most endpoints). With many pending approvals, polling can quickly exhaust limits.

**Rate Limiting Strategy:**
| Component | Limit | Behavior When Exceeded |
|-----------|-------|------------------------|
| API calls | 1 req/sec (tier 3) | Queue and batch |
| Concurrent polls | 100 tasks | Oldest tasks polled first |
| Backoff | Exponential | 5s → 10s → 20s → 30s max |

**⚠️ Batch Polling Efficiency (IMPORTANT)**

**Naive approach (O(n) API calls):**
```
For each pending task:
    Call reactions.get(channel, ts)  // One API call per task!
```
With 20 pending tasks, this makes 20 API calls per poll cycle → hits rate limit immediately.

**Efficient approach (O(1) API calls):**
```
Call conversations.history(channel, limit=100)  // One API call for ALL tasks
Parse response to check reactions on all pending messages
```

**Recommended Implementation:**
```rust
impl SlackAdapter {
    /// Efficient batch poll using conversations.history
    /// Returns decisions for ALL pending tasks in the channel with ONE API call
    async fn batch_poll_channel(
        &self,
        channel: &str,
        pending_tasks: &[ApprovalReference],
    ) -> Result<HashMap<TaskId, Option<ApprovalDecision>>, AdapterError> {
        // 1. Fetch recent messages from channel (single API call)
        let history = self.client
            .conversations_history(channel)
            .limit(100)  // Covers most pending approvals
            .await?;
        
        // 2. Build lookup by message timestamp
        let messages: HashMap<&str, &Message> = history.messages
            .iter()
            .map(|m| (m.ts.as_str(), m))
            .collect();
        
        // 3. Check each pending task against fetched messages
        let mut results = HashMap::new();
        for task_ref in pending_tasks {
            if let Some(msg) = messages.get(task_ref.external_id.as_str()) {
                // Check reactions on this message
                let decision = self.check_reactions(&msg.reactions);
                results.insert(task_ref.task_id.clone(), decision);
            } else {
                // Message not in recent history (old or deleted)
                results.insert(task_ref.task_id.clone(), None);
            }
        }
        
        Ok(results)
    }
}
```

**Complexity Comparison:**
| Approach | API Calls per Cycle | With 20 Tasks | With 100 Tasks |
|----------|---------------------|---------------|----------------|
| Per-task `reactions.get` | O(n) | 20 calls | 100 calls |
| Batch `conversations.history` | O(1) | 1 call | 1 call |

**Implementation:**
```rust
pub struct PollingScheduler {
    rate_limiter: RateLimiter,          // Token bucket, 1/sec
    pending_tasks: BTreeMap<Instant, TaskId>,  // Ordered by next poll time
    max_concurrent: usize,
}
```

### 5.4 Security Requirements

- Bot token MUST NOT be logged
- Slack API calls MUST use HTTPS
- User identity from Slack is trusted (Slack authenticates users)

## 6. Interfaces

### 6.1 Approval Message (ThoughtGate → Slack)

```
POST https://slack.com/api/chat.postMessage
Authorization: Bearer {bot_token}
Content-Type: application/json

{
  "channel": "#approvals",
  "blocks": [...],  // Block Kit message
  "metadata": {
    "event_type": "thoughtgate_approval",
    "event_payload": {
      "task_id": "550e8400-e29b-41d4-a716-446655440000"
    }
  }
}
```

**Response:**
```json
{
  "ok": true,
  "channel": "C1234567890",
  "ts": "1234567890.123456",
  "message": {...}
}
```

The `channel` and `ts` (timestamp) are stored to poll for reactions/replies.

### 6.2 Poll for Reactions (ThoughtGate ← Slack)

```
GET https://slack.com/api/reactions.get
  ?channel=C1234567890
  &timestamp=1234567890.123456
Authorization: Bearer {bot_token}
```

**Response:**
```json
{
  "ok": true,
  "message": {
    "reactions": [
      {"name": "+1", "users": ["U1234567"], "count": 1},
      {"name": "eyes", "users": ["U7654321"], "count": 1}
    ]
  }
}
```

### 6.3 Poll for Replies (Alternative)

```
GET https://slack.com/api/conversations.replies
  ?channel=C1234567890
  &ts=1234567890.123456
Authorization: Bearer {bot_token}
```

### 6.4 Adapter Interface

```rust
#[async_trait]
pub trait ApprovalAdapter: Send + Sync {
    /// Post approval request to external system
    /// Returns external reference for polling
    async fn post_approval_request(
        &self,
        request: &ApprovalRequest,
    ) -> Result<ApprovalReference, AdapterError>;
    
    /// Poll for decision on a pending approval
    /// Returns None if still pending
    async fn poll_for_decision(
        &self,
        reference: &ApprovalReference,
    ) -> Result<Option<ApprovalDecision>, AdapterError>;
    
    /// Cancel a pending approval (best-effort)
    async fn cancel_approval(
        &self,
        reference: &ApprovalReference,
    ) -> Result<(), AdapterError>;
    
    /// Adapter name for logging/metrics
    fn name(&self) -> &'static str;
}

pub struct ApprovalRequest {
    pub task_id: TaskId,
    pub tool_name: String,
    pub tool_arguments: serde_json::Value,
    pub principal: Principal,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub correlation_id: String,
}

/// Reference to posted approval message for polling
pub struct ApprovalReference {
    pub task_id: TaskId,
    pub external_id: String,      // e.g., Slack message_ts
    pub channel: String,          // e.g., Slack channel ID
    pub posted_at: DateTime<Utc>,
    pub next_poll_at: Instant,
    pub poll_count: u32,
}

pub struct ApprovalDecision {
    pub decision: Decision,
    pub decided_by: String,       // User who reacted/replied
    pub decided_at: DateTime<Utc>,
    pub method: DecisionMethod,
}

pub enum Decision {
    Approved,
    Rejected,
}

pub enum DecisionMethod {
    Reaction { emoji: String },
    Reply { text: String },
}
```

### 6.5 Errors

```rust
pub enum AdapterError {
    PostFailed { reason: String, retriable: bool },
    PollFailed { reason: String, retriable: bool },
    RateLimited { retry_after: Duration },
    InvalidToken,
    ChannelNotFound { channel: String },
    MessageNotFound { ts: String },
}
```

## 7. Functional Requirements

### F-001: Approval Request Posting

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                     APPROVAL REQUEST FLOW                                       │
│                                                                                 │
│   Task Created (InputRequired)                                                  │
│         │                                                                       │
│         ▼                                                                       │
│   ┌─────────────────────────────────────────────────────────────────────────┐  │
│   │  1. Build Slack Block Kit message                                       │  │
│   │  2. Rate limit check (wait if needed)                                   │  │
│   │  3. POST to chat.postMessage                                            │  │
│   │  4. Store channel + ts in ApprovalReference                             │  │
│   │  5. Schedule first poll (after poll_interval)                           │  │
│   └─────────────────────────────────────────────────────────────────────────┘  │
│         │                                                                       │
│         ▼                                                                       │
│   Polling Scheduled                                                             │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

- **F-001.1:** Build Block Kit message from task data
- **F-001.2:** Include task ID in message metadata
- **F-001.3:** Apply rate limiting before API call
- **F-001.4:** Store message reference (channel, ts) in task
- **F-001.5:** Schedule polling task
- **F-001.6:** Handle posting failures with retry

### F-002: Polling Scheduler

```
┌─────────────────────────────────────────────────────────────────────────────────┐
│                     POLLING SCHEDULER                                           │
│                                                                                 │
│   ┌─────────────────────────────────────────────────────────────────────────┐  │
│   │                    Polling Loop (background task)                       │  │
│   │                                                                         │  │
│   │   while !shutdown:                                                      │  │
│   │     1. Get next task due for polling (ordered by next_poll_at)         │  │
│   │     2. Wait until next_poll_at or new task arrives                     │  │
│   │     3. Rate limit check                                                 │  │
│   │     4. Poll adapter for decision                                        │  │
│   │     5. If decision found:                                               │  │
│   │        - Update task state                                              │  │
│   │        - Trigger execution pipeline (if approved)                       │  │
│   │        - Remove from polling queue                                      │  │
│   │     6. If still pending:                                                │  │
│   │        - Backoff: next_poll_at = now + interval * backoff_factor       │  │
│   │        - Re-queue for next poll                                         │  │
│   │     7. If task expired:                                                 │  │
│   │        - Remove from polling queue                                      │  │
│   │        - Task expiry handled by REQ-GOV-001                            │  │
│   │                                                                         │  │
│   └─────────────────────────────────────────────────────────────────────────┘  │
│                                                                                 │
└─────────────────────────────────────────────────────────────────────────────────┘
```

- **F-002.1:** Maintain priority queue of tasks by next poll time
- **F-002.2:** Respect rate limits across all polls
- **F-002.3:** Exponential backoff on repeated polls (5s → 10s → 20s → 30s)
- **F-002.4:** Remove expired tasks from queue
- **F-002.5:** Handle shutdown gracefully (drain queue)

### F-003: Decision Detection

- **F-003.1:** Check for approval reaction (👍 by default)
- **F-003.2:** Check for rejection reaction (👎 by default)
- **F-003.3:** If both reactions present, first one wins (by timestamp)
- **F-003.4:** Extract user ID of reactor for audit
- **F-003.5:** Look up user display name via Slack API (cached)

**Decision Priority:**
| Check Order | Signal | Decision |
|-------------|--------|----------|
| 1 | 👍 reaction | Approved |
| 2 | 👎 reaction | Rejected |
| 3 | Reply containing "approved" | Approved |
| 4 | Reply containing "rejected" | Rejected |

### F-004: Decision Processing

- **F-004.1:** Verify task is still in InputRequired state
- **F-004.2:** Build ApprovalRecord from decision
- **F-004.3:** On approval: update task, trigger execution pipeline
- **F-004.4:** On rejection: transition task to Rejected state
- **F-004.5:** Update Slack message to show decision (edit message)

### F-005: Approval Cancellation

- **F-005.1:** Remove task from polling queue on cancellation
- **F-005.2:** Optionally delete or update Slack message
- **F-005.3:** Best-effort (don't fail if Slack update fails)

### F-006: Slack Adapter Implementation

- **F-006.1:** Use Slack Web API (not incoming webhooks)
- **F-006.2:** Build Block Kit messages with clear formatting
- **F-006.3:** Include reaction instructions in message
- **F-006.4:** Cache user ID → display name mappings
- **F-006.5:** Handle Slack API errors gracefully

**Slack Message Format:**
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ 🔒 Approval Required: delete_user                                           │
├─────────────────────────────────────────────────────────────────────────────┤
│ *Tool:* `delete_user`                                                       │
│ *Principal:* production/agent-service                                       │
│                                                                             │
│ *Arguments:*                                                                │
│ ```                                                                         │
│ {                                                                           │
│   "user_id": "12345",                                                       │
│   "reason": "Account inactive"                                              │
│ }                                                                           │
│ ```                                                                         │
│                                                                             │
│ React with 👍 to *approve* or 👎 to *reject*                                │
│                                                                             │
│ Task ID: `abc-123` • Expires: 2025-01-08 11:30 UTC                         │
└─────────────────────────────────────────────────────────────────────────────┘
```

**After Approval:**
```
┌─────────────────────────────────────────────────────────────────────────────┐
│ ✅ Approved: delete_user                                                    │
├─────────────────────────────────────────────────────────────────────────────┤
│ *Approved by:* @alice                                                       │
│ *Approved at:* 2025-01-08 10:35 UTC                                        │
│                                                                             │
│ ... (original content) ...                                                  │
└─────────────────────────────────────────────────────────────────────────────┘
```

## 8. Non-Functional Requirements

### NFR-001: Observability

**Metrics:**
```
thoughtgate_approval_posts_total{adapter="slack", status="success|failed"}
thoughtgate_approval_polls_total{adapter="slack", result="pending|approved|rejected|expired|error"}
thoughtgate_approval_decisions_total{adapter="slack", decision="approved|rejected", method="reaction|reply"}
thoughtgate_approval_poll_latency_seconds{adapter="slack"}
thoughtgate_approval_decision_latency_seconds{adapter="slack"}  // Time from post to decision
thoughtgate_approval_polling_queue_size
```

**Logging:**
```json
{"level":"info","event":"approval_posted","task_id":"abc-123","adapter":"slack","channel":"C123","ts":"1234.5678"}
{"level":"debug","event":"approval_poll","task_id":"abc-123","poll_count":3,"result":"pending"}
{"level":"info","event":"approval_decided","task_id":"abc-123","decision":"approved","decided_by":"alice","method":"reaction"}
{"level":"warn","event":"poll_rate_limited","adapter":"slack","retry_after_ms":1000}
```

### NFR-002: Performance

| Metric | Target |
|--------|--------|
| Message post latency | < 500ms (P99) |
| Poll latency | < 200ms (P99) |
| Decision detection | Within 2 poll cycles of reaction |
| Memory per pending task | < 1KB |

### NFR-003: Reliability

- Polling continues across transient Slack API failures
- Exponential backoff prevents thundering herd
- Rate limiting prevents API exhaustion
- Graceful degradation if Slack is unavailable

### NFR-004: Security

- Bot token stored securely (K8s Secret)
- Bot token never logged
- User identity from Slack trusted
- HTTPS for all Slack API calls

## 9. Verification Plan

### 9.1 Edge Case Matrix

| Scenario | Expected Behavior | Test ID |
|----------|-------------------|---------|
| Message posted successfully | Reference stored, polling scheduled | EC-APR-001 |
| Message post fails (retriable) | Retry with backoff | EC-APR-002 |
| Message post fails (permanent) | Task marked failed | EC-APR-003 |
| Poll finds 👍 reaction | Task approved, execution triggered | EC-APR-004 |
| Poll finds 👎 reaction | Task rejected | EC-APR-005 |
| Poll finds both reactions | First reaction wins | EC-APR-006 |
| Poll finds no reactions | Re-queue with backoff | EC-APR-007 |
| Task expires while polling | Remove from queue | EC-APR-008 |
| Task cancelled while polling | Remove from queue, update message | EC-APR-009 |
| Slack rate limited | Backoff, retry later | EC-APR-010 |
| Slack API error | Log, retry with backoff | EC-APR-011 |
| Bot token invalid | Log error, mark adapter unhealthy | EC-APR-012 |
| Channel not found | Log error, fail task | EC-APR-013 |
| 100 concurrent approvals | All polled within rate limits | EC-APR-014 |
| Shutdown with pending polls | Graceful drain | EC-APR-015 |
| User reacts then unreacts | No decision (reaction removed) | EC-APR-016 |

### 9.2 Assertions

**Unit Tests:**
- `test_polling_scheduler_ordering` — Tasks polled in next_poll_at order
- `test_exponential_backoff` — Backoff increases correctly
- `test_rate_limiter` — API calls stay within limit
- `test_reaction_detection` — 👍/👎 correctly detected

**Integration Tests:**
- `test_full_approval_flow` — Post → Poll → Detect → Execute
- `test_rejection_flow` — Post → Poll → Detect → Reject
- `test_concurrent_approvals` — Many tasks polled correctly
- `test_slack_api_mock` — Adapter works with mock Slack

**Load Tests:**
- `test_100_concurrent_tasks` — System stays within rate limits
- `test_polling_under_load` — Latency stays acceptable

## 10. Implementation Reference

### Polling Scheduler

```rust
pub struct PollingScheduler {
    adapter: Arc<dyn ApprovalAdapter>,
    task_manager: Arc<dyn TaskManager>,
    execution_pipeline: Arc<ExecutionPipeline>,
    
    // Polling state
    pending: Mutex<BTreeMap<Instant, TaskId>>,
    references: DashMap<TaskId, ApprovalReference>,
    
    // Rate limiting
    rate_limiter: RateLimiter,
    
    // Config
    config: PollingConfig,
    
    // Shutdown
    shutdown: CancellationToken,
}

pub struct PollingConfig {
    pub base_interval: Duration,
    pub max_interval: Duration,
    pub max_concurrent: usize,
}

impl PollingScheduler {
    pub async fn run(&self) {
        loop {
            tokio::select! {
                _ = self.shutdown.cancelled() => break,
                _ = self.poll_next() => {}
            }
        }
    }
    
    async fn poll_next(&self) {
        // Get next task due for polling
        let (task_id, reference) = match self.get_next_due().await {
            Some(t) => t,
            None => {
                // No tasks pending, wait for new task or shutdown
                tokio::time::sleep(Duration::from_secs(1)).await;
                return;
            }
        };
        
        // Rate limit
        self.rate_limiter.acquire().await;
        
        // Poll for decision
        match self.adapter.poll_for_decision(&reference).await {
            Ok(Some(decision)) => {
                self.handle_decision(task_id, decision).await;
            }
            Ok(None) => {
                // Still pending, reschedule with backoff
                self.reschedule_with_backoff(task_id, reference).await;
            }
            Err(e) => {
                tracing::warn!(task_id = %task_id, error = %e, "Poll failed");
                self.reschedule_with_backoff(task_id, reference).await;
            }
        }
    }
    
    async fn handle_decision(&self, task_id: TaskId, decision: ApprovalDecision) {
        // Remove from polling queue
        self.references.remove(&task_id);
        
        // Build approval record
        let record = ApprovalRecord {
            decision: decision.decision,
            decided_by: decision.decided_by,
            decided_at: decision.decided_at,
            // ...
        };
        
        // Update task and trigger pipeline
        match decision.decision {
            Decision::Approved => {
                self.task_manager.approve(&task_id, record).await;
                self.execution_pipeline.execute(task_id).await;
            }
            Decision::Rejected => {
                self.task_manager.reject(&task_id, record).await;
            }
        }
    }
}
```

### Slack Adapter

```rust
pub struct SlackAdapter {
    client: reqwest::Client,
    bot_token: String,
    channel: String,
    approve_reaction: String,  // Default: "+1"
    reject_reaction: String,   // Default: "-1"
    user_cache: Cache<String, String>,  // user_id -> display_name
}

#[async_trait]
impl ApprovalAdapter for SlackAdapter {
    async fn post_approval_request(
        &self,
        request: &ApprovalRequest,
    ) -> Result<ApprovalReference, AdapterError> {
        let blocks = self.build_blocks(request);
        
        let response = self.client
            .post("https://slack.com/api/chat.postMessage")
            .bearer_auth(&self.bot_token)
            .json(&json!({
                "channel": self.channel,
                "blocks": blocks,
                "metadata": {
                    "event_type": "thoughtgate_approval",
                    "event_payload": { "task_id": request.task_id.to_string() }
                }
            }))
            .send()
            .await?;
        
        let body: SlackResponse = response.json().await?;
        
        if !body.ok {
            return Err(AdapterError::PostFailed {
                reason: body.error.unwrap_or_default(),
                retriable: false,
            });
        }
        
        Ok(ApprovalReference {
            task_id: request.task_id.clone(),
            external_id: body.ts.unwrap(),
            channel: body.channel.unwrap(),
            posted_at: Utc::now(),
            next_poll_at: Instant::now() + Duration::from_secs(5),
            poll_count: 0,
        })
    }
    
    async fn poll_for_decision(
        &self,
        reference: &ApprovalReference,
    ) -> Result<Option<ApprovalDecision>, AdapterError> {
        let response = self.client
            .get("https://slack.com/api/reactions.get")
            .bearer_auth(&self.bot_token)
            .query(&[
                ("channel", &reference.channel),
                ("timestamp", &reference.external_id),
            ])
            .send()
            .await?;
        
        let body: SlackReactionsResponse = response.json().await?;
        
        if !body.ok {
            return Err(AdapterError::PollFailed {
                reason: body.error.unwrap_or_default(),
                retriable: true,
            });
        }
        
        // Check for approval reaction
        if let Some(reaction) = body.find_reaction(&self.approve_reaction) {
            let user = reaction.users.first().unwrap();
            let display_name = self.get_user_name(user).await?;
            
            return Ok(Some(ApprovalDecision {
                decision: Decision::Approved,
                decided_by: display_name,
                decided_at: Utc::now(),
                method: DecisionMethod::Reaction { 
                    emoji: self.approve_reaction.clone() 
                },
            }));
        }
        
        // Check for rejection reaction
        if let Some(reaction) = body.find_reaction(&self.reject_reaction) {
            let user = reaction.users.first().unwrap();
            let display_name = self.get_user_name(user).await?;
            
            return Ok(Some(ApprovalDecision {
                decision: Decision::Rejected,
                decided_by: display_name,
                decided_at: Utc::now(),
                method: DecisionMethod::Reaction { 
                    emoji: self.reject_reaction.clone() 
                },
            }));
        }
        
        // No decision yet
        Ok(None)
    }
    
    async fn cancel_approval(
        &self,
        reference: &ApprovalReference,
    ) -> Result<(), AdapterError> {
        // Update message to show cancelled (best effort)
        let _ = self.client
            .post("https://slack.com/api/chat.update")
            .bearer_auth(&self.bot_token)
            .json(&json!({
                "channel": reference.channel,
                "ts": reference.external_id,
                "blocks": self.build_cancelled_blocks(),
            }))
            .send()
            .await;
        
        Ok(())
    }
    
    fn name(&self) -> &'static str {
        "slack"
    }
}
```

### Anti-Patterns to Avoid

- **❌ Callback endpoints:** Don't expose inbound endpoints; sidecars aren't addressable
- **❌ Polling too fast:** Respect rate limits; use exponential backoff
- **❌ Ignoring rate limit errors:** Always handle 429 responses
- **❌ Storing bot token in code:** Use K8s Secrets
- **❌ Blocking on polls:** Polling should be async and non-blocking

## 11. Definition of Done

- [ ] Slack adapter implemented with chat.postMessage
- [ ] Polling scheduler with priority queue
- [ ] Reaction detection (👍/👎)
- [ ] Rate limiting (token bucket)
- [ ] Exponential backoff on polls
- [ ] Message update after decision
- [ ] Cancellation support
- [ ] Graceful shutdown (drain polling queue)
- [ ] Metrics for all operations
- [ ] All edge cases (EC-APR-001 to EC-APR-016) covered
- [ ] Integration test with Slack API mock