# Adding Custom Instrumentation for Debugging

This guide shows you how to add custom logging and instrumentation to understand the aggregation and TopK interaction in more detail.

## Quick Instrumentation Points

### 1. Track Group Creation in GroupedHashAggregateStream

**File**: `datafusion/physical-plan/src/aggregates/row_hash.rs`

**Location**: In the `poll_next` method, after processing a batch

```rust
// After evaluate_group_by call
log::info!(
    "[AGG-GROUPS] Batch processed: input_rows={}, unique_groups={}, memory={}KB",
    batch.num_rows(),
    self.group_values.len(),
    self.reservation.size() / 1024
);
```

### 2. Track Group Emission

**File**: `datafusion/physical-plan/src/aggregates/row_hash.rs`

**Location**: In the emission logic (look for `ExecutionState::ProducingOutput`)

```rust
log::info!(
    "[AGG-EMIT] Emitting batch: rows={}, groups_remaining={}, memory_before={}KB",
    batch.num_rows(),
    self.group_values.len(),
    self.reservation.size() / 1024
);
```

### 3. Track TopK Heap Operations

**File**: `datafusion/physical-plan/src/topk/mod.rs`

**Location**: In `TopK::insert_batch` method

```rust
// At the start of insert_batch
log::info!(
    "[TOPK-INSERT] Receiving batch: rows={}, current_heap_size={}/{}, memory={}KB",
    batch.num_rows(),
    self.heap.inner.len(),
    self.heap.k,
    self.reservation.size() / 1024
);

// After processing
log::info!(
    "[TOPK-INSERT] Batch processed: heap_size={}/{}, replacements={}",
    self.heap.inner.len(),
    self.heap.k,
    self.metrics.row_replacements.value()
);
```

### 4. Track TopK Heap Compaction

**File**: `datafusion/physical-plan/src/topk/mod.rs`

**Location**: In `TopKHeap::maybe_compact` method

```rust
// Before compaction decision
let unused = self.store.unused_rows();
log::info!(
    "[TOPK-COMPACT] Checking compaction: store_batches={}, unused_rows={}, threshold={}",
    self.store.len(),
    unused,
    max_unused_rows
);

// If compaction happens
log::info!(
    "[TOPK-COMPACT] Compacting: old_batches={}, new_batches=1, memory_saved={}KB",
    old_batch_count,
    (old_size - new_size) / 1024
);
```

### 5. Track Memory Reservations

**File**: `datafusion/physical-plan/src/aggregates/row_hash.rs`

**Location**: After `reservation.try_resize()` calls

```rust
log::info!(
    "[AGG-MEMORY] Reservation updated: new_size={}KB, change={:+}KB",
    self.reservation.size() / 1024,
    (new_size as i64 - old_size as i64) / 1024
);
```

## Example: Complete Instrumentation for Group Lifecycle

Here's a complete example showing how to track a group from creation to emission:

```rust
// In GroupedHashAggregateStream

// 1. When a new group is created
impl GroupValues for YourGroupValues {
    fn intern(&mut self, cols: &[ArrayRef], group_indices: &mut Vec<usize>) -> Result<()> {
        let old_len = self.len();
        // ... existing logic ...
        let new_groups = self.len() - old_len;
        
        if new_groups > 0 {
            log::debug!(
                "[GROUP-CREATE] Created {} new groups, total={}",
                new_groups,
                self.len()
            );
        }
        Ok(())
    }
}

// 2. When accumulating values for groups
log::trace!(
    "[GROUP-ACCUM] Updating group {}: old_value={:?}, new_value={:?}",
    group_idx,
    old_value,
    new_value
);

// 3. When emitting groups
log::info!(
    "[GROUP-EMIT] Emitting groups: range={}-{}, total_groups={}",
    start_idx,
    end_idx,
    self.group_values.len()
);
```

## Detailed Memory Tracking Example

To understand memory flow in detail:

```rust
// Create a helper struct to track memory changes
struct MemoryTracker {
    component: String,
    last_size: usize,
}

impl MemoryTracker {
    fn new(component: &str) -> Self {
        Self {
            component: component.to_string(),
            last_size: 0,
        }
    }
    
    fn track(&mut self, current_size: usize, context: &str) {
        let delta = current_size as i64 - self.last_size as i64;
        log::info!(
            "[MEM-{}] {}: current={}KB, delta={:+}KB",
            self.component,
            context,
            current_size / 1024,
            delta / 1024
        );
        self.last_size = current_size;
    }
}

// Use it in GroupedHashAggregateStream
let mut mem_tracker = MemoryTracker::new("AGG");

// After each operation
mem_tracker.track(self.reservation.size(), "after_batch_process");
mem_tracker.track(self.reservation.size(), "after_emission");
```

## Visualizing the Flow

You can create a simple flow visualizer by logging structured data:

```rust
// Log in JSON format for easy parsing
log::info!(
    "{{\"event\":\"batch_received\",\"component\":\"aggregation\",\"rows\":{},\"groups\":{},\"memory\":{}}}",
    batch.num_rows(),
    self.group_values.len(),
    self.reservation.size()
);

log::info!(
    "{{\"event\":\"batch_emitted\",\"component\":\"aggregation\",\"rows\":{},\"memory\":{}}}",
    output_batch.num_rows(),
    self.reservation.size()
);

log::info!(
    "{{\"event\":\"batch_received\",\"component\":\"topk\",\"rows\":{},\"heap_size\":{},\"memory\":{}}}",
    batch.num_rows(),
    self.heap.inner.len(),
    self.reservation.size()
);
```

Then parse the logs:

```bash
cargo test --test aggregation_topk_debug -- --nocapture 2>&1 | grep -E '^\{' | jq .
```

## Performance Profiling Integration

For deeper performance analysis, you can use the `tracing` crate:

```rust
use tracing::{info_span, instrument};

#[instrument(skip(self, batch), fields(rows = batch.num_rows()))]
fn process_batch(&mut self, batch: RecordBatch) -> Result<()> {
    let _span = info_span!("process_batch", 
                           groups = self.group_values.len(),
                           memory_kb = self.reservation.size() / 1024);
    
    // ... existing logic ...
    
    Ok(())
}
```

Then run with tracing enabled:

```bash
RUST_LOG=datafusion_physical_plan=trace cargo test --test aggregation_topk_debug -- --nocapture
```

## Conditional Instrumentation

To avoid performance overhead in production, use conditional compilation:

```rust
#[cfg(debug_assertions)]
{
    log::debug!(
        "[DEBUG-ONLY] Detailed state: groups={}, memory={}, ...",
        self.group_values.len(),
        self.reservation.size()
    );
}
```

## Tips for Effective Instrumentation

1. **Use consistent prefixes**: `[AGG-*]`, `[TOPK-*]`, `[MEM-*]` for easy filtering
2. **Include context**: Always log relevant state (row counts, memory, etc.)
3. **Use appropriate log levels**:
   - `trace`: Very detailed, per-row operations
   - `debug`: Per-batch operations, state changes
   - `info`: Major events (emission, compaction)
   - `warn`: Unexpected but handled situations
4. **Measure before and after**: Log state before and after important operations
5. **Use structured logging**: JSON or key=value format for easy parsing

## Example Complete Instrumentation Session

```rust
// In row_hash.rs - GroupedHashAggregateStream::poll_next

log::info!("[AGG-START] Processing new batch");

// Before processing
log::debug!(
    "[AGG-STATE] Before: groups={}, memory={}KB, input_done={}",
    self.group_values.len(),
    self.reservation.size() / 1024,
    self.input_done
);

// Process batch
let group_indices = evaluate_group_by(&self.group_by, &batch)?;
log::debug!("[AGG-EVAL] Evaluated {} group indices", group_indices.len());

self.group_values.intern(&group_cols, &mut self.current_group_indices)?;
log::debug!(
    "[AGG-INTERN] After intern: total_groups={}",
    self.group_values.len()
);

// Update accumulators
for (idx, accumulator) in self.accumulators.iter_mut().enumerate() {
    accumulator.update_batch(&group_indices, &values)?;
    log::trace!("[AGG-ACCUM] Updated accumulator {}", idx);
}

// After processing
log::debug!(
    "[AGG-STATE] After: groups={}, memory={}KB",
    self.group_values.len(),
    self.reservation.size() / 1024
);

log::info!("[AGG-END] Batch processing complete");
```

This will give you a complete picture of what's happening at each stage!
