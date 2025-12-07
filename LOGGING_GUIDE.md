# Logging Guide for Aggregation + TopK Debugging

This document explains all the logging that has been added to help you understand the interaction between GroupedHashAggregateStream and TopK operators.

## Log Prefixes

All logs use consistent prefixes to make filtering easy:

### Aggregation Logs (row_hash.rs)

- `[AGG-PARTIAL]` - Partial aggregation mode operations
- `[AGG-FINAL]` - Final aggregation mode operations  
- `[AGG-BATCH]` - Batch processing within group_aggregate_batch
- `[AGG-OUTPUT]` - Output batch production
- `[AGG-EARLY-EMIT]` - Early emission due to memory pressure
- `[MEMORY]` - Memory tracking before/after emit operations
- `[BLOCKED-GROUPS]` - Blocked groups optimization decisions

### TopK Logs (topk/mod.rs)

- `[TOPK-INSERT]` - Batch insertion into TopK heap
- `[TOPK-COMPACT]` - Heap compaction operations
- `[TOPK-EMIT]` - Final emission of top K results
- `[TOPK-EARLY]` - Early termination checks
- `[TOPK-MEMORY]` - Memory tracking in RecordBatchStore

## Key Log Messages

### 1. Aggregation Receiving Input

```
[AGG-PARTIAL] Received input batch: rows=8192, current_groups=1523
[AGG-FINAL] Received input batch: rows=1523, current_groups=0, mode=Final
```

**What it means**: Aggregation operator received a new batch to process. Shows current number of groups before processing.

### 2. Group Creation

```
[AGG-BATCH] Created 245 new groups (total now: 1768)
```

**What it means**: New unique group values were discovered in the input batch.

### 3. Memory Tracking

```
[MEMORY] BEFORE emit(All): total=512KB (accumulators=256KB, group_values=245KB, num_groups=1768)
[MEMORY] AFTER emit(All): total=12KB (accumulators=8KB, group_values=4KB, num_groups=0), emitted_rows=1768, memory_freed=500KB
```

**What it means**: Shows memory usage before and after emitting groups. Key for understanding memory pressure.

### 4. EmitTo Calls

```
[AGG-PARTIAL] Group ordering triggered emit: emit_to=First(8192)
[AGG-FINAL] Group ordering triggered emit: emit_to=All, groups=1768
```

**What it means**: The group ordering logic determined it's time to emit groups. Shows what type of emission (First(n), All, NextBlock).

### 5. Soft Limit Hit

```
[AGG-FINAL] Hit soft group limit: groups=10, limit=Some(10), triggering emission
```

**What it means**: The number of groups reached the soft limit (from LIMIT clause), triggering early emission.

### 6. Early Emission (Memory Pressure)

```
[AGG-EARLY-EMIT] Memory pressure detected: groups=16384, batch_size=8192, triggering early emission
[AGG-EARLY-EMIT] Emitting first 16384 groups (keeping 0 groups)
[AGG-EARLY-EMIT] Early emission produced batch with 16384 rows
```

**What it means**: Memory reservation failed, so partial aggregation is emitting groups early to free memory.

### 7. Output Production

```
[AGG-OUTPUT] ProducingOutput: batch_rows=1768, batch_size=8192, input_done=true
[AGG-OUTPUT] Emitting final chunk: rows=1768, next_state=Done
[AGG-OUTPUT] ✓ Returning batch to upstream: rows=1768
```

**What it means**: Aggregation is producing output batches. Shows how large batches are sliced into batch_size chunks.

### 8. TopK Receiving Batch

```
[TOPK-INSERT] ▼ Receiving batch from aggregation: rows=1768, current_heap=0/10, memory=2KB
```

**What it means**: TopK operator received a batch from the aggregation layer. Shows current heap state.

### 9. TopK Processing

```
[TOPK-INSERT] Processed rows: added=10, rejected=1758, heap_now=10/10
[TOPK-INSERT] ✓ Batch processed: heap=10/10, memory=8KB (Δ+6KB), store_batches=1, replacements_total=10
```

**What it means**: Shows how many rows were added to the heap vs rejected. Memory delta shows growth.

### 10. TopK Compaction

```
[TOPK-COMPACT] Checking compaction: store_batches=5, unused_rows=7890, threshold=163850
[TOPK-COMPACT] Starting compaction: old_batches=5, unused_rows=7890, memory=256KB
[TOPK-COMPACT] ✓ Compaction complete: new_batches=1, memory=8KB, saved=248KB
```

**What it means**: TopK is consolidating multiple stored batches into one to save memory.

### 11. TopK Early Termination

```
[TOPK-EARLY] Checking early completion: heap is full, has common prefix
[TOPK-EARLY] ✓ Early completion triggered: last batch row prefix > max heap row prefix
[TOPK-INSERT] ⚠ Early termination triggered - TopK is finished!
```

**What it means**: TopK detected that all future rows will be larger than current top K, so it can stop processing.

### 12. TopK Final Emission

```
[TOPK-EMIT] Starting final emission: heap_size=10, batch_size=8192
[TOPK-EMIT] Emitted single batch with 10 rows, splitting into batch_size=8192
[TOPK-EMIT] Final chunk: 10 rows
[TOPK-EMIT] ✓ Emission complete: 1 output batches
```

**What it means**: TopK is producing its final output - the top K rows in sorted order.

## Filtering Logs

### See Only Aggregation Flow

```bash
RUST_LOG=info cargo test --test aggregation_topk_debug -- --nocapture 2>&1 | grep '\[AGG-'
```

### See Only TopK Flow

```bash
RUST_LOG=info cargo test --test aggregation_topk_debug -- --nocapture 2>&1 | grep '\[TOPK-'
```

### See Only Memory Operations

```bash
RUST_LOG=info cargo test --test aggregation_topk_debug -- --nocapture 2>&1 | grep '\[MEMORY\]'
```

### See Only EmitTo Calls

```bash
RUST_LOG=info cargo test --test aggregation_topk_debug -- --nocapture 2>&1 | grep 'emit'
```

### See Data Flow (Batch Passing)

```bash
RUST_LOG=info cargo test --test aggregation_topk_debug -- --nocapture 2>&1 | grep -E '(Returning batch to upstream|Receiving batch from)'
```

## Understanding the Complete Flow

Here's what a typical execution looks like:

### Phase 1: Aggregation Builds Groups

```
[AGG-FINAL] Received input batch: rows=8192, current_groups=0
[AGG-BATCH] Created 1523 new groups (total now: 1523)
[AGG-FINAL] After aggregation: total_groups=1523, memory=245KB
```

### Phase 2: Aggregation Continues Processing

```
[AGG-FINAL] Received input batch: rows=8192, current_groups=1523
[AGG-BATCH] Created 245 new groups (total now: 1768)
[AGG-FINAL] After aggregation: total_groups=1768, memory=289KB
```

### Phase 3: Soft Limit Triggers Emission

```
[AGG-FINAL] Hit soft group limit: groups=10, limit=Some(10), triggering emission
[MEMORY] BEFORE emit(All): total=289KB (accumulators=144KB, group_values=145KB, num_groups=10)
[MEMORY] AFTER emit(All): total=4KB (accumulators=2KB, group_values=2KB, num_groups=0), emitted_rows=10, memory_freed=285KB
```

### Phase 4: Output Production

```
[AGG-OUTPUT] ProducingOutput: batch_rows=10, batch_size=8192, input_done=true
[AGG-OUTPUT] Emitting final chunk: rows=10, next_state=Done
[AGG-OUTPUT] ✓ Returning batch to upstream: rows=10
```

### Phase 5: TopK Receives and Processes

```
[TOPK-INSERT] ▼ Receiving batch from aggregation: rows=10, current_heap=0/10, memory=2KB
[TOPK-INSERT] Processed rows: added=10, rejected=0, heap_now=10/10
[TOPK-INSERT] ✓ Batch processed: heap=10/10, memory=8KB (Δ+6KB), store_batches=1, replacements_total=10
```

### Phase 6: TopK Emits Final Results

```
[TOPK-EMIT] Starting final emission: heap_size=10, batch_size=8192
[TOPK-EMIT] Emitted single batch with 10 rows, splitting into batch_size=8192
[TOPK-EMIT] ✓ Emission complete: 1 output batches
```

## Debugging Scenarios

### Scenario 1: High Memory Usage

**Look for:**
- `[MEMORY]` logs showing large memory values
- `[AGG-EARLY-EMIT]` logs indicating memory pressure
- Number of groups growing large

**Example:**
```
[AGG-FINAL] After aggregation: total_groups=100000, memory=15MB
[AGG-EARLY-EMIT] Memory pressure detected: groups=100000, batch_size=8192
```

### Scenario 2: Many TopK Replacements

**Look for:**
- `[TOPK-INSERT]` logs showing high `replacements_total`
- Many rows being rejected vs added

**Example:**
```
[TOPK-INSERT] Processed rows: added=10, rejected=8182, heap_now=10/10
[TOPK-INSERT] ✓ Batch processed: replacements_total=50000
```

**What it means**: Input is not well-ordered, causing many heap updates.

### Scenario 3: Early Termination Not Working

**Look for:**
- `[TOPK-EARLY]` logs showing why early termination isn't happening
- Missing common sort prefix

**Example:**
```
[TOPK-EARLY] No common sort prefix, cannot attempt early completion
```

**What it means**: Query doesn't have matching ORDER BY and input ordering.

### Scenario 4: Frequent Compaction

**Look for:**
- Multiple `[TOPK-COMPACT]` logs
- High `unused_rows` values

**Example:**
```
[TOPK-COMPACT] Starting compaction: old_batches=25, unused_rows=200000, memory=5MB
```

**What it means**: Many batches with few used rows, triggering compaction.

## Performance Analysis

### Measuring Aggregation Efficiency

Count how many times groups are emitted:
```bash
grep -c '\[MEMORY\] AFTER emit' test_output.log
```

### Measuring TopK Efficiency

Count row replacements:
```bash
grep '\[TOPK-INSERT\].*replacements_total' test_output.log | tail -1
```

### Measuring Memory Churn

Track memory deltas:
```bash
grep '\[MEMORY\] AFTER emit' test_output.log | grep -oP 'memory_freed=\K[0-9]+'
```

## Tips

1. **Start with INFO level** - It shows the major operations without overwhelming detail
2. **Use DEBUG for deep dives** - Shows per-row and detailed state information
3. **Filter by prefix** - Use grep to focus on specific components
4. **Watch for ✓ and ▼ symbols** - They mark important transitions
5. **Track memory deltas** - Look for Δ symbols showing memory changes
6. **Follow batch flow** - Track rows from input → aggregation → TopK → output

## Adding Your Own Logs

If you need more detail, see `CUSTOM_INSTRUMENTATION_EXAMPLE.md` for examples of adding additional logging points.
