# Multi-Instance Redis Architecture Analysis

**Status**: Analysis Complete - Ready for Implementation Decision

**Date**: 2025-12-16

---

## Summary

Redis provides **two high-value use cases** for Dialect Coach:
1. **WebSocket Push Notifications** - Real-time usage stats updates
2. **Shared TTS Cache** - Reduce API costs, improve performance

Both justify the Redis dependency together.

---

## Use Case 1: WebSocket Push Notifications

### Current Architecture

**Component**: `user_state_connections` - HashMap mapping `user_id` → WebSocket sender channel

**Location**: `backend/src/state.rs:24-25`

**Usage**:
1. `websocket/user_state.rs:26` - `send_usage_stats_update()` - Push usage stats to frontend
2. `tts_handler.rs:155` - `notify_usage_stats_update()` - Push TTS usage stats to frontend

### How It Works (Single Instance)

```
User connects WebSocket → Registered in HashMap
User makes chat/TTS request → Backend saves to Postgres → Looks up WebSocket → Pushes update
Frontend receives real-time usage stats update
```

### Multi-Instance Failure Scenario

```
User connects WebSocket to Instance A → Registered in A's HashMap
User makes chat/TTS request → Load balancer routes to Instance B
Instance B saves to Postgres ✓
Instance B looks up user in B's HashMap → Not found
Instance B logs warning, continues
Frontend does NOT receive real-time update
```

### Impact

**Data Correctness**: ✅ No issue (saved to Postgres)

**User Experience**: ⚠️ Degraded
- Usage stats stale until:
  - Page refresh
  - Next WebSocket message (chat response triggers re-sync)
  - Manual action that loads fresh state

**Frequency** (without sticky sessions):
- With 2 instances: ~50% of requests hit wrong instance
- With 3 instances: ~67% of requests hit wrong instance

### Redis Pub/Sub Solution

**How It Works**:
```
Instance A: User WebSocket connected
Instance B: Receives API request, saves to Postgres
Instance B: Publishes message to Redis channel "user_state:{user_id}"
Instance A: Subscribed to Redis, receives message
Instance A: Forwards message to user's WebSocket
```

**Implementation**:
- Add `WebSocketPubSub` service (~150 lines)
- Subscribe to Redis channels for connected users
- Publish updates instead of direct channel sends
- Graceful degradation: If Redis down, logs warning, data still saved

**Code Changes**: ~500 lines total
- New file: `backend/src/websocket_pubsub.rs` (~150 lines)
- Update: `websocket/user_state.rs` (~10 lines changed)
- Update: `tts_handler.rs` (~10 lines changed)
- Update: `main.rs` - Initialize Redis (~20 lines)
- Update: `state.rs` - Add field (~5 lines)

---

## Use Case 2: Shared TTS Cache

### Current Architecture

**Component**: `TtsService` with local LRU cache

**Location**: `backend/src/tts_service.rs:11`

```rust
cache: Arc<tokio::sync::Mutex<lru::LruCache<String, TtsResponse>>>,
```

**Cache Details**:
- Capacity: 100 entries per instance
- Size: ~10-50MB depending on audio length
- Eviction: LRU (Least Recently Used)
- TTL: None (evicted when cache full)

### How It Works (Single Instance)

```
User requests TTS for "Hola, ¿cómo estás?"
  → Check cache (key = hash of text + voice + settings)
  → Miss → Call ElevenLabs/Azure API (~500ms, $0.30 per 1K chars)
  → Store in cache
  → Return audio

Same user requests same text again
  → Check cache
  → Hit → Return cached audio (~10ms, $0)
```

### Multi-Instance Problem

```
User on Instance A requests "Hola, ¿cómo estás?"
  → Instance A: Cache miss → API call → Cached on A

User on Instance B requests "Hola, ¿cómo estás?" (same text!)
  → Instance B: Cache miss → API call AGAIN → Cached on B

Result: Paid for same TTS synthesis twice
```

### Impact

**Cost Increase**:
- With 2 instances: ~50% cache hit rate reduction
- Example: 1000 unique requests, 500 repeats
  - Single instance: 1000 API calls (500 cache hits)
  - Two instances (no shared cache): ~1375 API calls (125 cache hits per instance)
  - **Extra cost**: ~37% more API calls

**Performance Degradation**:
- Cache hit: ~10ms
- API call: ~500ms
- **50x slower** for cache misses that should have been hits

**Wasted Resources**:
- Each instance downloads same audio files
- Memory: 100 entries × 2 instances = 200MB (vs 100MB shared)

### Redis Cache Solution

**How It Works**:
```
User on Instance A requests "Hola, ¿cómo estás?"
  → Check Redis (key = "tts:{hash}")
  → Miss → Call API → Store in Redis (TTL = 24 hours)
  → Return audio

User on Instance B requests "Hola, ¿cómo estás?"
  → Check Redis
  → Hit → Return cached audio
  → No API call needed
```

**Implementation**:
- Replace local LRU cache with Redis client
- Key format: `tts:{hash}` where hash = SHA256(text + voice + provider + settings)
- Value: Base64-encoded audio bytes
- TTL: 24 hours (configurable via constant)

**Code Changes**: ~100 lines total
- Update: `backend/src/tts_service.rs` (~50 lines changed)
  - Replace `lru::LruCache` with Redis operations
  - Async Redis get/set operations
- Update: `backend/Cargo.toml` - Remove `lru` dependency (already have Redis)
- Update: `main.rs` - Pass Redis client to TtsService (~5 lines)

**Cache Size Considerations**:
- Current: 100 entries × 50KB average = 5MB per instance
- Shared: 100 entries × 50KB = 5MB total (vs 10MB with 2 instances)
- Redis memory: 10MB for TTS cache is negligible

### Cost/Benefit Analysis

**Assumptions**:
- 1000 TTS requests per day
- 30% are repeated phrases (greetings, common expressions)
- ElevenLabs cost: $0.30 per 1K characters
- Average phrase: 50 characters

**Single Instance** (current):
- 1000 requests
- 300 cache hits (30%)
- 700 API calls
- Cost: 700 × 50 chars × $0.30/1000 = **$10.50/day**

**Two Instances (no shared cache)**:
- 1000 requests distributed across 2 instances
- Each instance: 500 requests, ~75 cache hits (15% hit rate)
- Total API calls: 850
- Cost: 850 × 50 chars × $0.30/1000 = **$12.75/day**
- **Extra cost**: $2.25/day = **$67.50/month**

**Two Instances (Redis shared cache)**:
- 1000 requests
- ~280 cache hits (28% hit rate, slightly lower due to network latency)
- 720 API calls
- Cost: 720 × 50 chars × $0.30/1000 = **$10.80/day**
- **Savings vs non-shared**: $58.50/month

**Redis Cost**: ~$3-7/month

**Net Savings**: $51.50 - $62.50/month (pays for Redis 7-20x over)

---

## Combined Redis Architecture

### Infrastructure

**Redis Service**: Railway/Render Managed Redis

**Recommendation**: Use platform's built-in managed Redis

**Why**:
- Same network (lower latency, no egress costs)
- Automatic backups and failover
- Simpler configuration (one less external service)
- Pricing already bundled

**When to use external** (Upstash, Redis Cloud):
- If you outgrow platform limits (>100MB)
- If you want Redis to survive platform migration
- Only needed at much larger scale

**Connection**:
- Railway: Redis add-on (~$3-5/month for 25MB)
- Render: Managed Redis (~$7/month for 25MB)
- Environment variable: `REDIS_URL=redis://default:password@host:port`

**Memory Requirements**:
- TTS cache: ~10MB
- Pub/Sub: Negligible (messages not stored)
- Total: ~15-20MB with headroom

**Connection Pooling**:
- Redis client library handles connection pool
- Default: 10 connections per instance
- Sufficient for this use case

### Redis Configuration

**Cache Eviction Policy**: `maxmemory-policy allkeys-lru`

**Why**:
- Automatically evicts least recently used keys when memory full
- No manual intervention needed
- Keeps hot cache entries, drops cold ones
- Standard for cache use cases

**Configuration**:
- Most managed Redis services default to this
- Verify in Railway/Render dashboard or set via config

**TTL Strategy**:
- TTS cache: 24 hours
- Matches usage stats rolling window
- Long enough for daily repeated phrases
- Short enough to auto-expire unused entries
- Configurable via constant:
  ```rust
  const TTS_CACHE_TTL_SECONDS: u64 = 86400; // 24 hours
  ```

### Code Structure

**New Files**:
1. `backend/src/redis_client.rs` (~100 lines)
   - Initialize Redis connection from `REDIS_URL`
   - Connection pooling setup
   - Health check function

2. `backend/src/websocket_pubsub.rs` (~150 lines)
   - `publish_to_user(user_id, message)` function
   - `subscribe_to_user(user_id)` function
   - Background task to listen to subscriptions

**Modified Files**:
1. `backend/Cargo.toml`
   - Add: `redis = { version = "0.27", features = ["tokio-comp", "connection-manager"] }`
   - Remove: `lru = "0.12"` (no longer needed)

2. `backend/src/state.rs`
   - Add: `pub redis_client: redis::Client`
   - Add: `pub websocket_pubsub: Arc<WebSocketPubSub>`

3. `backend/src/main.rs` (~40 lines changed)
   - Initialize Redis client from `REDIS_URL` env var
   - Initialize `WebSocketPubSub` service
   - Pass Redis client to `TtsService`

4. `backend/src/tts_service.rs` (~50 lines changed)
   - Replace `lru::LruCache` with Redis operations
   - `get_cached(key)` → `redis.get(key)`
   - `set_cached(key, value)` → `redis.setex(key, TTL, value)`

5. `backend/src/websocket/user_state.rs` (~10 lines changed)
   - Replace direct channel send with `websocket_pubsub.publish_to_user()`
   - Subscribe to Redis on WebSocket connect

6. `backend/src/tts_handler.rs` (~10 lines changed)
   - Replace direct channel send with `websocket_pubsub.publish_to_user()`

**Total Code Changes**: ~600 lines (400 new, 200 modified)

### Error Handling & Graceful Degradation

**Redis Connection Failure**:
```rust
match redis_client.get(&key).await {
    Ok(Some(value)) => return Ok(value),  // Cache hit
    Ok(None) => { /* Cache miss, proceed to API */ },
    Err(e) => {
        tracing::warn!("Redis get failed: {}, bypassing cache", e);
        // Proceed to API call without caching
    }
}
```

**Behavior When Redis Down**:
- TTS cache: Falls back to direct API calls (slower, more expensive, but functional)
- WebSocket push: Falls back to no push (same as current multi-instance behavior)
- Data correctness: Unaffected (Postgres still works)
- Application: Continues running

**No Single Point of Failure**: Redis is performance optimization, not critical path

### Monitoring & Metrics

**Per-User Cache Hit Tracking** (Existing Usage Stats):

Update `TtsUsage` struct in `shared/src/models/usage_stats.rs`:
```rust
pub struct TtsUsage {
    pub timestamp: i64,
    pub characters: u64,
    pub cache_hit: bool,  // ← New field
}
```

**Track per-user**:
- User makes TTS request → log whether it was cache hit or miss
- Stored in Postgres with other usage stats
- Frontend can see their own hit rate in usage dashboard
- Aggregates over rolling window (same as other usage stats)

**Global Stats at `/admin/status`**:

Add to admin status endpoint:
```json
{
  "redis": {
    "connected": true,
    "memory_used_mb": 8.5,
    "memory_max_mb": 25,
    "uptime_seconds": 86400,
    "total_commands_processed": 45234
  },
  "tts_cache": {
    "total_requests_24h": 1000,
    "cache_hits_24h": 300,
    "cache_misses_24h": 700,
    "hit_rate": 0.30
  },
  "websocket_pubsub": {
    "messages_published_24h": 450,
    "active_subscriptions": 12
  }
}
```

**Implementation**:
- Backend tracks global counters in memory (reset every 24h or on restart)
- `/admin/status` endpoint queries Redis `INFO` command for health
- Returns JSON with Redis health + cache stats
- Add to existing admin status module

**Metrics to Track**:
- Redis connection status
- Redis memory usage
- TTS cache hit rate (global)
- WebSocket pub/sub message count
- Redis command latency (p50, p95, p99)

### Testing Strategy

**Local Testing** (docker-compose):
```yaml
services:
  redis:
    image: redis:7-alpine
    ports:
      - "6379:6379"
    healthcheck:
      test: ["CMD", "redis-cli", "ping"]
      interval: 5s
      timeout: 3s
      retries: 5

  backend-1:
    build: ./backend
    environment:
      REDIS_URL: redis://redis:6379
      DATABASE_URL: postgres://...
    depends_on:
      redis:
        condition: service_healthy

  backend-2:
    build: ./backend
    environment:
      REDIS_URL: redis://redis:6379
      DATABASE_URL: postgres://...
    depends_on:
      redis:
        condition: service_healthy
```

**Test Scenarios**:
1. **TTS Cache Sharing**:
   - Request TTS on Instance 1
   - Request same TTS on Instance 2
   - Verify cache hit (check logs, no API call)
   - Check `/admin/status` shows hit rate increase

2. **WebSocket Pub/Sub**:
   - Connect WebSocket to Instance 1
   - Make API request to Instance 2
   - Verify WebSocket receives update in real-time

3. **Redis Failure**:
   - Stop Redis container
   - Make TTS request (should fall back to API)
   - Make chat request (should save to Postgres, no crash)
   - Restart Redis (should resume caching)

4. **Cache Hit Tracking**:
   - Make multiple TTS requests (some duplicates)
   - Check usage stats show `cache_hit: true/false`
   - Check `/admin/status` shows correct global hit rate

---

## Implementation Phases

### Phase 1: Redis Infrastructure Setup
**Time**: 30 minutes

**Tasks**:
1. Add Redis to docker-compose.yml for local testing
2. Add Redis service to Railway/Render
3. Set `REDIS_URL` environment variable
4. Add `redis` crate to `Cargo.toml`
5. Create `redis_client.rs` module with health check

**Deliverables**:
- Redis running locally and in production
- Backend can connect to Redis
- Health check passes

### Phase 2: Shared TTS Cache
**Time**: 2 hours

**Tasks**:
1. Update `TtsUsage` struct with `cache_hit` field
2. Update `TtsService` to use Redis instead of LRU
3. Track cache hits/misses in usage stats
4. Add global cache stats to `/admin/status`
5. Test cache hits across instances
6. Verify fallback when Redis down

**Deliverables**:
- TTS cache shared across instances
- Tests pass (no regressions)
- Cache hit tracking in usage stats
- Global metrics visible in admin dashboard
- Monitoring shows cache hit rate improvement

### Phase 3: WebSocket Pub/Sub
**Time**: 2-3 hours

**Tasks**:
1. Create `websocket_pubsub.rs` module
2. Update `websocket/user_state.rs` to publish/subscribe
3. Update `tts_handler.rs` to publish
4. Add pub/sub metrics to `/admin/status`
5. Test updates across instances
6. Verify graceful degradation if Redis down

**Deliverables**:
- WebSocket push notifications work across instances
- Tests pass
- Real-time updates verified in browser
- Metrics show pub/sub activity

### Phase 4: Production Deployment & Monitoring
**Time**: 1 hour

**Tasks**:
1. Deploy to Railway/Render with Redis
2. Monitor Redis memory usage (should stay under 25MB)
3. Monitor cache hit rates (target >25%)
4. Monitor WebSocket delivery success rate
5. Verify no errors in logs
6. Load test with 2+ instances

**Deliverables**:
- Production deployment successful
- Redis stable, low memory usage
- Cache hit rate meeting targets
- Real-time updates working
- No degradation in performance

**Total Implementation Time**: 5-6 hours

---

## Cost Analysis

### Infrastructure Costs

**Redis**:
- Railway: $3-5/month (included in usage-based pricing, ~25MB)
- Render: $7/month (managed Redis, 25MB)

**Backend Instances** (2 instances):
- Railway: ~$12-15/month (no change from single instance plan)
- Render: ~$25-35/month (no change)

**Total Added Cost**: $3-7/month for Redis

### TTS API Cost Savings

**Assumptions**:
- 1000 TTS requests/day
- 30% repeated phrases
- $0.30 per 1K characters (ElevenLabs)
- 50 characters average

**Without Shared Cache** (2 instances, local caches):
- ~850 API calls/day
- $12.75/day = **$382.50/month**

**With Shared Redis Cache** (2 instances):
- ~720 API calls/day
- $10.80/day = **$324/month**

**Monthly Savings**: $58.50

**Net Benefit**: $58.50 savings - $7 Redis cost = **$51.50/month savings**

**ROI**: Redis pays for itself 7-8x over

### Performance Benefits (Not Monetized)

- Cache hit latency: 10ms vs 500ms API call (50x faster)
- Better user experience (real-time updates)
- Lower API provider rate limit risk (fewer calls)
- Reduced instance memory usage (shared cache vs per-instance)

---

## Alternative Solutions Comparison

### Option 1: Redis for Both (Recommended)

**Pros**:
- ✅ Solves both problems completely
- ✅ Saves $51/month on TTS costs
- ✅ Real-time WebSocket updates work
- ✅ Scales to any number of instances
- ✅ Standard, well-understood architecture
- ✅ Two use cases justify the dependency

**Cons**:
- ❌ New dependency (Redis)
- ❌ ~600 lines of code
- ❌ $3-7/month infrastructure cost

**Verdict**: **Best overall solution**. TTS savings alone justify Redis, WebSocket is bonus.

---

### Option 2: Sticky Sessions Only (Pragmatic)

**Pros**:
- ✅ Zero code changes
- ✅ Zero infrastructure cost
- ✅ Works ~90% of the time

**Cons**:
- ❌ No TTS cache sharing (wastes $51/month)
- ❌ WebSocket updates fail on instance crash/rebalance
- ❌ Uneven load distribution
- ❌ Hidden dependency on infrastructure behavior

**Verdict**: Cheaper upfront, expensive long-term. Only viable at very small scale.

---

### Option 3: Redis for WebSocket Only, Keep Local TTS Cache

**Pros**:
- ✅ Real-time WebSocket updates work
- ✅ Slightly less code (~500 lines vs 600)

**Cons**:
- ❌ Still wasting $51/month on duplicate TTS calls
- ❌ Leaves money on the table
- ❌ Already paying for Redis, not using it fully

**Verdict**: Doesn't make sense. If adding Redis anyway, use it for TTS cache too.

---

### Option 4: Frontend Polling Instead of Redis Pub/Sub

**Pros**:
- ✅ Simpler than Redis Pub/Sub (~50 lines frontend code)
- ✅ No Redis needed for WebSocket (but still want it for TTS cache)

**Cons**:
- ❌ 10-30 second latency vs real-time
- ❌ Constant HTTP requests (wasteful)
- ❌ Removes real-time feature

**Verdict**: If adding Redis for TTS cache anyway, use it for WebSocket too.

---

## Recommendation

**Use Redis for both TTS caching and WebSocket pub/sub.**

### Reasoning

1. **TTS cache alone justifies Redis**: Saves $51/month, pays for itself 7-8x
2. **WebSocket pub/sub is incremental**: Only +150 lines of code since Redis already there
3. **Two use cases strengthen the architecture**: Not over-engineering for one feature
4. **Standard pattern**: Redis for caching + pub/sub is industry-standard
5. **Low risk**: Graceful degradation if Redis fails
6. **Future-proof**: Room for more Redis use cases (JWT blacklist, session storage)

### Implementation Order

**Phase 1**: Infrastructure setup (Redis running)
**Phase 2**: TTS cache (immediate cost savings)
**Phase 3**: WebSocket pub/sub (UX improvement)
**Phase 4**: Deployment and monitoring

This way you get infrastructure first, then tangible savings, then UX improvement.

---

## Additional Redis Use Cases (Future Considerations)

### 3. JWT Token Blacklist (Security)
- **Use Case**: Revoke tokens on logout or compromise
- **Complexity**: Low (~50 lines)
- **Priority**: Medium
- **When**: If you add "logout all devices" feature
- **Implementation**: Store revoked tokens in Redis with TTL matching JWT expiry

### 4. API Response Cache (Performance)
- **Use Case**: Cache Anthropic/OpenAI responses for identical prompts
- **Complexity**: Medium
- **Priority**: Low (prompts rarely identical in chat)
- **When**: If you see repeated API calls in logs
- **Caveat**: Cache invalidation is tricky for chat

### 5. Rate Limit Buckets (Burst Protection)
- **Use Case**: Block rapid-fire requests within seconds (DDoS protection)
- **Complexity**: Medium
- **Priority**: Low (current Postgres rate limiting works)
- **When**: If you see abuse patterns
- **Implementation**: Redis sliding window counters

**Note**: Don't implement these now. TTS cache + WebSocket pub/sub are sufficient.

---

## Decision Summary

### Questions Answered

1. **Redis Provider**: Railway/Render managed Redis (same network, simpler)
2. **Cache TTL**: 24 hours (matches usage stats window, configurable)
3. **Cache Eviction**: `allkeys-lru` (automatic, standard for cache)
4. **Metrics**:
   - Per-user cache hit tracking in existing usage stats (`cache_hit` field)
   - Global stats at `/admin/status` endpoint (Redis health, cache rates, pub/sub metrics)

### Next Steps

1. **User Decision**: Approve Redis for TTS cache + WebSocket pub/sub?
2. **Choose Platform**: Railway or Render? (affects Redis setup slightly)
3. **Implementation**: Execute Phase 1-4 over ~6 hours
4. **Testing**: Verify cache sharing and pub/sub work locally
5. **Deploy**: Production deployment with monitoring

**Total Timeline**: 1-2 days (including testing and deployment)

**Total Cost**: ~$3-7/month (pays for itself 7-20x via TTS savings)

**Total Code**: ~600 lines (400 new, 200 modified)
