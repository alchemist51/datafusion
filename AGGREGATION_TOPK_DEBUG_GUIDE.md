# Debugging GroupedHashAggregateStream and TopK Interaction

This guide helps you understand how aggregation groups are stored, emitted, and passed to the TopK operator.

## Test File

The test is located at: `datafusion/core/tests/aggregation_topk_debug.rs`

## Query Being Tested

```sql
SELECT "WatchID", 
       MIN("ResolutionWidth") as min_width, 
       MAX("ResolutionWidth") as max_width, 
       SUM("IsRefresh") as sum_refresh
FROM hits 
GROUP BY "WatchID" 
ORDER BY "WatchID" DESC 
LIMIT 10
```

## Running the Test

### Basic Run (with all debug output)
```bash
RUST_LOG=debug cargo test --test aggregation_topk_debug -- --nocapture
```

### Focused on Physical Plan Components
```bash
RUST_LOG=datafusion_physical_plan=debug cargo test --test aggregation_topk_debug -- --nocapture
```

### With Trace Level (very verbose)
```bash
RUST_LOG=trace cargo test --test aggregation_topk_debug -- --nocapture
```

### Running in Debug Mode with Debugger

To step through with a debugger (lldb on macOS):

```bash
# Build the test in debug mode
cargo test --test aggregation_topk_debug --no-run

# Find the test binary
TEST_BINARY=$(find target/debug/deps -name 'aggregation_topk_debug-*' -type f | head -1)

# Run with lldb
rust-lldb $TEST_BINARY -- test_aggregation_topk_interaction --nocapture
```

Then in lldb:
```
# Set breakpoints at key locations
b datafusion_physical_plan::aggregates::row_hash::GroupedHashAggregateStream::new
b datafusion_physical_plan::topk::TopK::insert_batch
b datafusion_physical_plan::topk::TopK::emit

# Run
r

# Step through, inspect variables, etc.
```

## Key Things to Watch For

### 1. Group Storage in GroupedHashAggregateStream

Look for log messages about:
- `group_values` being created and updated
- Number of unique groups being tracked
- Memory allocations for group storage

### 2. Group Emission

Watch for when groups are emitted from the hash aggregation:
- Batch sizes being emitted
- Timing of emissions (early vs. at end of input)
- Memory being freed after emission

### 3. Memory Tracking

Monitor:
- Memory reservation changes
- Peak memory usage
- Spilling behavior (if any)

### 4. TopK Interaction

Observe:
- When batches flow from aggregation to TopK
- How TopK maintains its heap
- Row replacements in the TopK heap
- Early termination optimization (if applicable)

### 5. Batch Flow

Track:
- Number of rows in each batch at each stage
- How batches are split/combined
- Final result assembly

## Understanding the Code Flow

### Aggregation Phase (row_hash.rs)

1. **Input Processing**: `GroupedHashAggregateStream` receives input batches
2. **Group Assignment**: Each row is assigned to a group via `group_values`
3. **Accumulation**: Accumulators update state for each group
4. **Emission**: Groups are emitted as `RecordBatch`es

### TopK Phase (topk/mod.rs)

1. **Batch Reception**: TopK receives batches from aggregation
2. **Heap Management**: Maintains a heap of top K rows
3. **Row Comparison**: Compares incoming rows against heap
4. **Emission**: Final top K results are emitted

## Adding Custom Instrumentation

If you want to add more detailed logging, you can modify:

### In row_hash.rs

Add logging in `GroupedHashAggregateStream::poll_next`:
```rust
log::info!("[AGG] Processing batch with {} rows, current groups: {}", 
           batch.num_rows(), 
           self.group_values.len());
```

### In topk/mod.rs

Add logging in `TopK::insert_batch`:
```rust
log::info!("[TOPK] Received batch with {} rows, heap size: {}/{}", 
           batch.num_rows(), 
           self.heap.inner.len(), 
           self.heap.k);
```

## Interpreting Results

### Expected Behavior

1. Aggregation creates groups for each unique `WatchID`
2. Groups are emitted (possibly in multiple batches)
3. TopK receives these batches and maintains top 10 by `WatchID DESC`
4. Final result contains exactly 10 rows

### Memory Patterns

- Aggregation memory grows as groups accumulate
- Memory may spike during emission
- TopK memory is bounded by K (10 in this case)

### Performance Considerations

- Number of unique groups affects aggregation memory
- TopK is efficient for small K values
- Early termination can save significant work

## Troubleshooting

### Test Fails to Find Data

If you see an error about the data path, update the path in the test:
```rust
let data_path = "/your/actual/path/to/clickbench_data/partitioned";
```

### Too Much Output

Reduce logging level:
```bash
RUST_LOG=info cargo test --test aggregation_topk_debug -- --nocapture
```

### Want to See Specific Components

Use module-specific logging:
```bash
RUST_LOG=datafusion_physical_plan::aggregates=debug,datafusion_physical_plan::topk=debug cargo test --test aggregation_topk_debug -- --nocapture
```

## Next Steps

After understanding the basic flow, you might want to:

1. **Modify the query** to test different scenarios (more/fewer groups, different K values)
2. **Add breakpoints** in specific functions to inspect state
3. **Profile memory usage** with tools like `heaptrack` or `valgrind`
4. **Benchmark** different configurations

## Related Files

- `datafusion/physical-plan/src/aggregates/row_hash.rs` - Hash aggregation implementation
- `datafusion/physical-plan/src/topk/mod.rs` - TopK implementation
- `datafusion/physical-plan/src/aggregates/group_values/` - Group storage implementations
- `datafusion/physical-expr/src/aggregate/` - Accumulator implementations
